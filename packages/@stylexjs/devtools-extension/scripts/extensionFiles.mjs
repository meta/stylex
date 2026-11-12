/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';

const requiredFiles = new Set([
  'assets/devtools.js',
  'assets/inspected-runtime.js',
  'assets/panel.js',
  'assets/reset.css',
  'assets/stylex-icon.svg',
  'assets/stylex-icon-16.png',
  'assets/stylex-icon-32.png',
  'assets/stylex-icon-48.png',
  'assets/stylex-icon-128.png',
  'assets/stylex-icon-256.png',
  'assets/stylex-icon-512.png',
  'assets/stylex.css',
  'devtools.html',
  'manifest.json',
  'panel.html',
]);
const generatedChunkPattern = /^assets\/chunk-[A-Za-z0-9_-]+\.js$/;

async function listFiles(directory, relative = '') {
  const entries = await fs.readdir(path.join(directory, relative), {
    withFileTypes: true,
  });
  const files = [];
  for (const entry of entries) {
    const next = path.posix.join(relative, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await listFiles(directory, next)));
    } else {
      assert(entry.isFile(), `Unexpected non-file output: ${next}`);
      files.push(next);
    }
  }
  return files.sort();
}

export async function verifyBrowserOutput(directory) {
  const files = await listFiles(directory);
  const generatedChunks = files.filter((file) =>
    generatedChunkPattern.test(file),
  );
  assert(
    generatedChunks.length > 0,
    'Extension output must contain at least one generated application chunk.',
  );
  const expectedFiles = new Set([...requiredFiles, ...generatedChunks]);
  assert.deepEqual(
    files,
    Array.from(expectedFiles).sort(),
    'Extension output does not match the exact package allowlist.',
  );
  return files;
}

export async function verifySharedOutputs(
  chromeDir,
  firefoxDir,
  safariDir = null,
) {
  const outputs = [
    ['chrome', chromeDir],
    ['firefox', firefoxDir],
    ...(safariDir == null ? [] : [['safari', safariDir]]),
  ];
  const fileLists = await Promise.all(
    outputs.map(([, directory]) => verifyBrowserOutput(directory)),
  );
  const chromeFiles = fileLists[0];

  for (let index = 1; index < fileLists.length; index += 1) {
    assert.deepEqual(
      fileLists[index],
      chromeFiles,
      `${outputs[index][0]} output file list differs from Chrome.`,
    );
  }

  for (const file of chromeFiles) {
    if (file === 'manifest.json') continue;
    const contents = await Promise.all(
      outputs.map(([, directory]) => fs.readFile(path.join(directory, file))),
    );
    for (let index = 1; index < contents.length; index += 1) {
      assert(
        contents[0].equals(contents[index]),
        `Shared ${outputs[index][0]} output differs: ${file}`,
      );
    }
  }

  const chromeManifest = JSON.parse(
    await fs.readFile(path.join(chromeDir, 'manifest.json'), 'utf8'),
  );
  const firefoxManifest = JSON.parse(
    await fs.readFile(path.join(firefoxDir, 'manifest.json'), 'utf8'),
  );
  assert.equal(chromeManifest.version, '0.1.1');
  assert.equal(chromeManifest.minimum_chrome_version, '148');
  assert.equal(firefoxManifest.version, '0.1.1');
  assert.equal(
    firefoxManifest.browser_specific_settings.gecko.strict_min_version,
    '153.0',
  );
  assert.deepEqual(
    firefoxManifest.browser_specific_settings.gecko.data_collection_permissions
      .required,
    ['none'],
  );
  if (safariDir != null) {
    const safariManifest = JSON.parse(
      await fs.readFile(path.join(safariDir, 'manifest.json'), 'utf8'),
    );
    assert.equal(safariManifest.version, '0.1.1');
    assert.deepEqual(safariManifest.permissions, ['devtools']);
    assert.deepEqual(safariManifest.host_permissions, [
      'http://*/*',
      'https://*/*',
    ]);
    assert.equal(safariManifest.background, undefined);
  }
  return chromeFiles;
}
