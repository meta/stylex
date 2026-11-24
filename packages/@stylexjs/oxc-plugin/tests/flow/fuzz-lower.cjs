#!/usr/bin/env node
'use strict';
// Random differential test of the Flow runtime-syntax lowering against hermes-parser.
//
//   node tests/flow/fuzz-lower.cjs <flow_blank binary> [count=300] [seed=1]
//
// Generates files with random `match` expressions and statements, components and hooks, lowers
// each with `flow_blank lower`, and checks that the lowered text normalizes (hermes parse, strip
// types, Babel generator) to the same program as hermes-parser's own lowering of the original.
// Inputs hermes rejects are skipped; flow_blank must then reject them too, or it is reported.

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const {execFileSync} = require('node:child_process');

const ROOT = path.resolve(__dirname, '../..');
const SM = process.env.STYLEX_MAIN || path.resolve(__dirname, '../../../../..');
const babel = require(require.resolve('@babel/core', {paths: [SM]}));
const hermes = require.resolve('babel-plugin-syntax-hermes-parser', {paths: [SM]});
const strip = require.resolve('@babel/plugin-transform-flow-strip-types', {paths: [SM]});

const [bin, countArg, seedArg] = process.argv.slice(2);
const count = Number(countArg || 300);
let seed = Number(seedArg || 1);
const rnd = () => ((seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648);
const pick = (a) => a[Math.floor(rnd() * a.length)];
const chance = (p) => rnd() < p;

function norm(code) {
  return babel
    .transformSync(code, {
      filename: 'x.js',
      babelrc: false,
      configFile: false,
      comments: false,
      parserOpts: {flow: 'all'},
      plugins: [hermes, [strip, {allowDeclareFields: true}]],
    })
    .code.replace(/^\s*;\s*$/gm, '')
    .replace(/\n\s*\n/g, '\n');
}

let names = 0;
const fresh = () => `v${names++}`;
const lit = () => pick(['1', '2', '-3', "'a'", '"b"', 'true', 'false', 'null', '10n', 'undefined', 'NaN', 'Foo.Bar', "Foo['x-y'].z"]);

function pattern(depth, bind = true) {
  const r = rnd();
  if (depth > 2 || r < 0.3) return pick([lit, lit, () => '_', () => (bind ? `const ${fresh()}` : '_')])();
  if (r < 0.45) {
    const els = Array.from({length: Math.floor(rnd() * 3)}, () => pattern(depth + 1, bind));
    if (chance(0.4)) els.push(bind && chance(0.5) ? `...const ${fresh()}` : '...');
    return `[${els.join(', ')}]`;
  }
  if (r < 0.65) {
    const keys = ['a', 'b', "'c-d'", '0', 'kind'];
    const used = new Set();
    const props = [];
    for (let i = 0; i < Math.floor(rnd() * 3) + 1; i++) {
      const k = pick(keys);
      if (used.has(k)) continue;
      used.add(k);
      if (bind && chance(0.25) && /^[a-z]+$/.test(k)) props.push(`const ${k}`);
      else props.push(`${k}: ${pattern(depth + 1, bind)}`);
    }
    if (chance(0.4)) props.push(bind && chance(0.5) ? `...const ${fresh()}` : '...');
    return `{${props.join(', ')}}`;
  }
  if (r < 0.75) return `${pattern(depth + 1, false)} | ${pattern(depth + 1, false)}`;
  if (r < 0.85 && bind) {
    const inner = pick([lit, () => `[${lit()}, ...]`, () => `{a: ${lit()}, ...}`])();
    return `${inner} as ${chance(0.5) ? 'const ' : ''}${fresh()}`;
  }
  if (r < 0.92) return `${pick(['Foo', 'Foo.Bar', 'Error'])} {${chance(0.5) ? `a: ${lit()}` : ''}}`;
  return lit();
}

function expr(depth) {
  if (depth > 1 || chance(0.6)) return pick(['x', '1', "'s'", 'styles.a', 'f(x)', '[x, y]', '({a: 1})', 'a ? b : c']);
  return matchExpr(depth + 1);
}

function cases(depth, body) {
  const n = Math.floor(rnd() * 4) + 1;
  const out = [];
  for (let i = 0; i < n; i++) {
    const guard = chance(0.2) ? ` if (${pick(['x > 1', 'ok', 'f()'])})` : '';
    out.push(`${pattern(0)}${guard} => ${body(depth)}`);
  }
  if (chance(0.5)) out.push(`_ => ${body(depth)}`);
  return out;
}

function matchExpr(depth) {
  const arg = pick(['x', 'a.b', "a['b']", 'f()', 'this.p', 'x.y.z', '(x, y)', 'await_']);
  return `match (${arg}) {\n  ${cases(depth, expr).join(',\n  ')},\n}`;
}

function matchStmt() {
  const arg = pick(['x', 'a.b', 'f()', 'x.y']);
  const body = () => `{ ${pick(['g();', 'return 1;', 'h(x); g();', '', 'if (x) { break; }'])} }`;
  return `match (${arg}) {\n  ${cases(0, body).join('\n  ')}\n}`;
}

function component() {
  const params = [];
  const n = Math.floor(rnd() * 4);
  for (let i = 0; i < n; i++) {
    const nm = pick(['a', 'b', 'label', 'size', 'onClick']) + i;
    params.push(pick([
      `${nm}: string`,
      `${nm}?: number`,
      `${nm}: number = 1`,
      `'data-${nm}' as ${nm}: string`,
      `${nm} as {p${i}, q${i}}: P`,
      `${nm}: T = f()`,
    ]));
  }
  if (chance(0.25)) params.push('ref: R');
  if (chance(0.3)) params.push(pick(['...rest: Props', '...{c, d = 2}: {c: number, d?: number}']));
  const renders = pick(['', ' renders Foo', ' renders? Foo', ' renders* Foo']);
  const kw = pick(['', 'export ', 'export default ']);
  const name = `C${names++}`;
  return `${kw}component ${name}(${params.join(', ')})${renders} {\n  return ${chance(0.4) ? matchExpr(1) : '<div />'};\n}`;
}

function record(i) {
  const els = [];
  const keys = ['a', 'b', 'default', "'c-d'", '1'];
  const used = new Set();
  for (let k = 0; k < Math.floor(rnd() * 4) + 1; k++) {
    const key = pick(keys);
    if (used.has(key)) continue;
    used.add(key);
    els.push(`${key}: ${pick(['number', 'string', '?T'])}${chance(0.4) ? ` = ${pick(['1', "'x'", 'f()'])}` : ''},`);
  }
  if (chance(0.4)) els.push(`static s${i}: number = ${pick(['1', 'f()'])},`);
  if (chance(0.4)) els.push(`m(): number { return ${expr(0)}; }`);
  if (chance(0.3)) els.push(`static n(): number { return 1; }`);
  return `record R${i} {\n  ${els.join('\n  ')}\n}\nconst rv${i} = R${i} {${chance(0.5) ? 'a: 1' : ''}};`;
}

function program() {
  const parts = ['// @flow'];
  let hasDefault = false;
  for (let i = 0; i < 3; i++) {
    const r = rnd();
    if (r < 0.35) parts.push(`const r${i} = ${matchExpr(0)};`);
    else if (r < 0.55) parts.push(`function s${i}() {\n  ${matchStmt()}\n}`);
    else if (r < 0.85) {
      let c = component();
      if (c.startsWith('export default')) {
        if (hasDefault) c = c.slice('export default '.length);
        hasDefault = true;
      }
      parts.push(c);
    } else if (r < 0.93) parts.push(`${chance(0.5) ? 'export ' : ''}hook useH${i}(x: number): number {\n  return ${expr(0)};\n}`);
    else parts.push(record(i));
  }
  return parts.join('\n') + '\n';
}

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'oxs-flow-fuzz-'));
const tally = {same: 0, differs: 0, bothReject: 0, hermesRejects: 0, oxsRejects: 0};
const bad = [];
for (let i = 0; i < count; i++) {
  names = 0;
  const src = program();
  const f = path.join(dir, `${i}.js`);
  fs.writeFileSync(f, src);
  let a, b, aErr, bErr;
  try {
    a = norm(src);
  } catch (e) {
    aErr = String(e.message).split('\n')[0];
  }
  try {
    b = norm(execFileSync(bin, ['lower', f], {encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe']}));
  } catch (e) {
    bErr = String(e.stderr || e.message).split('\n')[0];
  }
  if (aErr && bErr) tally.bothReject++;
  else if (aErr) {
    tally.hermesRejects++;
    bad.push({f, why: `hermes rejects, oxs lowers: ${aErr}`});
  } else if (bErr) {
    tally.oxsRejects++;
    bad.push({f, why: `oxs rejects: ${bErr}`});
  } else if (a === b) tally.same++;
  else {
    tally.differs++;
    bad.push({f, why: 'normalized output differs'});
  }
}
console.log(JSON.stringify(tally));
for (const x of bad.slice(0, 12)) console.log(`  ${x.f}: ${x.why}`);
process.exit(tally.differs ? 1 : 0);
