// Early errors: oxs against Babel on every snippet in cases.cjs. Both must throw the same
// SyntaxError (message and position), or neither. Each snippet runs in five forms:
//   plain   after a StyleX import, as a .js file (Babel's flow and jsx plugins)
//   ts      the same as a .ts file (Babel's typescript plugin)
//   flow    `// @flow` plus a type annotation (the oxc fast path for Flow)
//   flowp   `// @flow` plus Flow-only syntax (Flow's parser, then blanking)
//   lower   `// @flow` plus a `match` (lowered text, hermes-parser on Babel's side). hermes has
//           its own messages, so only "both throw or neither" counts here.
//
//   node tests/early/run.cjs [--show]
// Exits 1 when a case differs that is not in KNOWN.

const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');
const SM = process.env.STYLEX_MAIN || path.resolve(__dirname, '../../../../..');
const babel = require(require.resolve('@babel/core', {paths: [SM]}));
const hermes = require.resolve('babel-plugin-syntax-hermes-parser', {paths: [SM]});
const oxs = require(process.env.OXS || path.join(ROOT, 'js/index.js'));
const cases = require('./cases.cjs');

// Differences we keep. Generic parse errors where oxc's wording differs, and Flow files with
// runtime syntax where Flow's or oxc's parser rejects what hermes-parser accepts.
const KNOWN = new Set([
  ...['plain', 'ts', 'flow', 'flowp'].flatMap((f) => [
    `${f} lexical-in-if`,
    `${f} optional-chain-assign`,
    `${f} unexpected-token`,
  ]),
  'lower export-dup',
  'lower export-dup-default',
  'lower regex-flags',
  'lower private-in-ok',
]);

const IMP = "import * as stylex from '@stylexjs/stylex';\n";
const forms = {
  plain: {pre: IMP, file: '/t/x.js', plugins: ['flow', 'jsx']},
  ts: {pre: IMP, file: '/t/x.ts', plugins: [['typescript', {}]]},
  flow: {pre: '// @flow\n' + IMP + 'const __t: number = 1;\n', file: '/t/x.js', plugins: ['flow', 'jsx']},
  flowp: {pre: '// @flow\n' + IMP + 'type __T = ?number;\n', file: '/t/x.js', plugins: ['flow', 'jsx']},
  lower: {pre: '// @flow\n' + IMP + "const __m = match (1) { 1 => 'a', _ => 'b' };\n", file: '/t/x.js', hermes: true},
};
const strip = (m) => String(m).replace(/^\/t\/x\.(js|ts): /, '').split('\n')[0];
const tally = {};
const unexpected = [];
const known = [];
for (const [form, f] of Object.entries(forms)) {
  for (const [id, snippet] of cases) {
    if (form === 'ts' && /<div|type T/.test(snippet)) continue;
    const code = f.pre + snippet;
    let b = null, o = null, bLoc = null, oLoc = null;
    try {
      const opts = {filename: f.file, babelrc: false, configFile: false, sourceType: 'module'};
      if (f.hermes) babel.parseSync(code, {...opts, plugins: [hermes]});
      else babel.parseSync(code, {...opts, parserOpts: {plugins: f.plugins}});
    } catch (e) {
      b = strip(e.message);
      bLoc = e.loc ? `${e.loc.line}:${e.loc.column}` : null;
    }
    try {
      oxs.transformSync(code, {filename: f.file});
    } catch (e) {
      o = strip(e.message);
      oLoc = e.loc ? `${e.loc.line}:${e.loc.column}` : null;
    }
    const same = f.hermes ? (b == null) === (o == null) : b === o && (b == null || bLoc === oLoc);
    const k = same ? 'same' : b == null ? 'oxs-only' : o == null ? 'babel-only' : 'diff';
    tally[`${form} ${k}`] = (tally[`${form} ${k}`] || 0) + 1;
    if (!same) {
      const line = `${form.padEnd(6)} ${id.padEnd(28)} babel: ${b} @${bLoc}\n${' '.repeat(36)}oxs:   ${o} @${oLoc}`;
      (KNOWN.has(`${form} ${id}`) ? known : unexpected).push(line);
    }
  }
}
// Cross-file: an imported .stylex file with an early error. Babel reports the imported file's
// error (`unknown: <message> (line:column)` and its code frame) at the import.
{
  const os = require('node:os');
  const pluginRoot = path.join(SM, 'packages/@stylexjs/babel-plugin');
  const stylexMod = require(path.join(pluginRoot, 'lib/index.js'));
  const plugin = stylexMod.default ?? stylexMod;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'oxs-early-'));
  const main = path.join(dir, 'm.js');
  const code = IMP + "import {a} from './t.stylex';\nexport const s = stylex.create({x: {color: a}});\n";
  fs.writeFileSync(main, code);
  const opts = {unstable_moduleResolution: {type: 'experimental_crossFileParsing', rootDir: dir}};
  for (const [id, body] of [
    ['cross-export-redecl', 'export const a = 1;\nexport const a = 2;\n'],
    ['cross-let-redecl', 'let q = 1;\nlet q = 2;\nexport const a = "red";\n'],
    ['cross-break', 'break;\nexport const a = "red";\n'],
  ]) {
    fs.writeFileSync(path.join(dir, 't.stylex.js'), body);
    let b = null, o = null;
    try {
      babel.transformSync(code, {filename: main, babelrc: false, configFile: false, plugins: [[plugin, opts]]});
    } catch (e) {
      b = String(e.message).replace(main + ': ', '');
    }
    try {
      oxs.transformSync(code, {...opts, filename: main});
    } catch (e) {
      o = String(e.message);
    }
    const k = b === o ? 'same' : 'diff';
    tally[`cross ${k}`] = (tally[`cross ${k}`] || 0) + 1;
    if (k !== 'same') unexpected.push(`cross  ${id.padEnd(28)} babel: ${JSON.stringify(b)}\n${' '.repeat(36)}oxs:   ${JSON.stringify(o)}`);
  }
  fs.rmSync(dir, {recursive: true, force: true});
}
console.log(JSON.stringify(tally));
if (process.argv.includes('--show') && known.length) console.log('known:\n' + known.join('\n'));
if (unexpected.length) {
  console.log('unexpected:\n' + unexpected.join('\n'));
  process.exit(1);
}
