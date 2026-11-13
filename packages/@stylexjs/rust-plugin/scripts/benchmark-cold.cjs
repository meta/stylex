/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Start the clock first, so cold samples include module loading.
const start = process.hrtime.bigint();
const [engine, name] = process.argv.slice(2);
if (engine !== 'babel' && engine !== 'rust') {
  throw new Error('Expected babel or rust benchmark engine');
}

// Arguments only select a compiler and a known fixture; nothing is evaluated.
const { cases, loadEngine } = require('./benchmark.cjs');
const run = loadEngine(engine);
const entry = cases().find((item) => item.name === name);
if (entry == null) throw new Error('Unknown benchmark fixture');
for (const input of entry.files) run(input);
process.stdout.write(String(Number(process.hrtime.bigint() - start) / 1e6));
