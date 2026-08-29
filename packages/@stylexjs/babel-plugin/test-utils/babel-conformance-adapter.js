/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import { transformFileSync } from '@babel/core';

import stylexPlugin from '../src/index';

/**
 * Adapts the StyleX Babel plugin to the `compiler-conformance` adapter
 * contract. Everything Babel-specific lives here: parser configuration, the
 * shape of the transform result, and capturing diagnostics off the console.
 *
 * See `packages/compiler-conformance/README.md` for the contract itself.
 */

function formatDiagnostic(args) {
  return args
    .map((value) =>
      typeof value === 'string' ? value : JSON.stringify(value, null, 2),
    )
    .join(' ');
}

// The fixture `syntax` list uses the same names as `@babel/parser` plugins.
function toParserOpts(syntax) {
  return { plugins: [...syntax] };
}

function transform(fixture) {
  const warnings = [];
  const errors = [];
  const originalWarn = console.warn;
  const originalError = console.error;

  console.warn = (...args) => {
    warnings.push(formatDiagnostic(args));
  };
  console.error = (...args) => {
    errors.push(formatDiagnostic(args));
  };

  try {
    const result = transformFileSync(fixture.entryPath, {
      babelrc: false,
      configFile: false,
      filename: fixture.entryPath,
      parserOpts: toParserOpts(fixture.syntax),
      plugins: [[stylexPlugin, fixture.pluginOptions]],
    });

    if (result == null) {
      throw new Error(
        `Babel produced no result for fixture "${fixture.name}".`,
      );
    }

    const metadata = result.metadata?.stylex ?? [];

    return {
      css: stylexPlugin.processStylexRules(metadata, fixture.processOptions),
      errors,
      js: result.code,
      metadata,
      status: 'ok',
      warnings,
    };
  } catch (error) {
    return {
      error: {
        message: error instanceof Error ? error.message : String(error),
      },
      errors,
      status: 'error',
      warnings,
    };
  } finally {
    console.warn = originalWarn;
    console.error = originalError;
  }
}

export const babelAdapter = {
  name: '@stylexjs/babel-plugin',
  transform,
};
