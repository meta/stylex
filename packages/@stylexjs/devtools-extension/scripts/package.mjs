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
import JSZip from 'jszip';
import { verifySharedOutputs } from './extensionFiles.mjs';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const chromeDir = path.join(root, 'dist/chrome');
const firefoxDir = path.join(root, 'dist/firefox');
const artifactsDir = path.join(root, 'artifacts');
const archiveTimestamp = new Date('2000-01-01T00:00:00.000Z');

async function writeDeterministicArchive(sourceDir, archivePath, files) {
  const archive = new JSZip();
  for (const file of files) {
    archive.file(file, await fs.readFile(path.join(sourceDir, file)), {
      createFolders: false,
      date: archiveTimestamp,
      unixPermissions: 0o100644,
    });
  }
  const contents = await archive.generateAsync({
    type: 'nodebuffer',
    compression: 'DEFLATE',
    compressionOptions: { level: 9 },
    platform: 'UNIX',
  });
  await fs.writeFile(archivePath, contents);
}

async function main() {
  const files = await verifySharedOutputs(chromeDir, firefoxDir);
  await fs.rm(artifactsDir, { recursive: true, force: true });

  for (const [browserName, sourceDir] of [
    ['chrome', chromeDir],
    ['firefox', firefoxDir],
  ]) {
    const outputDir = path.join(artifactsDir, browserName);
    const result = spawnSync(
      'web-ext',
      [
        'build',
        '--source-dir',
        sourceDir,
        '--artifacts-dir',
        outputDir,
        '--overwrite-dest',
      ],
      { cwd: root, encoding: 'utf8', stdio: 'inherit' },
    );
    if (result.error != null) throw result.error;
    if (result.status !== 0) {
      throw new Error(`web-ext build failed for ${browserName}.`);
    }
    const archiveNames = (await fs.readdir(outputDir)).filter((name) =>
      name.endsWith('.zip'),
    );
    if (archiveNames.length !== 1) {
      throw new Error(`Expected one ${browserName} archive.`);
    }
    // web-ext validates the package layout but writes current timestamps. Rewrite
    // the validated archive so identical inputs produce identical store uploads.
    await writeDeterministicArchive(
      sourceDir,
      path.join(outputDir, archiveNames[0]),
      files,
    );
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
