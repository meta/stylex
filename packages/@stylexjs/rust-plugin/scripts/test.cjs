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
// `--js-only` skips the Rust tests, for runs on other Node versions.
const jsOnly = process.argv.includes('--js-only');
const nodeSuites = [
  'babel_facade_current',
  'native_api_current',
  'host_boundary_current',
  'babel_validation_current',
  'creation_data_boundary_current',
  'compiled_consumer_current',
  'named_env_callee_site_current',
  'host_memo_current',
  'creation_callback_effects_current',
  'theme_projection_current',
  'count_refusal_current',
  'memo_parameter_identity_current',
  'hole_spread_diagnostics_current',
  'unplugin_current',
  'benchmark_current',
];
const cargoTest = [
  'cargo',
  [
    'test',
    '--locked',
    '--no-fail-fast',
    '--workspace',
    '--manifest-path',
    path.join(root, 'Cargo.toml'),
  ],
];
const steps = [
  ...(jsOnly ? [] : [cargoTest]),
  ...nodeSuites.map((name) => [
    process.execPath,
    ['--test', path.join(root, 'tests', `${name}.cjs`)],
  ]),
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
