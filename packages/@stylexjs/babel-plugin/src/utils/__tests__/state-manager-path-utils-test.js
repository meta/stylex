/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import path from 'node:path';
import {
  filePathResolver,
  getRelativePath,
  matchesFileSuffix,
} from '../state-manager';

const fixtureDir = path.join(
  __dirname,
  '../../../__tests__/__fixtures__/golden/rewrite-theme-extension',
);
const entryFile = path.join(fixtureDir, 'input.js');

describe('state-manager path helpers', () => {
  test('resolves a relative theme import with and without an extension', () => {
    expect(filePathResolver('./tokens.stylex', entryFile, null)).toBe(
      path.join(fixtureDir, 'tokens.stylex.js'),
    );
    expect(filePathResolver('./tokens.stylex.js', entryFile, null)).toBe(
      path.join(fixtureDir, 'tokens.stylex.js'),
    );
  });

  test('resolves aliased theme imports', () => {
    expect(
      filePathResolver('#fixtures/tokens.stylex', entryFile, {
        '#fixtures/*': ['./*'],
      }),
    ).toBe(path.join(fixtureDir, 'tokens.stylex.js'));
  });

  test('matches theme suffixes and rewrites relative paths consistently', () => {
    expect(matchesFileSuffix('.stylex')('./tokens.stylex')).toBe(true);
    expect(matchesFileSuffix('.stylex')('./tokens.stylex.js')).toBe(true);
    expect(matchesFileSuffix('.stylex')('./tokens.js')).toBe(false);
    expect(
      getRelativePath(entryFile, path.join(fixtureDir, 'tokens.stylex.js')),
    ).toBe('./tokens.stylex.js');
  });
});
