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

function assertDiagnosticList(result, field) {
  const value = result[field];
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    throw new TypeError(`Adapter result.${field} must be an array of strings.`);
  }
}

function validateResult(result) {
  if (result == null || typeof result !== 'object' || Array.isArray(result)) {
    throw new TypeError('Adapter result must be an object.');
  }
  if (result.status !== 'ok' && result.status !== 'error') {
    throw new TypeError('Adapter result.status must be "ok" or "error".');
  }

  assertDiagnosticList(result, 'warnings');
  assertDiagnosticList(result, 'errors');

  if (result.status === 'ok') {
    if (typeof result.js !== 'string') {
      throw new TypeError('Successful adapter result.js must be a string.');
    }
    if (!Array.isArray(result.metadata)) {
      throw new TypeError(
        'Successful adapter result.metadata must be an array.',
      );
    }
    if (typeof result.css !== 'string') {
      throw new TypeError('Successful adapter result.css must be a string.');
    }
  } else if (
    result.error == null ||
    typeof result.error !== 'object' ||
    Array.isArray(result.error) ||
    typeof result.error.message !== 'string'
  ) {
    throw new TypeError(
      'Failed adapter result.error.message must be a string.',
    );
  }
}

/**
 * Shapes an adapter result — or a previously recorded `expected.json` — into
 * the canonical form the two are compared in. Both sides go through this
 * function so that recorded fixtures and live output are normalized
 * identically.
 *
 * See `README.md` for the adapter contract and the fixture schema.
 */
function normalizeResult(fixture, result, implementationTag) {
  validateResult(result);

  const context = {
    entry: fixture.entry,
    fixtureDir: fixture.dir,
    implementationTag,
    repoRoot: fixture.repoRoot,
  };
  const toDiagnostics = (values) =>
    values.map((value) => normalizeDiagnostic(value, context));

  const warnings = toDiagnostics(result.warnings);
  const errors = toDiagnostics(result.errors);

  if (result.status === 'error') {
    return {
      error: {
        message: normalizeDiagnostic(result.error.message, context),
      },
      errors,
      status: 'error',
      warnings,
    };
  }

  return {
    css: normalizeCss(result.css, context),
    errors,
    js: normalizePaths(result.js, context).trim(),
    metadata: normalizeData(result.metadata, context),
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
