/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Keep the clock before loading the benchmark module and compiler, so cold
// samples still include module loading as well as the first transform.
const start = process.hrtime.bigint();
const [engine, name] = process.argv.slice(2);
if (engine !== 'babel' && engine !== 'rust') {
  throw new Error('Expected babel or rust benchmark engine');
}

// Both the module path and the entry point are static. Arguments select a
// compiler and a known fixture; they are never interpreted as JavaScript.
const { cases, loadEngine } = require('./benchmark.cjs');
const run = loadEngine(engine);
const entry = cases().find((item) => item.name === name);
if (entry == null) throw new Error('Unknown benchmark fixture');
for (const input of entry.files) run(input);
process.stdout.write(String(Number(process.hrtime.bigint() - start) / 1e6));
