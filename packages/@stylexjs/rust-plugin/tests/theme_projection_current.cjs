/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const babel = require('@babel/core');
const plugin = require('@stylexjs/babel-plugin');
const native = require('../index');
const filename = '/app/repeated-theme.stylex.js';
const syntax = {
  filename,
  babelrc: false,
  configFile: false,
  highlightCode: false,
};
function compare(source, makeOptions = () => ({ options: {}, trace: [] })) {
  const expectedOptions = makeOptions();
  const expected = babel.transformSync(source, {
    ...syntax,
    plugins: [
      [
        plugin,
        {
          unstable_moduleResolution: { type: 'haste' },
          ...expectedOptions.options,
        },
      ],
    ],
  });
  const actualOptions = makeOptions();
  const actual = native.transformSync(source, {
    unstable_moduleResolution: { type: 'haste' },
    ...actualOptions.options,
    filename,
  });
  assert.equal(babel.transformSync(actual.code, syntax).code, expected.code);
  assert.deepEqual(actual.metadata.stylex, expected.metadata.stylex);
  assert.deepEqual(
    actualOptions.trace,
    expectedOptions.trace,
    'executed host operations retain their order',
  );
  return actualOptions.trace;
}

test('repeated nested palette projections retain values, order and nulls', () => {
  compare(
    "import * as stylex from '@stylexjs/stylex';const palette={light:{accent:'blue',space:4},dark:{accent:'red',space:8}};const values={accent:{default:palette.light.accent,'@media (dark)':palette.dark.accent},space:{default:palette.light.space,'@media (dark)':palette.dark.space}};export const tokens=stylex.defineVars(values);export const light=stylex.createTheme(tokens,{accent:values.accent.default,space:values.space.default});export const dark=stylex.createTheme(tokens,{accent:values.accent['@media (dark)'],space:values.space['@media (dark)']});",
  );
});

test('projecting a host-backed cached receiver reads each getter once at its use', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';const data=stylex.env.palette;export const s=stylex.create({first:{color:data.accent},second:{color:data.accent}});";
  const trace = compare(source, () => {
    const trace = [];
    let reads = 0;
    const palette = {
      get accent() {
        trace.push('accent');
        return ++reads === 1 ? 'red' : 'blue';
      },
    };
    return { trace, options: { env: { palette } } };
  });
  assert.deepEqual(trace, ['accent', 'accent']);
});

test('unused conditional host projections stay lazy', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';const data=stylex.env.palette;export const s=stylex.create({root:{color:true?data.accent:data.unused,width:false?data.unused:4}});";
  const trace = compare(source, () => {
    const trace = [];
    const palette = {
      get accent() {
        trace.push('accent');
        return 'red';
      },
      get unused() {
        trace.push('unused');
        throw new Error('unused getter must not execute');
      },
    };
    return { trace, options: { env: { palette } } };
  });
  assert.deepEqual(trace, ['accent']);
});
