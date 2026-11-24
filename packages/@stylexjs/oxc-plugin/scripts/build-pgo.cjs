/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Opt-in profile-guided build of the addon. Builds an instrumented addon, trains it on the Babel
// plugin's test inputs and the Flow fixtures (tools/perf/pgo-train.cjs), then builds
// js/stylex_oxc.node with the profile. Same code with a better layout, so results are the same.
// Needs llvm-profdata from `rustup component add llvm-tools`.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync, execFileSync } = require('node:child_process');
const { cargo, root, skippedByWorkspaceFanOut } = require('./cargo.cjs');

if (skippedByWorkspaceFanOut('build:pgo')) process.exit(0);

const base = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, 'target');
const lib = { darwin: 'libstylex_oxc.dylib', win32: 'stylex_oxc.dll' }[process.platform] || 'libstylex_oxc.so';

function findProfdata() {
  const sysroot = execFileSync('rustc', ['--print', 'sysroot'], { encoding: 'utf8' }).trim();
  const stack = [path.join(sysroot, 'lib', 'rustlib')];
  while (stack.length) {
    const dir = stack.pop();
    let entries = [];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {}
    for (const e of entries) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) stack.push(p);
      else if (e.name === 'llvm-profdata' || e.name === 'llvm-profdata.exe') return p;
    }
  }
  return null;
}

const profdata = findProfdata();
if (!profdata) {
  console.error('build:pgo needs llvm-profdata. Run `rustup component add llvm-tools`.');
  process.exit(1);
}
const data = fs.mkdtempSync(path.join(os.tmpdir(), 'oxc-plugin-pgo-'));
const raw = path.join(data, 'raw');
const fail = (msg) => {
  fs.rmSync(data, { recursive: true, force: true });
  console.error(`build:pgo: ${msg}`);
  process.exit(1);
};

const gen = `${base}-pgo-gen`;
if (cargo(['build', '--release', '-p', 'stylex_oxc'], { CARGO_TARGET_DIR: gen, RUSTFLAGS: `-Cprofile-generate=${raw}` }) !== 0) {
  fail('instrumented build failed');
}
// Build scripts and proc macros wrote profiles too. Keep only the training run's.
fs.rmSync(raw, { recursive: true, force: true });
const instrumented = path.join(data, 'instrumented.node');
fs.copyFileSync(path.join(gen, 'release', lib), instrumented);

// Train on the Babel plugin's own test inputs and the Flow fixtures, never on the benchmark
// fixtures or example apps that the reported speed is measured on.
const suiteInputs = path.join(data, 'suite-inputs.jsonl');
if (spawnSync(process.execPath, [path.join(__dirname, 'record-suite-inputs.cjs'), suiteInputs], { stdio: 'inherit' }).status !== 0) {
  fail('recording the Babel suite inputs failed (build the Babel plugin first)');
}
const inputs = [suiteInputs, path.join(root, 'tests/flow/fixtures')];
const train = spawnSync(process.execPath, [path.join(root, 'tools/perf/pgo-train.cjs'), ...inputs], {
  cwd: root,
  env: { ...process.env, OXS_ADDON: instrumented, LLVM_PROFILE_FILE: path.join(raw, '%p-%m.profraw') },
  stdio: 'inherit',
});
if (train.status !== 0) fail('training run failed');
const profiles = fs.readdirSync(raw).filter((f) => f.endsWith('.profraw')).map((f) => path.join(raw, f));
if (!profiles.length) fail('the training run wrote no profiles');
const merged = path.join(data, 'stylex_oxc.profdata');
if (spawnSync(profdata, ['merge', '-o', merged, ...profiles], { stdio: 'inherit' }).status !== 0) fail('llvm-profdata merge failed');

const use = `${base}-pgo-use`;
if (cargo(['build', '--release', '-p', 'stylex_oxc'], { CARGO_TARGET_DIR: use, RUSTFLAGS: `-Cprofile-use=${merged}` }) !== 0) {
  fail('profile-guided build failed');
}
fs.copyFileSync(path.join(use, 'release', lib), path.join(root, 'js', 'stylex_oxc.node'));
fs.rmSync(data, { recursive: true, force: true });
console.log('built js/stylex_oxc.node (profile-guided)');
