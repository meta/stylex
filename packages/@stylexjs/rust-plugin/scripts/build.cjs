/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '..');
const repository = path.resolve(root, '..', '..', '..');
const cargoHome = process.env.CARGO_HOME || path.join(os.homedir(), '.cargo');

// Release binaries name sources relative to the repository and the Cargo home,
// never by the absolute paths of the machine that built them.
const flags = process.env.CARGO_ENCODED_RUSTFLAGS
  ? process.env.CARGO_ENCODED_RUSTFLAGS.split('\x1f')
  : (process.env.RUSTFLAGS || '').split(/\s+/).filter(Boolean);
flags.push(
  `--remap-path-prefix=${repository}=stylex`,
  `--remap-path-prefix=${cargoHome}=cargo`,
);
const env = { ...process.env, CARGO_ENCODED_RUSTFLAGS: flags.join('\x1f') };
delete env.RUSTFLAGS;
// C dependencies embed __DATE__/__TIME__ (mimalloc's verbose banner); compilers
// take them from SOURCE_DATE_EPOCH, here the last commit's time, so the same
// source builds the same addon.
if (env.SOURCE_DATE_EPOCH == null) {
  const commit = spawnSync('git', ['log', '-1', '--format=%ct'], {
    cwd: root,
    encoding: 'utf8',
  });
  const time = commit.status === 0 ? commit.stdout.trim() : '';
  env.SOURCE_DATE_EPOCH = /^\d+$/.test(time) ? time : '0';
}

const filename = `stylex.${process.platform}-${process.arch}.node`;
// ld64 records the output path as a dylib's install name; Node loads addons by path.
const linkArgs =
  process.platform === 'darwin'
    ? ['--', '-C', `link-arg=-Wl,-install_name,@rpath/${filename}`]
    : [];

// The addon alone, as a final cdylib: rustc runs the release profile's LTO only
// for final artifacts, and the package also declares an `rlib` for its Rust API.
const result = spawnSync(
  'cargo',
  [
    'rustc',
    '--locked',
    '--manifest-path',
    path.join(root, 'Cargo.toml'),
    '--lib',
    '--crate-type',
    'cdylib',
    '--features',
    // Test-only diagnostics, for CI's test builds; never in npm builds.
    process.env.STYLEX_NATIVE_TEST_HOOKS === '1' ? 'node,test-hooks' : 'node',
    '--release',
    '--message-format=json',
    ...linkArgs,
  ],
  {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    env,
  },
);
if (result.error) {
  // For example cargo is not installed or not on PATH.
  console.error(`Could not run cargo: ${result.error.message}`);
  process.exit(1);
}
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) {
  for (const line of result.stdout.split('\n').filter(Boolean)) {
    try {
      const item = JSON.parse(line);
      if (item.reason === 'compiler-message')
        process.stderr.write(
          item.message.rendered || `${item.message.message}\n`,
        );
    } catch {
      process.stderr.write(`${line}\n`);
    }
  }
  if (result.error) process.stderr.write(`${result.error.message}\n`);
  process.exit(result.status || 1);
}
const artifact = result.stdout
  .split('\n')
  .filter(Boolean)
  .map((line) => JSON.parse(line))
  .find(
    (item) =>
      item.reason === 'compiler-artifact' &&
      item.target?.name === 'stylex_swc_plugin' &&
      item.target.crate_types?.includes('cdylib'),
  );
const library = artifact?.filenames.find((file) =>
  /\.(so|dylib|dll)$/.test(file),
);
if (!library)
  throw new Error('Cargo did not produce the StyleX native compiler');
const nativeDir = path.join(root, 'native');
fs.mkdirSync(nativeDir, { recursive: true });
fs.copyFileSync(library, path.join(nativeDir, filename));
console.log(`Built ${filename}`);
