/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

const {test} = require('node:test');
const assert = require('node:assert/strict');
const babel = require('@babel/core');
const babelPlugin = require('@stylexjs/babel-plugin');
const {transformSync} = require('..');

const filename = '/tmp/creation-current/callback-effects.js';
const header = "import * as stylex from '@stylexjs/stylex';";

for (const [name, value, env, keyframeCount] of [
  [
    'host-invoked compiler callbacks file their keyframes before consuming styles',
    'stylex.env.f(value=>stylex.keyframes({from:{opacity:value},to:{opacity:0}}))',
    {f: fn => fn(3)},
    1,
  ],
  [
    'each invocation of the same callback contributes effects in invocation order',
    'stylex.env.twice(value=>stylex.keyframes({from:{opacity:value},to:{opacity:0}}))',
    {twice: fn => { fn(1); return fn(3); }},
    2,
  ],
  [
    'nested host callbacks preserve captured values and forward effects once',
    'stylex.env.f(value=>stylex.env.f(other=>stylex.keyframes({from:{opacity:value+other},to:{opacity:0}})))',
    {f: fn => fn(3)},
    1,
  ],
  [
    'outer effects stay before effects from a nested host invocation',
    'stylex.env.f(value=>stylex.env.choose(stylex.keyframes({from:{opacity:value},to:{opacity:0}}),stylex.env.g(other=>stylex.keyframes({from:{opacity:other},to:{opacity:0}}))))',
    {f: fn => fn(1), g: fn => fn(3), choose: (_first, second) => second},
    2,
  ],
  [
    'outer rules filed around a nested invocation retain chronological order',
    'stylex.env.f(value=>stylex.env.choose(stylex.keyframes({from:{opacity:value},to:{opacity:0}}),stylex.env.g(other=>stylex.keyframes({from:{opacity:other},to:{opacity:0}})),stylex.keyframes({from:{opacity:value+1},to:{opacity:0}})))',
    {f: fn => fn(1), g: fn => fn(3), choose: (_first, _second, third) => third},
    3,
  ],
]) {
  test(name, () => {
    const source = `${header}export const styles=stylex.create({root:{animationName:${value}}});`;
    const options = {filename, env, debug: false, dev: false};
    const expected = babel.transformSync(source, {
      filename, babelrc: false, configFile: false,
      plugins: [[babelPlugin, {env, debug: false, dev: false}]],
    }).metadata.stylex;
    assert.equal(expected.filter(rule => rule[1].ltr.startsWith('@keyframes ')).length, keyframeCount);
    const actual = transformSync(source, options).metadata.stylex;
    // Compare whole rules: CSS text alone misses effects that were never filed.
    assert.deepEqual(actual, expected);
  });
}
