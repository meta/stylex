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

test('named env stored member and later identical callee are separate source sites', () => {
  const source =
    "import {env,create} from '@stylexjs/stylex';const twice=env.nested.double;export const styles=create({root:{width:twice(3)+2,opacity:[1,2].map(value=>env.nested.double(value))[0]}});";
  const filename = '/tmp/callee-env.js';
  const options = { env: { nested: { double: (value) => value * 2 } } };
  const syntax = {
    filename,
    babelrc: false,
    configFile: false,
    highlightCode: false,
  };
  const expected = babel.transformSync(source, {
    ...syntax,
    plugins: [[plugin, options]],
  });
  const actual = native.transformSync(source, { ...options, filename });
  assert.equal(babel.transformSync(actual.code, syntax).code, expected.code);
  assert.deepEqual(actual.metadata.stylex, expected.metadata.stylex);
});
