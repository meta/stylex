/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { splitTopLevel, walkCssSyntax } = require('../src/utils/valueSyntax');

test('never reports negative delimiter depths for incomplete edits', () => {
  const depths = [];

  walkCssSyntax(')]} value', 0, (character, index, depth) => {
    depths.push(depth);
    return true;
  });

  expect(
    depths.every(
      ({ curly, paren, square }) => curly >= 0 && paren >= 0 && square >= 0,
    ),
  ).toBe(true);
});

test.each(['red), blue', 'red], blue', 'red}, blue'])(
  'recovers top-level separators after an unmatched delimiter in %s',
  (value) => {
    expect(splitTopLevel(value, (character) => character === ',')).toEqual([
      { separator: null, value: value.slice(0, 4) },
      { separator: ',', value: 'blue' },
    ]);
  },
);
