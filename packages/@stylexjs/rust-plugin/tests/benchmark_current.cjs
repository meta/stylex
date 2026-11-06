/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const test = require('node:test');

const root = path.resolve(__dirname, '..');
const child = path.join(root, 'scripts/benchmark-cold.cjs');

function runChild(engine, name) {
  return spawnSync(process.execPath, [child, engine, name], {
    cwd: root,
    encoding: 'utf8',
    timeout: 30000,
  });
}

for (const engine of ['babel', 'rust']) {
  test(`cold benchmark loads ${engine} and returns a finite duration`, () => {
    const result = runChild(engine, 'tiny (create-basic)');
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /^\d+(?:\.\d+)?$/);
    const duration = Number(result.stdout);
    assert.ok(Number.isFinite(duration) && duration > 0);
  });
}

test('cold benchmark rejects unknown engines instead of evaluating arguments', () => {
  for (const engine of ['unknown', 'rust"); process.exit(93); //']) {
    const result = runChild(engine, 'tiny (create-basic)');
    assert.equal(result.status, 1);
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /Expected babel or rust benchmark engine/);
  }
});

test('cold benchmark treats quote and code-like fixture names as data', () => {
  for (const name of [
    'unknown',
    'tiny (create-basic)"); process.exit(93); //',
  ]) {
    const result = runChild('rust', name);
    assert.equal(result.status, 1);
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /Unknown benchmark fixture/);
  }
});

test('benchmark CLI preserves cold measurements and output equality', () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-benchmark-'));
  try {
    const output = path.join(directory, 'report.json');
    const result = spawnSync(
      process.execPath,
      [
        path.join(root, 'scripts/benchmark.cjs'),
        '--only=tiny',
        '--iterations=1',
        `--json=${output}`,
      ],
      { cwd: root, encoding: 'utf8', timeout: 60000 },
    );
    assert.equal(result.status, 0, result.stderr);
    const { report } = JSON.parse(fs.readFileSync(output, 'utf8'));
    assert.equal(report.length, 1);
    const row = report[0];
    assert.equal(row.name, 'tiny (create-basic)');
    assert.equal(row.comparable, true);
    assert.deepEqual(row.errors, {});
    for (const engine of ['babel', 'rust']) {
      assert.equal(row[engine].warm.samples, 1);
      assert.ok(Number.isFinite(row[engine].cold) && row[engine].cold > 0);
    }
    assert.ok(Number.isFinite(row.coldSpeedup) && row.coldSpeedup > 0);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
