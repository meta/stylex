/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Runs the Babel plugin's own Jest suite twice, once with the real plugin and once with every
// import of it mocked to js/babel.js, and compares the two runs test by test.
//   node scripts/parity.cjs [--tests substr,substr]
// Exit code 1 when any test's status or normalized failure differs.

const fs = require('node:fs');
const path = require('node:path');
const { execFileSync, spawnSync } = require('node:child_process');
const { stripVTControlCharacters } = require('node:util');

const packageRoot = path.resolve(__dirname, '..');
const repo = path.resolve(packageRoot, '../../..');
const babelRoot = path.join(repo, 'packages/@stylexjs/babel-plugin');
const output = path.resolve(
  process.env.STYLEX_TEST_OUTPUT || path.join(packageRoot, '.build/parity'),
);
fs.mkdirSync(output, { recursive: true });

const argv = process.argv.slice(2);
const only = argv.includes('--tests')
  ? argv[argv.indexOf('--tests') + 1].split(',')
  : null;

function listFiles() {
  try {
    return execFileSync('git', ['-C', repo, 'ls-files', 'packages/@stylexjs/babel-plugin'], {
      encoding: 'utf8',
    })
      .trim()
      .split('\n');
  } catch {
    const out = [];
    const walk = (dir) => {
      for (const e of fs.readdirSync(path.join(repo, dir), { withFileTypes: true })) {
        const p = path.join(dir, e.name);
        if (e.name === 'node_modules' || e.name === 'lib') continue;
        if (e.isDirectory()) walk(p);
        else out.push(p.split(path.sep).join('/'));
      }
    };
    walk('packages/@stylexjs/babel-plugin');
    return out;
  }
}

const tests = listFiles().filter(
  (file) =>
    file.includes('/__tests__/') &&
    file.endsWith('.js') &&
    !file.includes('/__fixtures__/') &&
    (!only || only.some((s) => file.includes(s))),
);
if (!tests.length) throw new Error('No Babel plugin tests found');
if (!fs.existsSync(path.join(babelRoot, 'lib/index.js'))) {
  throw new Error('Build the Babel plugin first: yarn workspace @stylexjs/babel-plugin build');
}
const config = require(path.join(babelRoot, 'jest.config.js'));

function run(engine) {
  const configFile = path.join(output, `${engine}.config.json`);
  fs.writeFileSync(
    configFile,
    JSON.stringify({
      ...config,
      rootDir: babelRoot,
      collectCoverage: false,
      coverageThreshold: undefined,
      setupFilesAfterEnv: [
        path.join(__dirname, engine === 'oxc' ? 'oxc-jest-setup.cjs' : 'unfocus-jest-setup.cjs'),
      ],
    }),
  );
  const results = path.join(output, `${engine}.results.json`);
  fs.rmSync(results, { force: true });
  const t0 = Date.now();
  spawnSync(
    process.execPath,
    [
      path.join(repo, 'node_modules/jest/bin/jest.js'),
      '--config',
      configFile,
      '--runInBand',
      '--ci',
      '--silent',
      '--watchman=false',
      '--runTestsByPath',
      ...tests.map((file) => path.join(repo, file)),
      '--json',
      `--outputFile=${results}`,
    ],
    { cwd: babelRoot, env: process.env, stdio: ['ignore', 'ignore', 'pipe'] },
  );
  if (!fs.existsSync(results)) throw new Error(`jest wrote no results for ${engine}`);
  console.log(`${engine}: jest took ${((Date.now() - t0) / 1000).toFixed(1)} s`);
  return results;
}

// Strips colors, stack frames and code frames, which differ per compiler.
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
    const name = path.relative(repo, suite.name).split(path.sep).join('/');
    if (suite.assertionResults.length === 0) {
      map.set(`${name} (suite)`, {
        status: 'failed',
        failure: normalizeFailure(suite.message || ''),
      });
    }
    const seen = new Map();
    for (const test of suite.assertionResults) {
      // Test names are not unique within a file. Number the repeats.
      const count = (seen.get(test.fullName) ?? 0) + 1;
      seen.set(test.fullName, count);
      map.set(`${name} > ${test.fullName}${count > 1 ? ` #${count}` : ''}`, {
        status: test.status,
        failure: normalizeFailure(test.failureMessages.join('\n')),
      });
    }
  }
  return map;
}

const babel = outcomes(run('babel'));
const oxc = outcomes(run('oxc'));
const differences = [];
for (const key of new Set([...babel.keys(), ...oxc.keys()])) {
  const expected = babel.get(key);
  const actual = oxc.get(key);
  if (expected?.status !== actual?.status || expected?.failure !== actual?.failure) {
    differences.push({ key, expected, actual });
  }
}
const babelPassed = [...babel.values()].filter((o) => o.status === 'passed');
const bothPassed = [...babel].filter(
  ([key, o]) => o.status === 'passed' && oxc.get(key)?.status === 'passed',
).length;
const oxcPassed = [...oxc.values()].filter((o) => o.status === 'passed').length;
fs.writeFileSync(path.join(output, 'differences.json'), JSON.stringify(differences, null, 2));
console.log(
  `suite parity ${bothPassed}/${babelPassed.length} vs Babel ` +
    `(${babel.size} tests; oxc passes ${oxcPassed}; differences ${differences.length})`,
);
for (const { key, expected, actual } of differences.slice(0, 30)) {
  console.log(`\n${key}\n  babel: ${expected?.status}\n  oxc:   ${actual?.status}`);
  if (expected?.failure !== actual?.failure) {
    console.log(`  babel failure:\n${expected?.failure}\n  oxc failure:\n${actual?.failure}`);
  }
}
process.exitCode = differences.length === 0 ? 0 : 1;
