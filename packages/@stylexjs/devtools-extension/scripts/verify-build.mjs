/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifySharedOutputs } from './extensionFiles.mjs';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
async function main() {
  const files = await verifySharedOutputs(
    path.join(root, 'dist/chrome'),
    path.join(root, 'dist/firefox'),
    path.join(root, 'dist/safari'),
  );
  console.log(`Verified ${files.length} browser extension files.`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
