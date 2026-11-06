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
      '--workspace',
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
  [
    process.execPath,
    ['--test', path.join(root, 'tests/host_boundary_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/babel_validation_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/creation_data_boundary_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/compiled_consumer_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/named_env_callee_site_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/host_memo_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/creation_callback_effects_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/theme_projection_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/count_refusal_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/memo_parameter_identity_current.cjs')],
  ],
  [
    process.execPath,
    ['--test', path.join(root, 'tests/benchmark_current.cjs')],
  ],
  // `compare` (used on Windows) tolerates the suite's own platform failures.
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
    // Vendored deep-nesting tests need more than cargo test's 2 MiB thread stacks.
    env: { RUST_MIN_STACK: String(16 * 1024 * 1024), ...process.env },
    stdio: 'inherit',
  });
  if (result.status !== 0) {
    failed++;
    console.error(`\nFAILED: ${[command, ...args].join(' ')}\n`);
  }
}
process.exitCode = failed === 0 ? 0 : 1;
