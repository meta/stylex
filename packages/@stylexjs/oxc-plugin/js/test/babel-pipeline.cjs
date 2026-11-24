#!/usr/bin/env node
'use strict';
// js/babel.js gives the same result as @stylexjs/babel-plugin when later plugins share its
// pass, as in Next.js (next/babel). Babel's plugin folds props at Program exit, after the JSX
// transform, and TypeScript's import elision sees the source's references at Program enter.
//   node js/test/babel-pipeline.cjs     (from any directory)
const assert = require('node:assert');
const path = require('node:path');

const pluginRoot = path.resolve(__dirname, '../../../../../packages/@stylexjs/babel-plugin');
const req = (m) => require(require.resolve(m, { paths: [pluginRoot] }));
const babel = req('@babel/core');
const realMod = require(path.join(pluginRoot, 'lib/index.js'));
const real = realMod.default ?? realMod;
const adapter = require('../babel.js');

const jsx = `import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({ a: { color: "red" }, b: { padding: 1 } });
export function C({ x }) {
  return <div {...stylex.props(styles.a)} id="q"><span sx={styles.b}>hi</span><i sx={null} />
    <p {...{ title: "t" }} {...stylex.props(x && styles.a)} /></div>;
}`;
const tokens = `import * as stylex from '@stylexjs/stylex';
import { vars } from './vars.stylex';
export const marker = stylex.defineMarker();
const styles = stylex.create({ a: { color: vars.c } });
export const p = stylex.props(styles.a);`;

function run(plugin, code, filename, plugins) {
  return babel.transformSync(code, {
    babelrc: false,
    configFile: false,
    filename,
    parserOpts: { plugins: filename.endsWith('.ts') ? ['typescript'] : ['jsx'] },
    plugins: [[plugin, { unstable_moduleResolution: { type: 'haste' } }], ...plugins],
  }).code;
}

const cases = [
  ['alone', jsx, '/app/c.js', []],
  ['jsx + object spread', jsx, '/app/c.js', [[req('@babel/plugin-transform-react-jsx'), { runtime: 'automatic' }], req('@babel/plugin-transform-object-rest-spread')]],
  ['typescript import elision', tokens, '/app/t.stylex.ts', [req('@babel/plugin-transform-typescript')]],
];
for (const [name, code, filename, plugins] of cases) {
  assert.strictEqual(run(adapter, code, filename, plugins), run(real, code, filename, plugins), name);
}
console.log(`ok ${cases.length} cases`);
