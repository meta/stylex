/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as messages from '../messages';
import { isPlainObject } from '../utils/object-utils';

export function validateNamespace(
  namespace: mixed,
  conditions: $ReadOnlyArray<string> = [],
): void {
  if (!isPlainObject(namespace)) {
    throw new Error(messages.ILLEGAL_NAMESPACE_VALUE);
  }
  const ns: { +[string]: mixed } = namespace;
  for (const key in ns) {
    validateStyleKey(key);
    const val = ns[key];
    if (val === null || typeof val === 'string' || typeof val === 'number') {
      continue;
    }
    if (Array.isArray(val)) {
      for (const v of val) {
        if (v === null || typeof v === 'string' || typeof v === 'number') {
          continue;
        }
        throw new Error(messages.ILLEGAL_PROP_ARRAY_VALUE);
      }
      continue;
    }
    if (isPlainObject(val)) {
      if (key.startsWith('@') || key.startsWith(':') || key.startsWith('[')) {
        if (conditions.includes(key)) {
          throw new Error(messages.DUPLICATE_CONDITIONAL);
        }
        validateNamespace(val, [...conditions, key]);
      } else {
        validateConditionalStyles(val as $FlowFixMe);
      }
      continue;
    }

    throw new Error(messages.ILLEGAL_PROP_VALUE);
  }
}

type ConditionalStyles = $ReadOnly<{
  [key: string]: string | number | ConditionalStyles,
}>;

function validateConditionalStyles(
  val: ConditionalStyles,
  conditions: $ReadOnlyArray<string> = [],
): void {
  for (const key in val) {
    validateStyleKey(key);
    const v = val[key];
    if (
      !(
        key.startsWith('@') ||
        key.startsWith(':') ||
        key.startsWith('[') ||
        // This is a placeholder for `defineConsts` values that are later inlined
        key.startsWith('var(--') ||
        key === 'default'
      )
    ) {
      throw new Error(messages.INVALID_PSEUDO_OR_AT_RULE);
    }
    if (conditions.includes(key)) {
      throw new Error(messages.DUPLICATE_CONDITIONAL);
    }
    // $FlowFixMe[invalid-compare]
    if (v === null || typeof v === 'string' || typeof v === 'number') {
      continue;
    }
    if (Array.isArray(v)) {
      for (const vv of v) {
        // $FlowFixMe[invalid-compare]
        if (vv === null || typeof vv === 'string' || typeof vv === 'number') {
          continue;
        }
        throw new Error(messages.ILLEGAL_PROP_VALUE);
      }
      continue;
    }
    if (isPlainObject(v)) {
      validateConditionalStyles(v, [...conditions, key]);
      continue;
    }
    throw new Error(messages.ILLEGAL_PROP_VALUE);
  }
}

// This policy applies only to keys consumed as CSS, never to intermediate
// JavaScript objects evaluated while constructing a style value.
export function validateStyleKey(key: string): void {
  if (key.startsWith('calc(')) {
    throw new Error(messages.INVALID_CALC_KEY);
  }
}
