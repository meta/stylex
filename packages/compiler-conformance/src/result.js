/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const {
  normalizeCss,
  normalizeData,
  normalizeDiagnostic,
  normalizePaths,
} = require('./normalize');

/**
 * Shapes an adapter result — or a previously recorded `expected.json` — into
 * the canonical form the two are compared in. Both sides go through this
 * function so that recorded fixtures and live output are normalized
 * identically.
 *
 * See `README.md` for the adapter contract and the fixture schema.
 */
function normalizeResult(fixture, result) {
  const context = {
    entry: fixture.entry,
    fixtureDir: fixture.dir,
    repoRoot: fixture.repoRoot,
  };
  const toDiagnostics = (values) =>
    (values ?? []).map((value) => normalizeDiagnostic(value, context));

  const warnings = toDiagnostics(result.warnings);
  const errors = toDiagnostics(result.errors);

  if (result.status === 'error') {
    const message = result.error?.message ?? '';
    return {
      error: { message: normalizeDiagnostic(message, context) },
      errors,
      status: 'error',
      warnings,
    };
  }

  return {
    css: normalizeCss(result.css, context),
    errors,
    js: normalizePaths(result.js ?? '', context).trim(),
    metadata: normalizeData(result.metadata ?? [], context),
    status: 'ok',
    warnings,
  };
}

/**
 * The part of a result that must match byte-for-byte after normalization:
 * transform status, StyleX metadata, generated CSS and diagnostics.
 */
function exactPart(result) {
  const { js: _js, ...rest } = result;
  return rest;
}

/**
 * The generated JavaScript, which is compared semantically rather than
 * exactly. `null` for fixtures whose transform is expected to fail.
 */
function jsPart(result) {
  return result.js ?? null;
}

module.exports = {
  exactPart,
  jsPart,
  normalizeResult,
};
