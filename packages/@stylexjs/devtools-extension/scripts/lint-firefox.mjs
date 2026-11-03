/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import { spawnSync } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

async function main() {
  const result = spawnSync(
    'web-ext',
    ['lint', '--source-dir', 'dist/firefox', '--output', 'json'],
    { cwd: root, encoding: 'utf8' },
  );
  if (result.error != null) throw result.error;

  const report = JSON.parse(result.stdout);
  const panelLines = (
    await fs.readFile(path.join(root, 'dist/firefox/assets/panel.js'), 'utf8')
  ).split('\n');
  const unexpectedWarnings = report.warnings.filter(
    (warning) =>
      warning.code !== 'UNSAFE_VAR_ASSIGNMENT' ||
      warning.file !== 'assets/panel.js' ||
      panelLines[warning.line - 1]?.trim() !== 'domElement.innerHTML = key;',
  );

  if (
    result.status !== 0 ||
    report.errors.length > 0 ||
    report.notices.length > 0 ||
    unexpectedWarnings.length > 0
  ) {
    throw new Error(
      `Firefox lint failed:\n${JSON.stringify(
        {
          errors: report.errors,
          notices: report.notices,
          warnings: unexpectedWarnings,
        },
        null,
        2,
      )}`,
    );
  }

  console.log(
    `Firefox lint passed; ${report.warnings.length} known React DOM warnings ignored.`,
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
