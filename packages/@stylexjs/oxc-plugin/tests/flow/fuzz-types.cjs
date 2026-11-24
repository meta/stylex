#!/usr/bin/env node
'use strict';
// Random Flow type syntax for the oxc fast path (crates/flow_blank/src/ts_fast.rs).
//
//   node tests/flow/fuzz-types.cjs <flow_blank binary> [count=2000] [seed=1]
//
// Writes `count` files mixing shared Flow/TS type syntax with Flow-only and TS-only forms, then
// runs `flow_blank fastcheck` on them. Every file the fast path takes must blank byte for byte
// like Flow's parser and give the same type names. Files the fast path declines are fine.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

const [bin, countArg, seedArg] = process.argv.slice(2);
if (!bin) {
  console.error('usage: fuzz-types.cjs <flow_blank binary> [count] [seed]');
  process.exit(64);
}
const count = Number(countArg || 2000);
let seed = Number(seedArg || 1);
// mulberry32
const rnd = () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};
const pick = (a) => a[Math.floor(rnd() * a.length)];
const chance = (p) => rnd() < p;
const NL = () => (chance(0.15) ? '\n  ' : ' ');

const NAMES = ['Foo', 'Bar', 'styles', 'Props', 'T', 'React.Node', 'StyleXStyles', 'Array', 'Map', 'mixed', 'empty', 'bool'];

function type(d = 0) {
  if (d > 2) return pick(['number', 'string', 'boolean', 'any', 'void', 'null', 'mixed', 'Foo', 'T', "'a'", '1', 'true', 'symbol', 'bigint']);
  const r = rnd();
  if (r < 0.2) return pick(['number', 'string', 'boolean', 'any', 'void', 'null', 'mixed', 'empty']);
  if (r < 0.32) return pick(NAMES) + (chance(0.4) ? `<${type(d + 1)}${chance(0.3) ? ', ' + type(d + 1) : ''}>` : '');
  if (r < 0.38) return `${type(d + 1)}[]`;
  if (r < 0.46) return `${type(d + 1)} |${NL()}${type(d + 1)}`;
  if (r < 0.5) return `${type(d + 1)} & ${type(d + 1)}`;
  if (r < 0.6) {
    const members = [];
    for (let i = 0; i < Math.floor(rnd() * 3) + 1; i++) {
      members.push(pick([
        () => `${pick(['a', 'b', 'styles', 'kind'])}${chance(0.3) ? '?' : ''}: ${type(d + 1)}`,
        () => `m(${pick(['x', 'styles'])}: ${type(d + 1)}): ${type(d + 1)}`,
        () => `[${pick(['k', 'styles'])}: string]: ${type(d + 1)}`,
        () => `(x: number): ${type(d + 1)}`,
        // Flow-only and TS-only members.
        () => `+ro: ${type(d + 1)}`,
        () => `...${pick(NAMES)}`,
        () => `readonly r: ${type(d + 1)}`,
        () => `get g(): number`,
      ])());
    }
    const sep = pick([', ', '; ', ',\n  ']);
    return pick(['{', '{', '{', '{|']) + members.join(sep) + (chance(0.2) ? sep : '') + (chance(0.1) ? ', ...' : '') + '}';
  }
  if (r < 0.7) {
    const ps = [];
    for (let i = 0; i < Math.floor(rnd() * 3); i++) {
      ps.push(pick([
        () => `${pick(['x', 'y', 'styles'])}${i}: ${type(d + 1)}`,
        () => `${pick(['x', 'y'])}${i}?: ${type(d + 1)}`,
        () => type(d + 1),
        () => `{a: ${type(d + 1)}}`,
      ])());
    }
    if (chance(0.2)) ps.push(`...rest: Array<${type(d + 1)}>`);
    return `(${ps.join(', ')}) => ${type(d + 1)}`;
  }
  if (r < 0.75) return `[${type(d + 1)}, ${type(d + 1)}]`;
  if (r < 0.8) return `typeof ${pick(['styles', 'x', 'Foo.bar'])}`;
  if (r < 0.83) return `${pick(NAMES)}['${pick(['a', 'b'])}']`;
  if (r < 0.86) return `(${type(d + 1)})`;
  // Flow-only and TS-only types.
  return pick(['?Foo', '$Keys<typeof styles>', '*', 'keyof Foo', 'unknown', 'never', 'undefined', 'object', '`a${string}`', 'Foo extends Bar ? 1 : 2', '{[K in Foo]: 1}', 'readonly string[]', '$ReadOnly<{a: 1}>', 'Class<Foo>', 'interface {a: 1}']);
}

function tparams() {
  const ps = [];
  for (let i = 0; i < Math.floor(rnd() * 2) + 1; i++) {
    ps.push(pick([`T${i}`, `T${i} = ${type(2)}`, `T${i}: ${type(2)}`, `+T${i}`, `T${i} extends Foo`, `in T${i}`]));
  }
  return `<${ps.join(', ')}>`;
}

function param(i) {
  return pick([
    () => `p${i}`,
    () => `p${i}: ${type()}`,
    () => `p${i}?: ${type()}`,
    () => `p${i}?`,
    () => `p${i}: ${type()} = ${pick(['1', "'x'", 'null'])}`,
    () => `{a, b}: ${type()}`,
    () => `[a${i}, b${i}]: ${type()}`,
    () => `...rest: Array<${type()}>`,
    () => `private p${i}: number`,
  ])();
}

