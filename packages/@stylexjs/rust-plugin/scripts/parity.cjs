/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const path = require('node:path');
const fs = require('node:fs');
const { execFileSync, spawnSync } = require('node:child_process');

const packageRoot = path.resolve(__dirname, '..');
const repository = path.resolve(packageRoot, '../../..');
const babelRoot = path.join(repository, 'packages/@stylexjs/babel-plugin');
const engine = process.argv[2] || 'rust';
if (!['rust', 'babel'].includes(engine))
  throw new Error('Expected rust or babel engine');
const output = path.resolve(
  process.env.STYLEX_TEST_OUTPUT || path.join(repository, '.build/rust-parity'),
);
fs.mkdirSync(output, { recursive: true });
const tracked = execFileSync('git', ['-C', repository, 'ls-files'], {
  encoding: 'utf8',
})
  .trim()
  .split('\n');
const tests = tracked.filter(
  (file) =>
    file.startsWith('packages/@stylexjs/babel-plugin/') &&
    file.includes('/__tests__/') &&
    file.endsWith('.js') &&
    !file.includes('/__fixtures__/'),
);
if (!tests.length) throw new Error('No tracked Babel tests discovered');
const config = require(path.join(babelRoot, 'jest.config.js'));
const temporaryConfig = path.join(output, `${engine}.config.json`);
fs.writeFileSync(
  temporaryConfig,
  JSON.stringify({
    ...config,
    rootDir: babelRoot,
    ...(engine === 'rust'
      ? {
          collectCoverage: false,
          coverageThreshold: undefined,
          setupFilesAfterEnv: [path.join(__dirname, 'native-jest-setup.cjs')],
        }
      : {
          setupFilesAfterEnv: [path.join(__dirname, 'unfocus-jest-setup.cjs')],
        }),
  }),
);
const args = [
  path.join(repository, 'node_modules/jest/bin/jest.js'),
  '--config',
  temporaryConfig,
  '--runInBand',
  '--ci',
  '--runTestsByPath',
  ...tests.map((file) => path.join(repository, file)),
  '--json',
  `--outputFile=${path.join(output, `${engine}.results.json`)}`,
];
if (engine === 'babel') args.push('--coverage');
const result = spawnSync(process.execPath, args, {
  cwd: babelRoot,
  env: process.env,
  stdio: 'inherit',
});
process.exitCode = result.status == null ? 1 : result.status;
