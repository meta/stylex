/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import { findCssVariableReferences } from '../src/utils/cssVariables';

test('finds nested variable references and their source ranges', () => {
  const value =
    'calc(var(--size) + var(--space, var(--fallback)) + var(--escaped\\+name))';
  const references = findCssVariableReferences(value);

  expect(references.map(({ name }) => name)).toEqual([
    '--size',
    '--space',
    '--fallback',
    '--escaped\\+name',
  ]);
  expect(references.map(({ start, end }) => value.slice(start, end))).toEqual(
    references.map(({ name }) => name),
  );
});

test('ignores variable-like text in strings, comments, and identifiers', () => {
  const value =
    '"var(--string)" myvar(--identifier) /* var(--comment) */ var(--real)';

  expect(findCssVariableReferences(value).map(({ name }) => name)).toEqual([
    '--real',
  ]);
});
