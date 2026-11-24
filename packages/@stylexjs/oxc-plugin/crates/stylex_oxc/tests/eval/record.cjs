#!/usr/bin/env node
'use strict';
// Records the Babel plugin's result for every case in cases.cjs into expected.json.
// The Rust test eval_babel.rs replays them through oxs. Rerun after editing cases.cjs.
//   node crates/stylex_oxc/tests/eval/record.cjs [--compare]
// --compare also runs oxs (js/index.js) and prints the cases that differ, including code
// reprinted by Babel and cases with JS function options that the Rust test cannot run.
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');

const OXS = path.resolve(__dirname, '../../../..');
const REPO = path.resolve(__dirname, '../../../../../../..');
const req = createRequire(path.join(REPO, 'package.json'));
const babel = req('@babel/core');
const plugin = req('./packages/@stylexjs/babel-plugin/lib/index.js');
const { cases, compose } = require('./cases.cjs');
// Run from the repo like Jest does, so Babel's parseSync in cross-file parsing finds its config.
process.chdir(REPO);

function frameLoc(message) {
  // "> 5 |     foo: bar" followed by "    |          ^^^".
  const lines = message.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = /^> +(\d+) \|/.exec(lines[i]);
    if (m && i + 1 < lines.length) {
      const bar = lines[i].indexOf('|');
      const caretLine = lines[i + 1];
      const first = caretLine.indexOf('^');
      if (first < 0) return { line: Number(m[1]) };
      const carets = caretLine.slice(first).replace(/[^^].*$/, '').length;
      return { line: Number(m[1]), column: first - bar - 2, carets };
    }
  }
  return null;
}

function bareMessage(message, filename) {
  let msg = String(message);
  if (filename && msg.startsWith(filename + ': ')) msg = msg.slice(filename.length + 2);
  if (msg.startsWith('unknown file: ')) msg = msg.slice('unknown file: '.length);
  // Drop the code frame: the first line that starts a frame row.
  const lines = msg.split('\n');
  const idx = lines.findIndex((l) => /^[> ] +\d+ \|/.test(l));
  return idx < 0 ? msg : lines.slice(0, idx).join('\n');
}

function runBabel(c) {
  const { code, options, filename } = compose(c);
  try {
    const r = babel.transformSync(code, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: c.ts ? ['typescript', 'jsx'] : ['jsx'] },
      plugins: [[plugin, options]],
    });
    return { code: r.code, metadata: r.metadata.stylex };
  } catch (e) {
    const r = { error: bareMessage(e.message, filename), loc: frameLoc(String(e.message)) };
    // `full`: the whole message, code frame included (errors whose frame Babel builds itself).
    if (c.full) r.full = String(e.message).startsWith(filename + ': ') ? String(e.message).slice(filename.length + 2) : String(e.message);
    return r;
  }
}

function hasFunctions(v) {
  if (typeof v === 'function') return true;
  if (v && typeof v === 'object') return Object.values(v).some(hasFunctions);
  return false;
}

const out = [];
for (const c of cases) {
  const { code, options, filename } = compose(c);
  const babelResult = runBabel(c);
  out.push({
    id: c.id,
    code,
    options: { ...options, filename },
    jsOnly: hasFunctions(options) || undefined,
    expected: babelResult,
  });
}
// Paths inside this directory are stored as {{DIR}} so any checkout can replay them.
fs.writeFileSync(path.join(__dirname, 'expected.json'), JSON.stringify(out, null, 1).split(__dirname).join('{{DIR}}') + '\n');
console.log(`recorded ${out.length} cases`);

if (process.argv.includes('--compare')) {
  const oxs = require(process.env.OXS_JS || path.join(OXS, 'js/index.js'));
  let diffs = 0;
  for (const c of cases) {
    const { code, options, filename } = compose(c);
    const want = runBabel(c);
    let got;
    try {
      const r = oxs.transformSync(code, { ...options, filename });
      let printed = r.code;
      try {
        printed = babel.transformSync(r.code, {
          filename,
          babelrc: false,
          configFile: false,
          parserOpts: { plugins: c.ts ? ['typescript', 'jsx'] : ['jsx'] },
        }).code;
      } catch {}
      got = { code: printed, metadata: r.metadata.stylex };
    } catch (e) {
      got = { error: String(e.message), loc: e.loc ? { line: e.loc.line, column: e.loc.column } : null };
    }
    if (want.full != null && got.error != null) {
      if (want.full !== got.error) {
        diffs++;
        console.log(`DIFF ${c.id}\n  babel: ${JSON.stringify(want.full)}\n  oxs:   ${JSON.stringify(got.error)}`);
      }
      continue;
    }
    const a = JSON.stringify(want.error != null ? { error: want.error, line: want.loc?.line, column: want.loc?.column } : want);
    const b = JSON.stringify(got.error != null ? { error: got.error, line: got.loc?.line, column: got.loc?.column } : got);
    if (a !== b) {
      diffs++;
      console.log(`DIFF ${c.id}\n  babel: ${a}\n  oxs:   ${b}`);
    }
  }
  console.log(`${diffs} of ${cases.length} cases differ`);
}
