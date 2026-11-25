/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const path = require('node:path');

test.only = test;
it.only = it;
describe.only = describe;
global.fit = test;
global.fdescribe = describe;

function plugin() {
  const native = require('../babel.js');
  native.default = native;
  return native;
}

jest.doMock(path.resolve(__dirname, '../../babel-plugin/src/index.js'), plugin);
jest.doMock(path.resolve(__dirname, '../../babel-plugin/lib/index.js'), plugin);
jest.doMock('@stylexjs/babel-plugin', plugin);
