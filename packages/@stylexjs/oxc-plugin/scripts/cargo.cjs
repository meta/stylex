/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Runs cargo in this package. Flow's parser (crates/flow_blank's flow_parser dependency) still
// has a `#![feature]` line, so RUSTC_BOOTSTRAP is allowed for that one crate only.

const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '..');

function cargo(args, extraEnv = {}) {
  const result = spawnSync('cargo', args, {
    cwd: root,
    env: {
      ...process.env,
      RUSTC_BOOTSTRAP: 'flow_parser',
      // Deeply nested core tests need more than the default 2 MiB test thread stack.
      RUST_MIN_STACK: process.env.RUST_MIN_STACK || String(16 * 1024 * 1024),
      ...extraEnv,
    },
    stdio: 'inherit',
  });
  if (result.error) throw result.error;
  return result.status == null ? 1 : result.status;
}

// The root `build` and `test:packages` scripts fan out to every workspace. This package needs
// a Rust toolchain, so it only builds and tests when asked for directly
// (`yarn workspace @stylexjs/oxc-plugin build`, or `yarn build` in this directory).
function skippedByWorkspaceFanOut(what) {
  // npm and yarn set INIT_CWD to where the command started. A fan-out starts at the repo root.
  const started = process.env.INIT_CWD;
  const fanOut =
    started != null &&
    path.relative(root, path.resolve(started)).startsWith('..');
  if (fanOut && process.env.STYLEX_OXC_PLUGIN !== '1') {
    console.log(
      `@stylexjs/oxc-plugin: skipping ${what} in a workspace-wide run. ` +
        `Run \`yarn workspace @stylexjs/oxc-plugin ${what}\`, or set STYLEX_OXC_PLUGIN=1.`,
    );
    return true;
  }
  return false;
}

module.exports = { cargo, root, skippedByWorkspaceFanOut };
