/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { parseCssValue } = require('../src/utils/css');

test.each([
  'red !important',
  'red!important',
  'red ! important',
  'red! IMPORTANT',
])('parses the CSS priority in %s', (value) => {
  expect(parseCssValue(value)).toEqual({ value: 'red', important: true });
});

test('does not treat important text inside a value as CSS priority', () => {
  expect(parseCssValue('var(--important)')).toEqual({
    value: 'var(--important)',
    important: false,
  });
});
