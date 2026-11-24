/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Builds the napi addon (release) into js/stylex_oxc.node, and the flow_blank tool the Flow
// tests use.

const fs = require('node:fs');
const path = require('node:path');
const { cargo, root, skippedByWorkspaceFanOut } = require('./cargo.cjs');

if (skippedByWorkspaceFanOut('build')) process.exit(0);

const status = cargo(['build', '--release', '-p', 'stylex_oxc', '-p', 'flow_blank']);
if (status !== 0) process.exit(status);

const target = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, 'target');
const lib = {
  darwin: 'libstylex_oxc.dylib',
  win32: 'stylex_oxc.dll',
}[process.platform] || 'libstylex_oxc.so';
const out = path.join(root, 'js', 'stylex_oxc.node');
fs.copyFileSync(path.join(target, 'release', lib), out);
console.log(`built ${path.relative(process.cwd(), out)}`);
