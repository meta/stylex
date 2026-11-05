/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const root = path.resolve(__dirname, '..');
const result = spawnSync(
  'cargo',
  [
    'build',
    '--locked',
    '--manifest-path',
    path.join(root, 'Cargo.toml'),
    '--features',
    'node',
    '--release',
    '--message-format=json',
  ],
  {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    env: process.env,
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
const filename = `stylex.${process.platform}-${process.arch}.node`;
fs.copyFileSync(library, path.join(nativeDir, filename));
console.log(`Built ${filename}`);
