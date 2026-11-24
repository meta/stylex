#!/usr/bin/env node
'use strict';
// Flow corpus test. Compiles Flow files with oxs and with the Babel plugin behind
// babel-plugin-syntax-hermes-parser (StyleX's own Babel setup), and compares
//   - errors (both throw, or neither),
//   - metadata.stylex (exact JSON),
//   - CSS (processStylexRules over each side's metadata),
//   - code, after both outputs go through the same normalizer (hermes parse, strip Flow
//     types, Babel generator without comments),
//   - and, for oxs, that the source map points into the original file.
// A file that differs is compiled again as plain JS (hermes-lowered, types stripped, lines
// kept). If the plain control differs the same way, the gap is not Flow's and the file counts
// as `differs-also-plain`.
//
//   node tests/flow/run.cjs [fixtures|stylex|react|all ...] [--json out.json] [--show]
//
// Corpora:
//   fixtures  tests/flow/fixtures/*.js, synthetic StyleX files with every Flow construct.
//             Each runs three times (defaults, dev + debug, and hermes' React 19 target with
//             enum lowering). Every run must match.
//   stylex    packages/**/src/**/*.js with a @flow pragma.
//   react     $REACT_SRC/packages/*/src/**/*.js, a facebook/react checkout.
//   dir=<p>   every .js file under <p>.
// Exit code 1 when a fixture differs. Corpus differences are reported, not fatal.

const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');
const SM = process.env.STYLEX_MAIN || path.resolve(__dirname, '../../../../..');
const X = SM;
const REACT = process.env.REACT_SRC || '';
const pluginRoot = path.join(SM, 'packages/@stylexjs/babel-plugin');
const req = (m) => require(require.resolve(m, {paths: [pluginRoot, SM]}));
const babel = req('@babel/core');
const stylexMod = require(path.join(pluginRoot, 'lib/index.js'));
const stylexPlugin = stylexMod.default ?? stylexMod;
const hermes = require.resolve('babel-plugin-syntax-hermes-parser', {paths: [SM]});
const stripTypes = require.resolve('@babel/plugin-transform-flow-strip-types', {paths: [SM]});
const oxs = require(process.env.OXS || path.join(ROOT, 'js/index.js'));

const args = process.argv.slice(2);
const flag = (k) => args.includes(k);
const jsonOut = args.includes('--json') ? args[args.indexOf('--json') + 1] : null;
const show = flag('--show');
let corpora = args.filter((a, i) => !a.startsWith('--') && args[i - 1] !== '--json');
if (corpora.length === 0) corpora = ['fixtures'];
if (corpora.includes('all')) corpora = ['fixtures', 'stylex', 'react'];

function norm(code) {
  return babel
    .transformSync(code, {
      filename: 'x.js',
      babelrc: false,
      configFile: false,
      comments: false,
      sourceType: 'unambiguous',
      parserOpts: {flow: 'all'},
      plugins: [hermes, [stripTypes, {allowDeclareFields: true}]],
    })
    .code.replace(/^\s*;\s*$/gm, '')
    .replace(/\n\s*\n/g, '\n');
}

function walk(dir, keep, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const e of fs.readdirSync(dir, {withFileTypes: true})) {
    const f = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (!['node_modules', 'lib', 'dist', 'build'].includes(e.name)) walk(f, keep, out);
    } else if (keep(f)) out.push(f);
  }
  return out;
}

