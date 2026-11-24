'use strict';
// Perf repro: member access through a chain of top-level constant objects.
// Each level looks up keys of the level below. Babel and #2008 are linear here.
//   node tests/perf/const-chain.cjs [path/to/js/index.js]
const path = require('node:path');
const c = require(path.resolve(process.argv[2] || path.join(__dirname, '../../js/index.js')));
for (const n of [20, 40, 80]) {
  const keys = Array.from({ length: n }, (_, i) => `c${i}`);
  let s = "import * as stylex from '@stylexjs/stylex';\n";
  s += `const a = {${keys.map((k, i) => `${k}: '#${100000 + i}'`).join(', ')}};\n`;
  s += `const b = {${keys.map((k) => `${k}: a.${k}`).join(', ')}};\n`;
  s += `const d = {${keys.map((k) => `${k}: b.${k}`).join(', ')}};\n`;
  s += `export const t = stylex.create({x: {${keys.map((k, i) => `'--v${i}': d.${k}`).join(', ')}}});\n`;
  const t0 = Date.now();
  c.transformSync(s, {});
  console.log(`${n} keys, 3 levels: ${Date.now() - t0} ms`);
}
