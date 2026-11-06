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

const header = "import * as stylex from '@stylexjs/stylex';";
const filename = '/app/namespace-leaves.js';
function outcome(transform) {
  try {
    const result = transform();
    return {rules: result.metadata.stylex.map(rule => rule[1].ltr)};
  } catch (error) {
    return {
      name: error.name,
      message: String(error.message).split('\n')[0].replace(/^.*?: /, ''),
    };
  }
}

for (const [label, expression, env] of [
  ['source namespace with Date leaf', '{a:{color:stylex.env.f()}}', {f: () => new Date(0)}],
  ['source namespace with Map leaf', '{a:{color:stylex.env.f()}}', {f: () => new Map()}],
  ['live plain namespace with Date leaf', '{a:stylex.env.f()}', {f: () => ({color: new Date(0)})}],
  ['Date is not a materializable namespace', '{a:stylex.env.f()}', {f: () => new Date(0)}],
]) {
  test(label, () => {
    const source = `${header}export const styles=stylex.create(${expression});`;
    const expected = outcome(() => babel.transformSync(source, {
      filename, babelrc: false, configFile: false, plugins: [[babelPlugin, {env}]],
    }));
    assert.equal(expected.name, 'Error', 'The original throws a plain validation error');
    assert.deepEqual(outcome(() => transformSync(source, {filename, env})), expected);
  });
}
