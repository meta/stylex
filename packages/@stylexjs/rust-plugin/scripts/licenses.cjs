/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// Generates RUST_DEPENDENCY_LICENSES.md and LICENSE_PROVENANCE.json from the
// Cargo packages of the Node addon's dependency graph; `--check` fails if the
// committed files differ. Run `cargo fetch --locked` first.

'use strict';

const { execFileSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
const repoRoot = path.resolve(root, '../../..');
const MD = path.join(root, 'RUST_DEPENDENCY_LICENSES.md');
const JSON_FILE = path.join(root, 'LICENSE_PROVENANCE.json');
const SUPPLEMENT = fs.existsSync(
  path.join(__dirname, 'licenses-supplement.json'),
)
  ? require('./licenses-supplement.json')
  : {};
const SCOPE =
  'cargo tree -p stylex_rust_compiler --features node -e normal --target all';
const TREE_ARGS = [
  'tree',
  '--locked',
  '-p',
  'stylex_rust_compiler',
  '--features',
  'node',
  '--edges',
  'normal',
  '--target',
  'all',
  '--prefix',
  'none',
  '--format',
  '{p}',
  '--color',
  'never',
];

const LICENSE_FILE =
  /^(licen[cs]e|copying|notice|unlicense|copyright)([-.].*)?$/i;
const COPYRIGHT = /^\s*(copyright\b(?!\s+and\s+permission)|\(c\)\s*\d|©)/i;
const TEMPLATE =
  /^\s*copyright\s+(\[yyyy\]\s+\[name of copyright owner\]|\{yyyy\}\s+\{name of copyright owner\})\s*$/i;
const TITLE = /^(the\s+)?mit\s+license(\s+\(mit\))?\.?$/i;
const APACHE_END = 'END OF TERMS AND CONDITIONS';
// The Apache appendix is instructions plus a notice template; filled-in
// copyright lines are kept per crate.
const APPENDIX =
  /^(APPENDIX: How to apply|To apply the Apache License|Licensed under the Apache License, Version 2\.0 \(the "License"\)|you may not use this file|You may obtain a copy|https?:\/\/www\.apache\.org\/licenses\/LICENSE-2\.0$|Unless required by applicable law|distributed under the License|WITHOUT WARRANTIES OR CONDITIONS|See the License for|limitations under the License)/;

const sha256 = (text) => crypto.createHash('sha256').update(text).digest('hex');
const paragraphs = (text) =>
  text
    .replace(/\r\n?/g, '\n')
    .split(/\n[ \t]*\n/)
    .map((p) => p.replace(/^\n+|\s+$/g, ''))
    .filter((p) => p.trim() !== '');
// Whitespace, URL scheme and template bracket style do not change a text.
const normalize = (text) =>
  text
    .replace(/https:\/\//g, 'http://')
    .replace(
      /\{yyyy\} \{name of copyright owner\}/g,
      '[yyyy] [name of copyright owner]',
    )
    .replace(/"\{\}"/g, '"[]"')
    .replace(/(apache\.org\/licenses\/)LICENSE-2\.0(\s+TERMS)/g, '$1$2')
    .replace(/\s+/g, ' ')
    .trim();

// Splits one license file into copyright paragraphs and the text units the
// rest consists of (an Apache-2.0 text, its addenda, or the whole remainder).
function decompose(text) {
  const copyright = [];
  const units = [];
  let body = [];
  const isApache =
    /^\s*Apache License\s+Version 2\.0/.test(text) && text.includes(APACHE_END);
  let inAppendix = false;
  const appendix = [];
  const tail = [];
  for (const raw of paragraphs(text)) {
    const p = raw.trim();
    if (isApache && inAppendix) {
      if (TEMPLATE.test(p) || APPENDIX.test(p.replace(/\s+/g, ' ')))
        appendix.push(raw);
      else if (COPYRIGHT.test(p)) copyright.push(raw);
      else tail.push(raw);
      continue;
    }
    if (TEMPLATE.test(p) && isApache) continue;
    const lines = raw.split('\n');
    if (
      body.length === 0 &&
      TITLE.test(lines[0].trim()) &&
      lines.length > 1 &&
      lines.slice(1).every((l) => COPYRIGHT.test(l))
    ) {
      copyright.push(lines.slice(1).join('\n'));
      continue;
    }
    if (body.length === 0 && TITLE.test(p)) continue;
    if (COPYRIGHT.test(p)) {
      copyright.push(raw);
      continue;
    }
    body.push(raw);
    if (isApache && p.endsWith(APACHE_END)) {
      units.push(body.join('\n\n'));
      body = [];
      inAppendix = true;
    }
  }
  if (body.length) units.push(body.join('\n\n'));
  if (tail.length) units.push(tail.join('\n\n'));
  // An Apache-2.0 text is grouped by its terms and shown with its appendix.
  const display = units.map((u, i) =>
    i === 0 && appendix.length ? [u, ...appendix].join('\n\n') : u,
  );
  return { copyright, units, display };
}

function packages() {
  const cwd = root;
  // Plain output whatever the environment's CARGO_TERM_COLOR says.
  const env = { ...process.env, CARGO_TERM_COLOR: 'never' };
  const tree = execFileSync('cargo', TREE_ARGS, { cwd, encoding: 'utf8', env })
    .split('\n')
    .map((l) => l.replace(/ \(.*$/, '').trim())
    .filter(Boolean);
  const wanted = new Set(tree);
  const meta = JSON.parse(
    execFileSync(
      'cargo',
      [
        'metadata',
        '--locked',
        '--features',
        'node',
        '--format-version',
        '1',
        '--color',
        'never',
      ],
      {
        cwd,
        encoding: 'utf8',
        maxBuffer: 1 << 28,
        env,
      },
    ),
  );
  const found = meta.packages.filter((p) =>
    wanted.has(`${p.name} v${p.version}`),
  );
  if (found.length !== wanted.size) {
    const known = new Set(found.map((p) => `${p.name} v${p.version}`));
    const missing = [...wanted].filter((id) => !known.has(id));
    throw new Error(
      `cargo metadata is missing packages of the tree: ${missing.map((id) => JSON.stringify(id)).join(', ')}`,
    );
  }
  return found.sort(
    (a, b) =>
      a.name.localeCompare(b.name, 'en') ||
      a.version.localeCompare(b.version, 'en'),
  );
}

function walk(dir, recursive) {
  const out = [];
  for (const entry of fs
    .readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory() && recursive) out.push(...walk(full, true));
    else if (entry.isFile() && LICENSE_FILE.test(entry.name)) out.push(full);
  }
  return out;
}

// The license files of a package: anywhere in a registry package; for a path
// package, its own directory, else the supplement, else the nearest ancestor.
function sources(pkg) {
  const dir = path.dirname(pkg.manifest_path);
  const id = `${pkg.name}@${pkg.version}`;
  const read = (file, base) => {
    const bytes = fs.readFileSync(file);
    const name = path.relative(base, file).split(path.sep).join('/');
    return { file: name, sha256: sha256(bytes), text: bytes.toString('utf8') };
  };
  let files = walk(dir, pkg.source != null).map((f) =>
    read(f, pkg.source != null ? dir : repoRoot),
  );
  const extra = (SUPPLEMENT[id] ?? []).map((s) => ({
    file: s.source,
    sha256: s.sha256,
    supplement: s,
  }));
  if (pkg.source == null && files.length === 0 && extra.length === 0) {
    for (
      let up = path.dirname(dir);
      files.length === 0 && up.startsWith(repoRoot);
      up = path.dirname(up)
    ) {
      files = walk(up, false).map((f) => read(f, repoRoot));
    }
  }
  return [...files, ...extra];
}

function generate() {
  const groups = new Map(); // normalized key -> { variants: Map(raw -> count), crates: Set, copyright: Map(para -> Set) }
  const byHash = new Map();
  const group = (key) => {
    if (!groups.has(key)) {
      groups.set(key, {
        id: sha256(key).slice(0, 8),
        variants: new Map(),
        crates: new Set(),
        copyright: new Map(),
      });
      byHash.set(sha256(key), key);
    }
    return groups.get(key);
  };
  const rows = [];
  const pending = [];
  for (const pkg of packages()) {
    const label = `${pkg.name} ${pkg.version}`;
    const files = sources(pkg);
    if (files.length === 0) throw new Error(`no license text for ${label}`);
    const row = {
      name: pkg.name,
      version: pkg.version,
      license: pkg.license ?? '',
      files: [],
      groups: new Set(),
    };
    rows.push(row);
    for (const f of files) {
      row.files.push({ path: f.file, sha256: f.sha256 });
      if (f.supplement && f.supplement.bodies) {
        pending.push({ row, label, s: f.supplement });
        continue;
      }
      const { copyright, units, display } = decompose(
        f.supplement ? f.supplement.text : f.text,
      );
      const keys = units.map((u, i) => {
        const key = normalize(u);
        const g = group(key);
        g.variants.set(display[i], (g.variants.get(display[i]) ?? 0) + 1);
        g.crates.add(label);
        row.groups.add(g);
        return g;
      });
      const owner = keys[0] ?? group('');
      for (const c of copyright) {
        if (!owner.copyright.has(c)) owner.copyright.set(c, new Set());
        owner.copyright.get(c).add(label);
      }
    }
  }
  for (const { row, label, s } of pending) {
    const found = s.bodies.map((hash) => groups.get(byHash.get(hash)));
    if (found.some((g) => g == null))
      throw new Error(`${label}: a supplement text is not in any package`);
    for (const g of found) {
      g.crates.add(label);
      row.groups.add(g);
    }
    const g = found[0];
    for (const c of s.copyright ?? []) {
      if (!g.copyright.has(c)) g.copyright.set(c, new Set());
      g.copyright.get(c).add(label);
    }
  }
  return {
    rows,
    groups: [...groups.values()].filter((g) => g.crates.size > 0),
  };
}

const byCount = (a, b) =>
  b.crates.size - a.crates.size || (a.id < b.id ? -1 : 1);

function render({ rows, groups }) {
  groups.sort(byCount);
  const out = [
    '# Rust dependency license texts',
    '',
    "License and notice texts of every Cargo package in the Node addon's dependency graph",
    `(\`${SCOPE}\`), which includes procedural macros and`,
    'target-specific crates. Generated by `scripts/licenses.cjs`; `LICENSE_PROVENANCE.json` lists',
    'the source file of each text. Texts that differ only in whitespace, URL scheme, a license title',
    "or the Apache-2.0 appendix template appear once; each package's copyright lines are kept verbatim.",
    '',
    '| Package | Declared license | Texts |',
    '|---|---|---|',
    ...rows.map(
      (r) =>
        `| ${r.name} ${r.version} | ${r.license} | ${[...r.groups]
          .sort(byCount)
          .map((g) => g.id)
          .join(', ')} |`,
    ),
  ];
  for (const g of groups) {
    const [text] = [...g.variants].sort(
      (a, b) =>
        b[1] - a[1] || b[0].length - a[0].length || (a[0] < b[0] ? -1 : 1),
    )[0] ?? [''];
    out.push(
      '',
      `## Text ${g.id}`,
      '',
      `Packages: ${[...g.crates].join(', ')}`,
    );
    if (text) out.push('', text);
    if (g.copyright.size) {
      const lines = [...g.copyright]
        .map(
          ([para, crates]) =>
            `${[...crates].join(', ')}:\n  ${para.split('\n').join('\n  ')}`,
        )
        .sort();
      out.push('', 'Copyright notices:', '', '```text', ...lines, '```');
    }
  }
  return out.join('\n') + '\n';
}

function provenance({ rows }) {
  const lines = rows.map((r) =>
    JSON.stringify({
      name: r.name,
      version: r.version,
      license: r.license,
      files: r.files,
    }),
  );
  return `{\n  "scope": ${JSON.stringify(SCOPE)},\n  "packages": [\n    ${lines.join(',\n    ')}\n  ]\n}\n`;
}

function main() {
  const result = generate();
  const files = [
    [MD, render(result)],
    [JSON_FILE, provenance(result)],
  ];
  if (process.argv.includes('--check')) {
    const stale = files.filter(
      ([file, text]) =>
        !fs.existsSync(file) || fs.readFileSync(file, 'utf8') !== text,
    );
    if (stale.length) {
      console.error(
        `Out of date: ${stale.map(([f]) => path.basename(f)).join(', ')}. Run node scripts/licenses.cjs.`,
      );
      process.exit(1);
    }
    console.log('License inventory is current.');
  } else {
    for (const [file, text] of files) fs.writeFileSync(file, text);
  }
}

if (require.main === module) main();
module.exports = { decompose, normalize, sha256, generate };
