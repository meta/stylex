#!/usr/bin/env node
'use strict';
// Babel's ['flow', { all: true }] parser plugin turns on the compiler's flow: 'all'.
// `stylex.props<mixed>(x)` is a call only to the Flow parser. Plain JS reads it as
// `(stylex.props < mixed) > (x)`, so it compiles only when Flow parsing is on.
// Also checks js/babel.js finds @babel/core when the working directory has none.
//   node js/test/flow-option.cjs     (from any directory)
const assert = require('node:assert');
const os = require('node:os');
const path = require('node:path');

const pluginRoot = path.resolve(__dirname, '../../../../../packages/@stylexjs/babel-plugin');
const oxs = require('../index.js');

const code = `import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({ a: { color: "red" } });
export const y = stylex.props<mixed>(styles.a);`;
const compiled = (out) => out.includes('className: "x1e2nbdu"') && !out.includes('stylex.props');
const FLOW_ALL = [['flow', { all: true }]];

// transformSync, as the harness calls it.
assert(!compiled(oxs.transformSync(code, { __parserPlugins: [] }).code), 'no flow plugin: plain JS');
assert(!compiled(oxs.transformSync(code, { __parserPlugins: ['flow'] }).code), "plain 'flow': pragma rules");
assert(compiled(oxs.transformSync(code, { __parserPlugins: FLOW_ALL }).code), 'flow all: Flow parse');
// An explicit flow option wins over the parser plugins.
assert(!compiled(oxs.transformSync(code, { flow: false, __parserPlugins: FLOW_ALL }).code), 'flow: false wins');

// createCompiler, as js/babel.js calls it.
const c = oxs.createCompiler({});
assert(compiled(c.transform(code, undefined, FLOW_ALL).code), 'createCompiler with flow all');
assert(!compiled(c.transform(code, undefined, []).code), 'createCompiler without');

// js/babel.js in a Babel config, run from a directory with no @babel/core.
process.chdir(os.tmpdir());
const babel = require(require.resolve('@babel/core', { paths: [pluginRoot] }));
const plugin = require('../babel.js');
const out = babel.transformSync(code, {
  babelrc: false,
  configFile: false,
  parserOpts: { plugins: FLOW_ALL },
  plugins: [[plugin, {}]],
}).code;
assert(compiled(out), `babel.js with flow all:\n${out}`);
console.log('flow-option: ok');
