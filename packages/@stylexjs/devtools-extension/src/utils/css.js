/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

export function normalizeCssProperty(property: string): string {
  const trimmed = property.trim();
  return trimmed.startsWith('--') ? trimmed : trimmed.toLowerCase();
}

export function formatCssValue(value: string, important: boolean): string {
  return important ? `${value} !important` : value;
}

export function parseCssValue(value: string): {
  value: string,
  important: boolean,
} {
  const trimmed = value.trim();
  if (!/!\s*important\s*$/i.test(trimmed)) {
    return { value: trimmed, important: false };
  }
  return {
    value: trimmed.replace(/!\s*important\s*$/i, '').trim(),
    important: true,
  };
}
