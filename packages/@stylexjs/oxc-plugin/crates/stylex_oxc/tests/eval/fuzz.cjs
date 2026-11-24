#!/usr/bin/env node
'use strict';
// Differential fuzzer for the evaluator: random JS expressions through the Babel plugin and
// oxs. Values are compared through defineConsts (`constVal` of `String(expr)` and
// `typeof expr`), deopt reasons and thrown errors through create().
//   node crates/stylex_oxc/tests/eval/fuzz.cjs [--count 3000] [--seed 1] [--show 20]
const path = require('node:path');
const fs = require('node:fs');
const { createRequire } = require('node:module');

const REPO = path.resolve(__dirname, '../../../../../../..');
const req = createRequire(path.join(REPO, 'package.json'));
const babel = req('@babel/core');
const plugin = req('./packages/@stylexjs/babel-plugin/lib/index.js');
const oxs = require(process.env.OXS_JS || path.resolve(__dirname, '../../../../js/index.js'));

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, all) => (a.startsWith('--') ? [...acc, [a.slice(2), all[i + 1]]] : acc), []),
);
const COUNT = Number(args.count || 3000);
let seed = Number(args.seed || 1);
const SHOW = Number(args.show || 20);
function rand() {
  seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
  return seed / 4294967296;
}
const pick = (a) => a[Math.floor(rand() * a.length)];
const chance = (p) => rand() < p;

const NUMS = ['0', '1', '2', '3', '10', '0.5', '1.5', '(-1)', '1e21', '1e-7', '0x10', '255', '0.1', '100', '7', '(-0)', '2.5', '(1/3)'];
const STRS = ["''", "'a'", "'1'", "' 12 '", "'0x1f'", "'abc'", "'1e3'", "'red'", "'Infinity'", "'-3'", "'a,b'", "'px'"];
// `fn` alone is left out: String(closure) is Babel's own source text, which CSS then rejects.
const ATOMS = ['true', 'false', 'null', 'undefined', 'NaN', 'Infinity', 'nope', 'k', 'arr', 'obj', 'v1', 'v2', 'v3', 'o1.p', 'o1[k]', 'f1(2)', 'f2(k, 1)'];
const BIN = ['+', '-', '*', '/', '%', '**', '<', '>', '<=', '>=', '==', '!=', '===', '!==', '|', '&', '^', '<<', '>>', '>>>', 'in'];
const UN = ['-', '+', '!', '~', 'typeof ', 'void '];
const LOG = ['||', '&&', '??'];
const STR_METHODS = [
  ['toUpperCase', 0], ['slice', 2], ['indexOf', 1], ['includes', 1], ['padStart', 2], ['repeat', 1],
  ['split', 1], ['charAt', 1], ['trim', 0], ['at', 1], ['concat', 1], ['replace', 2], ['startsWith', 1],
];
const ARR_METHODS = [['join', 1], ['slice', 1], ['indexOf', 1], ['includes', 1], ['concat', 1], ['at', 1], ['map', 'fn'], ['filter', 'fn']];
const STATIC = [
  ['Math.max', 2], ['Math.min', 2], ['Math.round', 1], ['Math.floor', 1], ['Math.abs', 1], ['Math.pow', 2],
  ['Math.sign', 1], ['Math.trunc', 1], ['Math.sqrt', 1], ['Number.parseFloat', 1], ['Number.parseInt', 1],
  ['Number.isInteger', 1], ['Object.keys', 1], ['Object.values', 1], ['Array.isArray', 1], ['String', 1],
  ['Number', 1], ['String.fromCharCode', 1], ['Array.of', 2],
];

