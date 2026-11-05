/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '..');
const steps = [
  [
    'cargo',
    [
      'test',
      '--locked',
      '--no-fail-fast',
      '--manifest-path',
      path.join(root, 'Cargo.toml'),
    ],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/babel_facade_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/native_api_current.cjs')],
  ],
  [process.execPath, ['--test', path.join(root, 'tests/unplugin_current.cjs')]],
  // `compare` (used on Windows) requires the native compiler to reproduce the
  // JS compiler's results there, including the unchanged suite's own
  // platform-specific failures; elsewhere every test must pass.
  [
    process.execPath,
    [
      path.join(__dirname, 'parity.cjs'),
      process.env.STYLEX_PARITY_MODE || 'rust',
    ],
  ],
];
// Run every suite so one failure does not hide the others.
let failed = 0;
for (const [command, args] of steps) {
  const result = spawnSync(command, args, {
    cwd: root,
    env: process.env,
    stdio: 'inherit',
  });
  if (result.status !== 0) {
    failed++;
    console.error(`\nFAILED: ${[command, ...args].join(' ')}\n`);
  }
}
process.exitCode = failed === 0 ? 0 : 1;
