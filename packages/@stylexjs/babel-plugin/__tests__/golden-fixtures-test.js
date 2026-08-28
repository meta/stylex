/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import {
  exactPart,
  getFixtureNames,
  isJsEquivalent,
  jsPart,
  loadFixture,
  normalizeResult,
  readExpected,
  writeExpected,
} from 'compiler-conformance';

import { babelAdapter } from '../test-utils/babel-conformance-adapter';

// Re-records every `expected.json` from the current Babel output. Only use it
// after reviewing the resulting diff.
const UPDATE_GOLDEN = process.env.STYLEX_UPDATE_GOLDEN === '1';

describe('@stylexjs/babel-plugin golden fixtures', () => {
  test.each(getFixtureNames())('%s', (fixtureName) => {
    const fixture = loadFixture(fixtureName);
    const rawResult = babelAdapter.transform(fixture);

    if (UPDATE_GOLDEN) {
      writeExpected(fixtureName, rawResult, fixture);
    }

    const actual = normalizeResult(fixture, rawResult);
    const expected = readExpected(fixtureName, fixture);

    // Transform status, StyleX metadata, generated CSS and diagnostics have to
    // match exactly once normalized.
    expect(exactPart(actual)).toEqual(exactPart(expected));

    // Generated JavaScript is compared as a normalized AST so that an
    // implementation is free to print it differently. When the programs really
    // do differ, assert on the sources to get a readable diff.
    if (!isJsEquivalent(jsPart(expected), jsPart(actual), fixture.syntax)) {
      expect(jsPart(actual)).toBe(jsPart(expected));
    }
  });
});
