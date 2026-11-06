/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// Fails if the built addon embeds a build-machine path (the checkout, Cargo's
// home, the user's home), which build.cjs remaps. rustc's `/rustc/<hash>/`
// paths are fine.
'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const name = `stylex.${process.platform}-${process.arch}.node`;
const binary = fs.readFileSync(path.join(__dirname, '..', 'native', name));
const prefixes = [
  process.env.GITHUB_WORKSPACE || path.resolve(__dirname, '../../../..'),
  process.env.CARGO_HOME || path.join(os.homedir(), '.cargo'),
  os.homedir(),
].filter((prefix) => prefix.length > 3);
const found = new Set();
// On Windows a path may appear with either separator, or escaped.
for (const p of prefixes) {
  for (const needle of [
    p,
    p.replaceAll('\\', '/'),
    p.replaceAll('\\', '\\\\'),
  ]) {
    const bytes = Buffer.from(needle);
    for (
      let at = binary.indexOf(bytes);
      at !== -1;
      at = binary.indexOf(bytes, at + 1)
    ) {
      found.add(
        binary.toString('latin1', at, at + 160).split(/[^\x20-\x7e]/)[0],
      );
    }
  }
}
if (found.size > 0) {
  console.error(
    `${name} embeds build-machine paths:\n${[...found].join('\n')}`,
  );
  process.exit(1);
}
console.log(`${name}: no build-machine paths`);