function params() {
  const ps = [];
  if (chance(0.15)) ps.push(`this: ${type()}`);
  for (let i = 0; i < Math.floor(rnd() * 3); i++) ps.push(param(i));
  if (ps.some((p) => p.startsWith('...'))) {
    const rest = ps.filter((p) => p.startsWith('...'));
    return [...ps.filter((p) => !p.startsWith('...')), rest[0]].join(',' + NL());
  }
  return ps.join(',' + NL());
}

function ret() {
  return chance(0.5) ? `${pick([':', '\n  :'])} ${type()}` : '';
}

function expr(d = 0) {
  if (d > 1) return pick(['x', '1', 'styles.a', 'f()', "'s'"]);
  return pick([
    () => 'x',
    () => `f<${type(2)}>(${expr(d + 1)})`,
    () => `new Map<${type(2)}, ${type(2)}>()`,
    () => `(${expr(d + 1)} as ${type(2)})`,
    () => `${expr(d + 1)} as const`,
    () => `(${expr(d + 1)}: ${type(2)})`,
    () => `(${params()})${ret()} => ${expr(d + 1)}`,
    () => `async (x: number): Promise<void> => {}`,
    () => `cond ? (${pick(['a', 'b'])}) : ${expr(d + 1)}`,
    () => `cond ? (a)${pick([': T', ''])} => b : c`,
    () => `cond ? (a: T): T => a : c`,
    () => `<Foo<number> a={1} />`,
    () => `x!`,
    () => `x satisfies Foo`,
    () => `a < b > (c)`,
    () => `function <T>(x: T): T { return x; }`,
  ])();
}

function classDecl(i) {
  const members = [];
  for (let k = 0; k < Math.floor(rnd() * 4) + 1; k++) {
    members.push(pick([
      () => `f${k}: ${type()};`,
      () => `f${k}: ${type()}`,
      () => `f${k}: ${type()} = 1;`,
      () => `static s${k}: ${type()} = 2;`,
      () => `#p${k}: ${type()};`,
      () => `declare d${k}: ${type()};`,
      () => `m${k}${chance(0.3) ? tparams() : ''}(${params()})${ret()} { return x; }`,
      () => `[computed${k}]: number = 1;`,
      () => `*gen${k}(): Iterator<number> { yield 1; }`,
      () => `[k: string]: number;`,
      () => `+cov${k}: number;`,
      () => `readonly ro${k}: number = 1;`,
      () => `public pub${k}: number = 1;`,
      () => `opt${k}?: number;`,
    ])());
  }
  const head = `class C${i}${chance(0.3) ? tparams() : ''}${chance(0.3) ? ` extends Base${chance(0.5) ? `<${type(2)}>` : ''}` : ''}${chance(0.25) ? ` implements I${i}, J${i}` : ''}`;
  return `${pick(['', 'export ', 'abstract ', 'declare '])}${head} {\n  ${members.join('\n  ')}\n}`;
}

function stmt(i) {
  return pick([
    () => `${pick(['', 'export '])}type A${i}${chance(0.3) ? tparams() : ''} = ${type()};`,
    () => `${pick(['', 'export '])}type A${i} = ${type()}`,
    () => `${pick(['', 'export '])}interface I${i}${chance(0.3) ? ` extends ${pick(NAMES)}` : ''} { a: ${type()}; m(${pick(['', 'x: number', 'styles: T, y?: string'])}): void; ${pick(['', '(z: number): string;', '[k: string]: mixed;'])} }`,
    () => `import type {${pick(['A', 'A, B', 'A as Z'])}} from './t${i}';`,
    () => `import type D${i} from './d${i}';`,
    () => `import {type X${i}, y${i}} from './m${i}';`,
    () => `import {y${i}, type X${i}} from './m${i}';`,
    () => `import {type X${i}, type W${i}} from './m${i}';`,
    () => `import D${i}, {type X${i}} from './m${i}';`,
    () => `import typeof TT${i} from './m${i}';`,
    () => `export type {A${i}} from './e${i}';`,
    () => `export {type A${i}, b${i}} from './e${i}';`,
    () => `export type * from './e${i}';`,
    () => `const v${i}: ${type()} = ${expr()};`,
    () => `let w${i}${ret()} = null;`,
    () => `function fn${i}${chance(0.3) ? tparams() : ''}(${params()})${ret()} {\n  return ${expr()};\n}`,
    () => `const af${i} = (${params()})${ret()} => ${expr()};`,
    () => classDecl(i),
    () => `opaque type O${i} = number;`,
    () => `declare var dv${i}: ${type()};`,
    () => `declare export function de${i}(): void;`,
    () => `enum E${i} {A, B}`,
    () => `namespace N${i} {}`,
    () => `function pred${i}(x: mixed): x is string { return true; }`,
    () => `function checks${i}(x: mixed): boolean %checks { return !!x; }`,
    () => `@dec class Dec${i} {}`,
    () => `${expr()};`,
    () => `(${expr()});`,
    () => `[1, 2].forEach(f);`,
  ])();
}

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'oxs-flow-types-'));
const files = [];
for (let i = 0; i < count; i++) {
  const lines = ['// @flow'];
  for (let k = 0; k < Math.floor(rnd() * 4) + 1; k++) lines.push(stmt(k));
  const f = path.join(dir, `t${i}.js`);
  fs.writeFileSync(f, lines.join('\n') + '\n');
  files.push(f);
}
const list = path.join(dir, 'list.txt');
fs.writeFileSync(list, files.join('\n'));
const r = spawnSync(bin, ['fastcheck', list], {encoding: 'utf8'});
process.stdout.write(r.stdout.split('\n').slice(-12).join('\n'));
process.exit(r.status ?? 1);
