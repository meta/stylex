/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { transformSync } = require('@babel/core');
const stylexPlugin = require('../../src/index').default;

function transform(source, filename = '/src/main.js') {
  return transformSync(source, {
    filename,
    babelrc: false,
    plugins: [[stylexPlugin, { unstable_moduleResolution: { type: 'haste' } }]],
  });
}

module.exports = { transform, stylexPlugin };