function gen(d) {
  if (d <= 0 || chance(0.25)) {
    const r = rand();
    if (r < 0.4) return pick(NUMS);
    if (r < 0.75) return pick(STRS);
    return pick(ATOMS);
  }
  const r = rand();
  if (r < 0.22) return `(${gen(d - 1)} ${pick(BIN)} ${gen(d - 1)})`;
  if (r < 0.32) return `(${pick(UN)}${gen(d - 1)})`;
  if (r < 0.4) return `(${gen(d - 1)} ${pick(LOG)} ${gen(d - 1)})`;
  if (r < 0.46) return `(${gen(d - 1)} ? ${gen(d - 1)} : ${gen(d - 1)})`;
  if (r < 0.52) return `\`a\${${gen(d - 1)}}b\${${gen(d - 1)}}\``;
  if (r < 0.58) return `[${gen(d - 1)}, ${gen(d - 1)}]`;
  if (r < 0.64) return `({a: ${gen(d - 1)}, [${gen(d - 1)}]: 1})`;
  if (r < 0.68) return `({...${gen(d - 1)}, z: 2})`;
  if (r < 0.74) return `${gen(d - 1)}${pick(['.length', '[0]', '[1]', '.a', "['a']", '[k]'])}`;
  if (r < 0.82) {
    const [m, n] = pick(STATIC);
    return `${m}(${Array.from({ length: n }, () => gen(d - 1)).join(', ')})`;
  }
  if (r < 0.88) {
    const [m, n] = pick(STR_METHODS);
    return `${pick(STRS)}.${m}(${Array.from({ length: n }, () => gen(d - 1)).join(', ')})`;
  }
  if (r < 0.93) {
    const [m, n] = pick(ARR_METHODS);
    const a = n === 'fn' ? `(x) => ${gen(d - 1).replace(/\bk\b/g, 'x')}` : Array.from({ length: n }, () => gen(d - 1)).join(', ');
    return `[${gen(d - 1)}, ${gen(d - 1)}].${m}(${a})`;
  }
  if (r < 0.97) return `fn(${gen(d - 1)})`;
  return `(${gen(d - 1)}, ${gen(d - 1)})`;
}

const PRE = "const k = 'a';\nconst arr = [1, 'b'];\nconst obj = { a: 'x', b: 2 };\nconst fn = (x) => x + 1;\n";
// A random prelude: declarations that reference each other, shadowing, mutation and closures.
function prelude() {
  const out = [];
  const kinds = ['const', 'let', 'var'];
  out.push(`${pick(kinds)} v1 = ${gen(2)};`);
  out.push(`${pick(kinds)} v2 = ${chance(0.5) ? 'v1' : gen(2)};`);
  out.push(`const o1 = { p: ${gen(2)}, a: v2 };`);
  out.push(`const f1 = (x) => ${chance(0.5) ? 'x * v1' : gen(2).replace(/\bk\b/g, 'x')};`);
  out.push(`const f2 = (a, b) => ${chance(0.5) ? '`${a}-${b}`' : '[a, b]'};`);
  if (chance(0.3)) out.push(`${pick(kinds)} v3 = ${gen(2)};`);
  if (chance(0.1)) out.push(pick(['v2 = 1;', 'o1.p = 2;', 'v1++;', 'delete o1.a;', "Object.assign(o1, {});", 'o1.push(1);']));
  if (chance(0.1)) out.push(`function g() { const v1 = ${gen(1)}; return v1; }`);
  if (chance(0.1)) out.push(`if (true) { let v2 = ${gen(1)}; }`);
  return out.join('\n') + '\n';
}
const IMPORT = "import * as stylex from '@stylexjs/stylex';\n";
const FILENAME = '/project/oxs-eval/src/x.stylex.js';
const OPTIONS = { unstable_moduleResolution: { type: 'commonJS', rootDir: '/project/oxs-eval' } };

