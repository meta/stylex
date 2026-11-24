#!/usr/bin/env node
'use strict';

// Real source files through Babel and a candidate, file by file. Every file under
// the given dirs that imports StyleX runs with two option sets (production and dev,
// like the unplugin passes them). Differences are reduced to small repros.
//
//   node tools/fuzz/files.cjs --candidate js/index.js --out DIR [--dirs a,b] [--no-reduce]
//
// Default dirs: packages/docs/src, packages/shared-ui/src,
// packages/typescript-tests and packages/@stylexjs/atoms/src.

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { setup } = require('./compare.cjs');

const M = path.resolve(__dirname, '../../../../..');
const args = {};
for (let i = 2; i < process.argv.length; i++) {
  const a = process.argv[i];
  if (a.startsWith('--')) args[a.slice(2)] = process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[++i] : 'true';
}
const candidate = path.resolve(args.candidate);
const out = path.resolve(args.out);
const dirs = (args.dirs || ['packages/docs/src', 'packages/shared-ui/src', 'packages/typescript-tests', 'packages/@stylexjs/atoms/src', 'examples'].join(','))
  .split(',').map((d) => path.resolve(M, d));

function walk(d, acc = []) {
  if (!fs.existsSync(d)) return acc;
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    if (['node_modules', '__tests__', 'dist', 'build', '.next'].includes(e.name) || e.name.startsWith('.')) continue;
    const f = path.join(d, e.name);
    if (e.isDirectory()) walk(f, acc);
    else if (/\.(m?jsx?|tsx?)$/.test(e.name) && !/\.d\.ts$/.test(e.name)) acc.push(f);
  }
  return acc;
}

function pluginsFor(f) {
  if (/\.tsx$/.test(f)) return ['typescript', 'jsx'];
  if (/\.(ts|mts|cts)$/.test(f)) return ['typescript'];
  return [['flow', { all: false }], 'jsx'];
}

function pkgRoot(f) {
  let d = path.dirname(f);
  while (d !== '/' && !fs.existsSync(path.join(d, 'package.json'))) d = path.dirname(d);
  return d;
}

const files = dirs.flatMap((d) => walk(d)).filter((f) => /@stylexjs\/stylex|from ['"]stylex['"]/.test(fs.readFileSync(f, 'utf8')));
const engine = setup({ repo: M, candidate, root: M });
fs.mkdirSync(out, { recursive: true });
const report = { date: new Date().toISOString(), candidate, files: files.length, runs: 0, same: 0, diffs: [] };
const variants = [
  ['prod', (root) => ({ dev: false, treeshakeCompensation: true, unstable_moduleResolution: { type: 'commonJS', rootDir: root }, importSources: ['stylex', '@stylexjs/stylex'] })],
  ['dev', (root) => ({ dev: true, runtimeInjection: true, unstable_moduleResolution: { type: 'commonJS', rootDir: root } })],
];
const cwd = process.cwd();
for (const f of files) {
  const root = pkgRoot(f);
  process.chdir(root);
  for (const [vname, mk] of variants) {
    const input = { code: fs.readFileSync(f, 'utf8'), filename: f, options: mk(root), parserPlugins: pluginsFor(f), sourceType: 'module', processConfig: { useLayers: true } };
    const a = engine.run(engine.realPlugin, input);
    const b = engine.run(engine.adapter, input);
    report.runs++;
    let kinds = [];
    if (a.ok && b.ok) {
      if (a.code !== b.code && engine.canon(a.code, input) !== engine.canon(b.code, input)) kinds.push('code-diff');
      if (JSON.stringify(a.metadata) !== JSON.stringify(b.metadata)) {
        const s = (m) => m.map((r) => JSON.stringify(r)).sort().join('\n');
        if (s(a.metadata) !== s(b.metadata)) kinds.push('metadata-diff');
      }
      if (a.css !== b.css) kinds.push('css-diff');
    } else if (!a.ok && !b.ok) {
      if (a.error !== b.error) kinds.push('both-throw-diff');
    } else kinds.push(a.ok ? 'candidate-only-error' : 'babel-only-error');
    if (!kinds.length) {
      report.same++;
      continue;
    }
    const rel = path.relative(M, f);
    const dir = path.join(out, `${rel.replace(/[/\\]/g, '__')}.${vname}`);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, `input${path.extname(f)}`), input.code);
    fs.writeFileSync(path.join(dir, 'call.json'), JSON.stringify({ filename: f, options: input.options, parserPlugins: input.parserPlugins }, null, 1));
    if (a.ok) fs.writeFileSync(path.join(dir, 'babel.out.js'), a.code);
    if (b.ok) fs.writeFileSync(path.join(dir, 'candidate.out.js'), b.code);
    const d = { file: rel, variant: vname, kinds, babelError: a.ok ? undefined : a.error, candError: b.ok ? undefined : b.error, repro: path.relative(out, dir) };
    if (args['no-reduce'] == null && a.ok && b.ok && kinds.includes('code-diff')) {
      const red = spawnSync(process.execPath, [path.join(__dirname, 'reduce-file.cjs'), '--candidate', candidate, '--dir', dir, '--budget', '1500'], { encoding: 'utf8', timeout: 300000, cwd: root });
      const m = (red.stdout || '').match(/reduced (\d+) -> (\d+) bytes/);
      if (m) {
        const rdir = path.join(dir, 'reduced');
        d.reducedBytes = Number(m[2]);
        d.reducedCode = fs.readFileSync(path.join(rdir, fs.readdirSync(rdir).find((x) => x.startsWith('input.'))), 'utf8');
      }
    }
    report.diffs.push(d);
  }
}
process.chdir(cwd);
fs.writeFileSync(path.join(out, 'report.json'), JSON.stringify(report, null, 1));
console.log(`real files: ${report.files} files, ${report.runs} runs, ${report.same} same, ${report.diffs.length} differ`);
for (const d of report.diffs.slice(0, 20)) console.log(`  ${d.file} [${d.variant}] ${d.kinds.join(',')} ${d.candError || d.babelError || ''}`);
