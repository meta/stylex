#!/usr/bin/env node
'use strict';
// Random StyleX + Flow programs, compiled by oxs and by the Babel plugin behind
// babel-plugin-syntax-hermes-parser, compared like tests/flow/run.cjs does (errors, metadata,
// CSS, normalized code). Each program uses `stylex.create`/`stylex.props` inside components,
// hooks, `match` arms, enums and typed code.
//
//   node tests/flow/fuzz-stylex.cjs [count=200] [seed=1] [--dev]
//
// Programs that differ the same way as plain JS count as `differs-also-plain` (not Flow's).

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {compareCode, baseOptions} = require('./run.cjs');

const args = process.argv.slice(2).filter((a) => !a.startsWith('--'));
const dev = process.argv.includes('--dev');
const count = Number(args[0] || 200);
let seed = Number(args[1] || 1);
const rnd = () => ((seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648);
const pick = (a) => a[Math.floor(rnd() * a.length)];
const chance = (p) => rnd() < p;

const KEYS = ['root', 'active', 'big', 'small', 'muted'];
const PROPS = [
  "color: 'red'",
  "color: 'blue'",
  'padding: 8',
  "margin: '4px'",
  "fontSize: '1.5rem'",
  "backgroundColor: {default: 'white', ':hover': 'gray'}",
  "display: 'flex'",
  "width: {default: 100, '@media (min-width: 800px)': 200}",
];

function styleRef() {
  return pick([
    () => `styles.${pick(KEYS)}`,
    () => `cond && styles.${pick(KEYS)}`,
    () => `x ? styles.${pick(KEYS)} : styles.${pick(KEYS)}`,
    () => `[styles.${pick(KEYS)}, styles.${pick(KEYS)}]`,
    () => `match (kind) { 'a' => styles.${pick(KEYS)}, 'b' if (cond) => styles.${pick(KEYS)}, _ => null }`,
    () => `match (size) { Size.Small => styles.small, Size.Large => styles.big }`,
    () => `match (shape) { {kind: 'x', const w} => w > 1 ? styles.big : styles.small, _ => styles.${pick(KEYS)} }`,
    () => `xstyle`,
  ])();
}

function propsCall() {
  const n = Math.floor(rnd() * 3) + 1;
  return `stylex.props(${Array.from({length: n}, styleRef).join(', ')})`;
}

function jsx() {
  return pick([
    () => `<div {...${propsCall()}} />`,
    () => `<span data-x="1" {...${propsCall()}}>{label}</span>`,
    () => `<div {...${propsCall()}}>\n      <i {...${propsCall()}} />\n    </div>`,
  ])();
}

function decl(i) {
  const r = rnd();
  const params = `kind: 'a' | 'b', size: Size, shape: Shape, cond: boolean, x: boolean, label?: string = 'l', xstyle?: StyleXStyles<> = null`;
  if (r < 0.4) {
    const ref = chance(0.2) ? 'ref: React.RefSetter<HTMLElement>, ' : '';
    const kw = pick(['', 'export ']);
    const body = chance(0.3)
      ? `match (kind) {\n    'a' => { return ${jsx()}; }\n    _ => { return null; }\n  }`
      : `return ${jsx()};`;
    return `${kw}component C${i}(${ref}${params})${pick(['', ' renders Foo'])} {\n  ${body}\n}`;
  }
  if (r < 0.6) {
    return `hook useS${i}(kind: 'a' | 'b', size: Size, shape: Shape, cond: boolean, x: boolean, xstyle: ?StyleXStyles<>): {className?: string} {\n  return ${propsCall()};\n}`;
  }
  if (r < 0.8) {
    return `export function f${i}<T>(kind: 'a' | 'b', size: Size, shape: Shape, cond: boolean, x: boolean, xstyle: T): {className?: string, ...} {\n  const p: {className?: string} = ${propsCall()};\n  return (p: any);\n}`;
  }
  return `const v${i} = (kind: 'a' | 'b', size: Size, shape: Shape, cond: boolean, x: boolean, xstyle: mixed): mixed =>\n  ${propsCall()};`;
}

function program() {
  const out = [
    '// @flow',
    "import * as React from 'react';",
    "import * as stylex from '@stylexjs/stylex';",
    "import type {StyleXStyles} from '@stylexjs/stylex';",
    'enum Size {Small, Large}',
    "type Shape = {kind: 'x', w: number} | {kind: 'y'};",
  ];
  const create = () => {
    const entries = KEYS.map((k) => {
      const ps = Array.from({length: Math.floor(rnd() * 2) + 1}, () => pick(PROPS));
      return `  ${k}: {${[...new Set(ps.map((p) => p.split(':')[0]))].map((name) => ps.find((p) => p.startsWith(name))).join(', ')}},`;
    });
    return `const styles = stylex.create({\n${entries.join('\n')}\n});`;
  };
  const before = chance(0.5);
  if (before) out.push(create());
  for (let i = 0; i < 3; i++) out.push(decl(i));
  if (!before) out.push(create());
  return out.join('\n') + '\n';
}

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'oxs-flow-sx-'));
const tally = {same: 0, differs: 0, 'differs-also-plain': 0, 'both-error': 0, 'babel-error-only': 0, 'oxs-error-only': 0};
const bad = [];
for (let i = 0; i < count; i++) {
  const code = program();
  const file = path.join(dir, `p${i}.js`);
  fs.writeFileSync(file, code);
  const options = {...baseOptions(file), ...(dev ? {dev: true, debug: true} : {})};
  const r = compareCode(code, file, options, true);
  tally[r.status]++;
  if (!['same', 'both-error', 'differs-also-plain'].includes(r.status)) {
    bad.push(`${file}: ${r.status} ${(r.problems || []).join(',')} ${r.bErr || ''} ${r.oErr || ''}`);
  }
}
console.log(JSON.stringify(tally));
for (const b of bad.slice(0, 12)) console.log('  ' + b);
process.exit(tally.differs + tally['oxs-error-only'] + tally['babel-error-only'] ? 1 : 0);
