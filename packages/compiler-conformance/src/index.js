/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { isJsEquivalent, normalizeAst } = require('./ast');
const {
  EXPECTED_FILE,
  FIXTURES_DIR,
  MANIFEST_FILE,
  REPO_ROOT,
  getExpectedPath,
  getFixtureNames,
  loadFixture,
  readExpected,
  writeExpected,
} = require('./fixtures');
const {
  FIXTURE_ROOT_TOKEN,
  REPO_ROOT_TOKEN,
  normalizeCss,
  normalizeData,
  normalizeDiagnostic,
  normalizePaths,
} = require('./normalize');
const { exactPart, jsPart, normalizeResult } = require('./result');

module.exports = {
  EXPECTED_FILE,
  FIXTURES_DIR,
  FIXTURE_ROOT_TOKEN,
  MANIFEST_FILE,
  REPO_ROOT,
  REPO_ROOT_TOKEN,
  exactPart,
  getExpectedPath,
  getFixtureNames,
  isJsEquivalent,
  jsPart,
  loadFixture,
  normalizeAst,
  normalizeCss,
  normalizeData,
  normalizeDiagnostic,
  normalizePaths,
  normalizeResult,
  readExpected,
  writeExpected,
};