function files(corpus) {
  switch (corpus) {
    case 'fixtures':
      return walk(path.join(__dirname, 'fixtures'), (f) => f.endsWith('.js')).sort();
    case 'stylex':
      return walk(path.join(SM, 'packages'), (f) => /\/src\//.test(f) && f.endsWith('.js'))
        .filter((f) => /@(no)?flow\b/.test(fs.readFileSync(f, 'utf8').slice(0, 4000)))
        .sort();
    case 'react':
      return !REACT ? [] : walk(path.join(REACT, 'packages'), (f) => /\/packages\/[^/]+\/src\//.test(f) && f.endsWith('.js')).sort();
    default:
      // `dir=<path>`: every .js file under a directory (local corpora that are not checked in).
      if (corpus.startsWith('dir=')) return walk(path.resolve(corpus.slice(4)), (f) => f.endsWith('.js')).sort();
      throw new Error(`unknown corpus ${corpus}`);
  }
}

function baseOptions(file) {
  return {
    unstable_moduleResolution: {type: 'commonJS', rootDir: path.dirname(file)},
    runtimeInjection: false,
  };
}

// `flowParserOptions` (oxs) are hermes-parser's own parser options on the Babel side.
function runBabel(code, file, options) {
  const {flowParserOptions, ...pluginOptions} = options;
  return babel.transformSync(code, {
    filename: file,
    babelrc: false,
    configFile: false,
    sourceType: 'unambiguous',
    parserOpts: flowParserOptions || {},
    plugins: [hermes, [stylexPlugin, pluginOptions]],
  });
}

function checkMap(map, code) {
  if (map == null) return null;
  if (map.sourcesContent && map.sourcesContent[0] != null && map.sourcesContent[0] !== code) {
    return 'map sourcesContent is not the original file';
  }
  return null;
}

// The same file as plain JS: hermes lowers runtime syntax, types are stripped, lines are kept.
// A difference that also shows up on this control is a general compiler gap, not a Flow one.
function plain(code) {
  return babel.transformSync(code, {
    filename: 'x.js',
    babelrc: false,
    configFile: false,
    retainLines: true,
    sourceType: 'unambiguous',
    parserOpts: {flow: 'all'},
    plugins: [hermes, [stripTypes, {allowDeclareFields: true}]],
  }).code;
}

function compare(file, options, control = true) {
  const code = fs.readFileSync(file, 'utf8');
  return compareCode(code, file, options, control);
}

function compareCode(code, file, options, control) {
  let b, o, bErr, oErr;
  try {
    b = runBabel(code, file, options);
  } catch (e) {
    bErr = String(e.message).split('\n')[0];
  }
  try {
    o = oxs.transformSync(code, {...options, filename: file});
  } catch (e) {
    oErr = String(e.message).split('\n')[0];
  }
  if (bErr && oErr) return {status: 'both-error', bErr, oErr};
  if (bErr) return {status: 'babel-error-only', bErr};
  if (oErr) {
    if (control) {
      const c = compareCode(plain(code), file, options, false);
      if (c.status === 'oxs-error-only' && c.oErr === oErr) return {status: 'differs-also-plain', oErr};
    }
    return {status: 'oxs-error-only', oErr};
  }
  const bMeta = b.metadata.stylex ?? [];
  const oMeta = o.metadata.stylex ?? [];
  const usesStylex = bMeta.length > 0 || oMeta.length > 0 || /@stylexjs\/stylex|stylex/.test(code);
  const problems = [];
  if (JSON.stringify(bMeta) !== JSON.stringify(oMeta)) problems.push('metadata');
  const bCss = stylexPlugin.processStylexRules(bMeta, {useLayers: false});
  const oCss = oxs.processStylexRules(oMeta, {useLayers: false});
  if (bCss !== oCss) problems.push('css');
  let bCode, oCode;
  try {
    bCode = norm(b.code);
    oCode = norm(o.code);
    if (bCode !== oCode) problems.push('code');
  } catch (e) {
    problems.push('normalize: ' + String(e.message).split('\n')[0]);
  }
  const mapProblem = checkMap(o.map, code);
  if (mapProblem) problems.push(mapProblem);
  if (problems.length && control) {
    let c;
    try {
      c = compareCode(plain(code), file, options, false);
    } catch {}
    // Same outputs on both sides as for the plain file: the Flow path changed nothing.
    if (c && c.out && c.out.bCode === bCode && c.out.oCode === oCode && c.out.bMeta === JSON.stringify(bMeta) && c.out.oMeta === JSON.stringify(oMeta)) {
      return {status: 'differs-also-plain', problems, rules: bMeta.length, usesStylex};
    }
  }
  if (problems.length && show && control) {
    console.log(`--- ${file}: ${problems.join(', ')}`);
    if (problems.includes('metadata')) console.log(JSON.stringify(bMeta), '\n', JSON.stringify(oMeta));
    if (problems.includes('code')) {
      const tmp = require('node:os').tmpdir();
      fs.writeFileSync(path.join(tmp, 'oxs-flow-babel.js'), bCode);
      fs.writeFileSync(path.join(tmp, 'oxs-flow-oxs.js'), oCode);
      try {
        require('node:child_process').execFileSync('diff', [path.join(tmp, 'oxs-flow-babel.js'), path.join(tmp, 'oxs-flow-oxs.js')], {stdio: 'inherit'});
      } catch {}
    }
  }
  const out = {bCode, oCode, bMeta: JSON.stringify(bMeta), oMeta: JSON.stringify(oMeta)};
  return {status: problems.length ? 'differs' : 'same', problems, rules: bMeta.length, usesStylex, out};
}

module.exports = {compareCode, baseOptions, norm};
if (require.main !== module) return;

const report = {};
let fixtureFailures = 0;
for (const corpus of corpora) {
  const list = files(corpus);
  const runs =
    corpus === 'fixtures'
      ? [
          {},
          {dev: true, debug: true},
          {flowParserOptions: {reactRuntimeTarget: '19', transformOptions: {TransformEnumSyntax: {enable: true}}}},
        ]
      : [{}];
  const tally = {files: list.length, runs: 0, same: 0, differs: 0, 'differs-also-plain': 0, 'both-error': 0, 'babel-error-only': 0, 'oxs-error-only': 0, withRules: 0};
  const bad = [];
  const ok = []; // ids that match Babel (or both error)
  for (const extra of runs) {
    for (const f of list) {
      const r = compare(f, {...baseOptions(f), ...extra});
      delete r.out;
      // ids must not depend on where the checkout sits
      const rel = f.startsWith(ROOT + path.sep) ? path.relative(ROOT, f) : path.relative(X, f);
      if (r.status === 'same' || r.status === 'both-error') ok.push(`flow > ${corpus} > ${rel}${Object.keys(extra).length ? ` ${JSON.stringify(extra)}` : ''}`);
      tally[r.status]++;
      tally.runs++;
      if (r.rules) tally.withRules++;
      if (r.status !== 'same' && r.status !== 'both-error') {
        delete r.out;
        bad.push({file: path.relative(X, f), options: extra, ...r});
        // A fixture must match Babel, or differ the same way on its plain-JS control.
        if (corpus === 'fixtures' && r.status !== 'differs-also-plain') fixtureFailures++;
      }
    }
  }
  report[corpus] = {tally, bad, ok};
  console.log(`${corpus}: ${JSON.stringify(tally)}`);
  for (const b of bad.slice(0, 15)) {
    console.log(`  ${b.status} ${b.file} ${JSON.stringify(b.options)} ${(b.problems || []).join(',')} ${b.bErr || b.oErr || ''}`);
  }
  if (bad.length > 15) console.log(`  ... ${bad.length - 15} more`);
}
if (jsonOut) fs.writeFileSync(jsonOut, JSON.stringify(report, null, 2));
process.exit(fixtureFailures ? 1 : 0);
