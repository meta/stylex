/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// The package's test run. Build first (`yarn build`) and build the Babel plugin, which the
// comparisons use as the reference.
//   node scripts/test.cjs [--skip-cargo] [--skip-parity]
// Runs every step so one failure does not hide the others.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { cargo, root, skippedByWorkspaceFanOut } = require('./cargo.cjs');

if (skippedByWorkspaceFanOut('test')) process.exit(0);

const argv = process.argv.slice(2);
const addon = path.join(root, 'js', 'stylex_oxc.node');
if (!fs.existsSync(addon)) {
  console.error('js/stylex_oxc.node is missing. Run `yarn build` first.');
  process.exit(1);
}
const target = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, 'target');
const flowBlank = path.join(target, 'release', process.platform === 'win32' ? 'flow_blank.exe' : 'flow_blank');
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'oxc-plugin-test-'));

function node(args) {
  const result = spawnSync(process.execPath, args, { cwd: root, stdio: 'inherit' });
  return result.status == null ? 1 : result.status;
}

const steps = [];
if (!argv.includes('--skip-cargo')) {
  steps.push(['cargo test', () => cargo(['test', '--release', '--workspace', '--no-fail-fast'])]);
}
steps.push(
  [
    'golden fixtures',
    () => {
      const json = path.join(scratch, 'golden.json');
      const status = node(['tests/golden.cjs', '--json', json]);
      if (status !== 0) return status;
      const { passed, total } = JSON.parse(fs.readFileSync(json, 'utf8'));
      return passed === total && total > 0 ? 0 : 1;
    },
  ],
  ['Flow fixtures and StyleX Flow sources', () => node(['tests/flow/run.cjs', 'fixtures', 'stylex'])],
  ['Flow StyleX fuzz', () => node(['tests/flow/fuzz-stylex.cjs', '200', '1'])],
  ['Flow lowering fuzz', () => node(['tests/flow/fuzz-lower.cjs', flowBlank, '300', '1'])],
  ['Flow fast path fuzz', () => node(['tests/flow/fuzz-types.cjs', flowBlank, '2000', '1'])],
  ['early errors vs Babel', () => node(['tests/early/run.cjs'])],
  ['js: flow option', () => node(['js/test/flow-option.cjs'])],
  ['js: Babel pipeline', () => node(['js/test/babel-pipeline.cjs'])],
  ['js: options parity', () => node(['js/test/options-parity.cjs'])],
  [
    'fuzzer vs the Babel plugin',
    () => {
      const out = path.join(scratch, 'fuzz');
      const status = node([
        'tools/fuzz/run.cjs',
        '--candidate',
        'js/index.js',
        '--count',
        '1000',
        '--seed',
        'package-test-1',
        '--jobs',
        String(Math.max(1, Math.min(8, os.cpus().length))),
        '--out',
        out,
      ]);
      if (status !== 0) return status;
      const summary = JSON.parse(fs.readFileSync(path.join(out, 'summary.json'), 'utf8'));
      return summary.against === 0 ? 0 : 1;
    },
  ],
);
if (!argv.includes('--skip-parity')) {
  steps.push(['Babel plugin suite, real plugin vs js/babel.js', () => node(['scripts/parity.cjs'])]);
}

const failed = [];
for (const [name, step] of steps) {
  console.log(`\n== ${name}`);
  if (step() !== 0) {
    failed.push(name);
    console.error(`FAILED: ${name}`);
  }
}
fs.rmSync(scratch, { recursive: true, force: true });
console.log(failed.length ? `\n${failed.length} step(s) failed: ${failed.join(', ')}` : '\nall steps passed');
process.exitCode = failed.length ? 1 : 0;
