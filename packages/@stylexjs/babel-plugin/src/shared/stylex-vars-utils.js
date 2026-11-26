/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { CSSType } from './types';

import { applyMediaQueryOrder } from './utils/media-query-order';

export type VarsConfigValue =
  | string
  | $ReadOnly<{ default: VarsConfigValue, [string]: VarsConfigValue }>;

export type VarsConfig = $ReadOnly<{
  [string]: VarsConfigValue | CSSType<>,
}>;

const SPLIT_TOKEN = '__$$__';

export function applyMediaQueryOrderToValue(
  value: VarsConfigValue,
): VarsConfigValue {
  return applyMediaQueryOrder({ value }).value;
}

export function collectVarsByAtRule(
  key: string,
  { nameHash, value }: { +nameHash: string, +value: VarsConfigValue },
  collection: { [string]: Array<string> } = {},
  atRules: Array<string> = [],
): void {
  if (typeof value === 'string' || typeof value === 'number') {
    const val = typeof value === 'number' ? value.toString() : value;
    const key =
      atRules.length === 0 ? 'default' : [...atRules].sort().join(SPLIT_TOKEN);
    collection[key] ??= [];
    collection[key].push(`--${nameHash}:${val};`);
    return;
  }
  // $FlowFixMe[invalid-compare]
  if (value === null) {
    return;
  }
  if (Array.isArray(value)) {
    throw new Error('Array is not supported in defineVars');
  }
  if (typeof value === 'object') {
    if (value.default === undefined) {
      throw new Error('Default value is not defined for ' + key + ' variable.');
    }
    for (const atRule of Object.keys(value)) {
      collectVarsByAtRule(
        key,
        { nameHash, value: value[atRule] },
        collection,
        atRule === 'default' ? atRules : [...atRules, atRule],
      );
    }
  }
}

export function wrapWithAtRules(ltr: string, atRule: string): string {
  return atRule
    .split(SPLIT_TOKEN)
    .reduce((acc, atRule) => `${atRule}{${acc}}`, ltr);
  // Wrapper in alphabetic order such that `@supports` wraps `@media`
}

export function priorityForAtRule(atRule: string): number {
  if (atRule === 'default') {
    return 1;
  }
  return 1 + atRule.split(SPLIT_TOKEN).length;
}

export function getDefaultValue(value: VarsConfigValue): ?string {
  if (typeof value === 'string' || typeof value === 'number') {
    return value.toString();
  }
  if (value == null) {
    return null;
  }
  if (Array.isArray(value)) {
    throw new Error('Array is not supported in defineVars');
  }
  if (typeof value === 'object') {
    if (value.default === undefined) {
      throw new Error('Default value is not defined for variable.');
    }
    return getDefaultValue(value.default);
  }
  throw new Error('Invalid value in defineVars');
}
