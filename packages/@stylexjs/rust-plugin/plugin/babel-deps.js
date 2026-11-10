/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Babel internals resolved from the host's own @babel/core.
const { createRequire } = require('node:module');
const babelCore = require('@babel/core');
const babelRequire = createRequire(require.resolve('@babel/core'));
const generate = babelRequire('@babel/generator').default;
const { NodePath } = babelRequire('@babel/traverse');
const babelParser = babelRequire('@babel/parser');
const { TraceMap, originalPositionFor } = babelRequire(
  '@jridgewell/trace-mapping',
);
const t = babelCore.types;

module.exports = {
  babelParser,
  generate,
  NodePath,
  originalPositionFor,
  t,
  TraceMap,
};
