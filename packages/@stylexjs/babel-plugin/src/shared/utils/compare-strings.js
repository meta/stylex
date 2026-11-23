/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

// Compare UTF-16 code units so generated output does not depend on the locale.
export function compareStrings(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

export function compareStringsWithDefaultFirst(a: string, b: string): number {
  if (a === 'default' && b !== 'default') {
    return -1;
  }
  if (b === 'default' && a !== 'default') {
    return 1;
  }
  return compareStrings(a, b);
}
