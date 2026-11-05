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
const { stripVTControlCharacters } = require('node:util');

const packageRoot = path.resolve(__dirname, '..');
const repository = path.resolve(packageRoot, '../../..');
const babelRoot = path.join(repository, 'packages/@stylexjs/babel-plugin');
// `compare` runs both engines and requires the native compiler to reproduce
// the JS compiler test for test, including any failures the unchanged suite
// has on this platform (for example Windows path separators in snapshots).
const mode = process.argv[2] || 'rust';
if (!['rust', 'babel', 'compare'].includes(mode))
  throw new Error('Expected rust, babel or compare');
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

function run(engine, coverage) {
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
            setupFilesAfterEnv: [
              path.join(__dirname, 'unfocus-jest-setup.cjs'),
            ],
          }),
    }),
  );
  const results = path.join(output, `${engine}.results.json`);
  const args = [
    path.join(repository, 'node_modules/jest/bin/jest.js'),
    '--config',
    temporaryConfig,
    '--runInBand',
    '--ci',
    '--runTestsByPath',
    ...tests.map((file) => path.join(repository, file)),
    '--json',
    `--outputFile=${results}`,
  ];
  if (coverage) args.push('--coverage');
  const result = spawnSync(process.execPath, args, {
    cwd: babelRoot,
    env: process.env,
    stdio: 'inherit',
  });
  return { status: result.status == null ? 1 : result.status, results };
}

// Failure text without colours, stack frames and the code frame Jest prints
// for the throwing line, which point into each compiler's own source.
function normalizeFailure(message) {
  return stripVTControlCharacters(message)
    .split(/\r?\n/)
    .filter(
      (line) =>
        !/^\s+at /.test(line) &&
        !/^\s*>?\s*\d+ \|/.test(line) &&
        !/^\s+\|/.test(line),
    )
    .join('\n')
    .trim();
}

function outcomes(file) {
  const report = JSON.parse(fs.readFileSync(file, 'utf8'));
  const map = new Map();
  for (const suite of report.testResults) {
    const name = path
      .relative(repository, suite.name)
      .split(path.sep)
      .join('/');
    if (suite.assertionResults.length === 0) {
      map.set(`${name} (suite)`, {
        status: suite.status,
        failure: normalizeFailure(suite.message || ''),
      });
    }
    const seen = new Map();
    for (const test of suite.assertionResults) {
      // Test names are not unique within a file; number the repeats.
      const count = (seen.get(test.fullName) ?? 0) + 1;
      seen.set(test.fullName, count);
      const suffix = count > 1 ? ` #${count}` : '';
      map.set(`${name} > ${test.fullName}${suffix}`, {
        status: test.status,
        failure: normalizeFailure(test.failureMessages.join('\n')),
      });
    }
  }
  return map;
}

if (mode !== 'compare') {
  process.exitCode = run(mode, mode === 'babel').status;
} else {
  const babel = outcomes(run('babel', false).results);
  const rust = outcomes(run('rust', false).results);
  const differences = [];
  for (const key of new Set([...babel.keys(), ...rust.keys()])) {
    const expected = babel.get(key);
    const actual = rust.get(key);
    if (
      expected?.status !== actual?.status ||
      expected?.failure !== actual?.failure
    ) {
      differences.push({ key, expected, actual });
    }
  }
  const failing = [...babel.values()].filter(
    (entry) => entry.status === 'failed',
  ).length;
  console.log(
    `JS compiler: ${babel.size} tests, ${failing} failing on this platform; native differences: ${differences.length}`,
  );
  for (const { key, expected, actual } of differences.slice(0, 50)) {
    console.log(
      `\n${key}\n  js:     ${expected?.status}\n  native: ${actual?.status}`,
    );
    if (expected?.failure !== actual?.failure) {
      console.log(
        `  js failure:\n${expected?.failure}\n  native failure:\n${actual?.failure}`,
      );
    }
  }
  process.exitCode = differences.length === 0 ? 0 : 1;
}
