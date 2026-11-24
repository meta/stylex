/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Records the Babel plugin's inputs from its own Jest suite into a JSONL file, for the
// profile-guided build's training run.
//   node scripts/record-suite-inputs.cjs <out.jsonl>

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const out = path.resolve(process.argv[2] || '');
if (!process.argv[2]) throw new Error('usage: record-suite-inputs.cjs <out.jsonl>');
const repo = path.resolve(__dirname, '../../../..');
const babelRoot = path.join(repo, 'packages/@stylexjs/babel-plugin');
const config = require(path.join(babelRoot, 'jest.config.js'));
fs.rmSync(out, { force: true });
const configFile = `${out}.jest.json`;
fs.writeFileSync(
  configFile,
  JSON.stringify({
    ...config,
    rootDir: babelRoot,
    collectCoverage: false,
    coverageThreshold: undefined,
    setupFilesAfterEnv: [path.join(__dirname, 'record-jest-setup.cjs')],
  }),
);
const result = spawnSync(
  process.execPath,
  [
    path.join(repo, 'node_modules/jest/bin/jest.js'),
    '--config',
    configFile,
    '--runInBand',
    '--ci',
    '--silent',
    '--watchman=false',
    path.join(babelRoot, '__tests__'),
  ],
  { cwd: babelRoot, env: { ...process.env, OXC_RECORD_FILE: out }, stdio: ['ignore', 'ignore', 'pipe'] },
);
fs.rmSync(configFile, { force: true });
const lines = fs.existsSync(out) ? fs.readFileSync(out, 'utf8').split('\n').filter(Boolean).length : 0;
if (!lines) {
  console.error(String(result.stderr).slice(-2000));
  throw new Error('recorded no inputs');
}
console.log(`recorded ${lines} Babel-suite inputs`);
