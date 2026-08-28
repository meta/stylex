/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const { normalizeResult } = require('./result');

const FIXTURES_DIR = path.join(__dirname, '..', 'fixtures');
const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');

const MANIFEST_FILE = 'manifest.json';
const EXPECTED_FILE = 'expected.json';

const DEFAULT_ENTRY = 'input.js';
const DEFAULT_SYNTAX = ['flow'];
const DEFAULT_PROCESS_OPTIONS = {
  enableLTRRTLComments: false,
  useLayers: false,
};

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

/** Every fixture directory that contains a manifest, in stable order. */
function getFixtureNames() {
  return fs
    .readdirSync(FIXTURES_DIR)
    .filter((entry) =>
      fs.existsSync(path.join(FIXTURES_DIR, entry, MANIFEST_FILE)),
    )
    .sort();
}

/** Reads a fixture manifest and resolves it against the filesystem. */
function loadFixture(name) {
  const dir = path.join(FIXTURES_DIR, name);
  const manifest = readJson(path.join(dir, MANIFEST_FILE));
  const entry = manifest.entry ?? DEFAULT_ENTRY;

  return {
    description: manifest.description ?? '',
    dir,
    entry,
    entryPath: path.join(dir, entry),
    name,
    pluginOptions: manifest.pluginOptions ?? {},
    processOptions: manifest.processOptions ?? DEFAULT_PROCESS_OPTIONS,
    repoRoot: REPO_ROOT,
    syntax: manifest.syntax ?? DEFAULT_SYNTAX,
  };
}

function getExpectedPath(name) {
  return path.join(FIXTURES_DIR, name, EXPECTED_FILE);
}

/** The recorded result for a fixture, normalized the same way live output is. */
function readExpected(name, fixture = loadFixture(name)) {
  return normalizeResult(fixture, readJson(getExpectedPath(name)));
}

/** Records a result as the new expectation for a fixture. */
function writeExpected(name, result, fixture = loadFixture(name)) {
  writeJson(getExpectedPath(name), normalizeResult(fixture, result));
}

module.exports = {
  EXPECTED_FILE,
  FIXTURES_DIR,
  MANIFEST_FILE,
  REPO_ROOT,
  getExpectedPath,
  getFixtureNames,
  loadFixture,
  readExpected,
  writeExpected,
};
