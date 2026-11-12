/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import genFileBasedIdentifier from '../file-based-identifier';

describe('genFileBasedIdentifier', () => {
  test('creates file/export identifiers without a key', () => {
    expect(
      genFileBasedIdentifier({
        fileName: 'tokens.stylex.js',
        exportName: 'vars',
      }),
    ).toBe('tokens.stylex.js//vars');
  });

  test('creates file/export identifiers with a key', () => {
    expect(
      genFileBasedIdentifier({
        fileName: 'tokens.stylex.js',
        exportName: 'vars',
        key: 'foreground',
      }),
    ).toBe('tokens.stylex.js//vars.foreground');
  });
});