function bare(msg) {
  let m = String(msg);
  for (const p of [FILENAME + ': ', 'unknown file: ']) if (m.startsWith(p)) m = m.slice(p.length);
  const lines = m.split('\n');
  const i = lines.findIndex((l) => /^[> ] +\d+ \|/.test(l));
  return i < 0 ? m : lines.slice(0, i).join('\n');
}
function frameLine(msg) {
  const m = /^> +(\d+) \|/m.exec(String(msg));
  if (!m) return null;
  const lines = String(msg).split('\n');
  const i = lines.findIndex((l) => l.startsWith('>'));
  const bar = lines[i].indexOf('|');
  const c = lines[i + 1] ? lines[i + 1].indexOf('^') : -1;
  return `${m[1]}:${c < 0 ? '?' : c - bar - 2}`;
}
function runBabel(code) {
  try {
    const r = babel.transformSync(code, { filename: FILENAME, babelrc: false, configFile: false, plugins: [[plugin, OPTIONS]] });
    return { meta: JSON.stringify(r.metadata.stylex.map((m) => m[1].constVal ?? m[1].ltr)) };
  } catch (e) {
    return { err: bare(e.message), at: frameLine(e.message) };
  }
}
function runOxs(code) {
  try {
    const r = oxs.transformSync(code, { ...OPTIONS, filename: FILENAME });
    return { meta: JSON.stringify(r.metadata.stylex.map((m) => m[1].constVal ?? m[1].ltr)) };
  } catch (e) {
    return { err: String(e.message), at: e.loc ? `${e.loc.line}:${e.loc.column}` : null };
  }
}
// Differences outside the evaluator: parser messages, and create() with an empty string value
// (Babel crashes in its style compiler, a surface issue).
function noise(a, b) {
  const t = `${a.err || ''} ${b.err || ''}`;
  if (/oxc parse error|Identifier directly after number|Illegal expression|Unexpected token|Invalid characters/.test(t)) return true;
  if (a.err === "Cannot read properties of undefined (reading 'type')" && b.meta != null) return true;
  // The host splits error messages at NUL to carry the location.
  if (/\u0000/.test(t)) return true;
  if (/<omitted>/.test(t)) return true;
  return false;
}
const same = (a, b) => (a.err != null || b.err != null ? a.err === b.err && (a.at == null || a.at === b.at) : a.meta === b.meta);

let diffs = 0;
let noisy = 0;
let agree = 0;
const shown = new Map();
for (let i = 0; i < COUNT; i++) {
  const e = gen(Number(args.depth || 3));
  if (e === "''") continue;
  const pre = PRE + prelude();
  const consts = `${IMPORT}${pre}export const c = stylex.defineConsts({ s: String(${e}), t: typeof (${e}) });\n`;
  const create = `${IMPORT}${pre}export const styles = stylex.create({ a: { '--v': ${e} } });\n`;
  const bc = runBabel(consts);
  let ok = true;
  let detail = '';
  if (bc.err == null) {
    const oc = runOxs(consts);
    if (!same(bc, oc)) {
      ok = false;
      detail = `consts\n  babel: ${JSON.stringify(bc)}\n  oxs:   ${JSON.stringify(oc)}`;
    }
  }
  if (ok) {
    const bk = runBabel(create);
    const ok2 = runOxs(create);
    if ((bk.err != null || ok2.err != null) && !same(bk, ok2)) {
      ok = false;
      detail = `create\n  babel: ${JSON.stringify(bk)}\n  oxs:   ${JSON.stringify(ok2)}`;
    }
  }
  if (ok) agree++;
  else if (noise(...detail.split('\n').slice(1, 3).map((l) => JSON.parse(l.replace(/^ +(babel|oxs): +/, '')))) ) noisy++;
  else {
    diffs++;
    const sig = detail.replace(/"[^"]*"/g, '"_"').slice(0, 120);
    if (!shown.has(sig) && shown.size < SHOW) {
      shown.set(sig, true);
      console.log(`DIFF ${e}\n${pre}${detail}\n`);
    }
  }
}
console.log(`${agree}/${COUNT} agree, ${diffs} differ, ${noisy} outside the evaluator (seed ${args.seed || 1})`);
