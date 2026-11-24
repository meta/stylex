/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Jest setup for the oxc run of the Babel plugin's suite. Every import of the Babel plugin
// resolves to js/babel.js, the drop-in this package ships.

const path = require('node:path');

// A stray `.only` must not hide other tests. The Babel run does the same.
require('./unfocus-jest-setup.cjs');

const babelPluginRoot = path.resolve(__dirname, '../../babel-plugin');

function plugin() {
  const oxc = require('../js/babel.js');
  oxc.default = oxc;
  return oxc;
}

jest.doMock(path.join(babelPluginRoot, 'src/index.js'), plugin);
jest.doMock(path.join(babelPluginRoot, 'lib/index.js'), plugin);
jest.doMock('@stylexjs/babel-plugin', plugin);
