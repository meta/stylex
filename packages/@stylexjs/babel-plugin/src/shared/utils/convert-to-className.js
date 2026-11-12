/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { TRawValue, StyleRule, StyleXOptions } from '../common-types';

import createHash from '../hash';
import dashify from './dashify';
import transformValue from './transform-value';
import { generateCSSRule } from './generate-css-rule';
import { defaultOptions } from './default-options';
import * as messages from '../messages';
import { sortAtRules, sortPseudos } from './rule-utils';

// This function takes a single style rule and transforms it into a CSS rule.
// [color: 'red'] => ['color', 'classname-for-color-red', CSSRULE{ltr, rtl, priority}]
//
// It converts the camelCased style key to a dash-separated key.
// Handles RTL-flipping
// Hashes to get a className
// Returns the final key, className a CSS Rule
export function convertStyleToClassName(
  objEntry: $ReadOnly<[string, TRawValue]>,
  pseudos: $ReadOnlyArray<string>,
  atRules: $ReadOnlyArray<string>,
  constRules: $ReadOnlyArray<string>,
  options: StyleXOptions = defaultOptions,
): StyleRule {
  const { classNamePrefix = 'x' } = options;
  const [key, rawValue] = objEntry;
  const dashedKey = key.startsWith('--') ? key : dashify(key);

  let value: string | $ReadOnlyArray<string> = Array.isArray(rawValue)
    ? rawValue.map((eachValue) => transformValue(key, eachValue, options))
    : transformValue(key, rawValue, options);

  if (
    Array.isArray(value) &&
    value.find((val) => val.startsWith('var(') && val.endsWith(')'))
  ) {
    value = variableFallbacks(value);
  }

  const sortedPseudos = sortPseudos(pseudos ?? []);
  const sortedAtRules = sortAtRules([...atRules, ...constRules]);

  const pseudoHashString = sortedPseudos.join('');
  const atRuleHashString = sortedAtRules.join('');

  // NOTE: 'null' is used to keep existing hashes stable.
  // This should be removed in a future version.
  const modifierHashString = pseudoHashString + atRuleHashString || 'null';
  const valueAsString = Array.isArray(value) ? value.join(', ') : value;
  const stringToHash = dashedKey + valueAsString + modifierHashString;

  // NOTE: '<>' is used to keep existing hashes stable.
  // This should be removed in a future version.
  const className = classNamePrefix + createHash('<>' + stringToHash);

  const cssRules = generateCSSRule(
    className,
    dashedKey,
    value,
    pseudos,
    atRules,
    constRules,
    options,
  );

  return [key, className, cssRules];
}

export default function variableFallbacks(
  values: $ReadOnlyArray<string>,
): $ReadOnlyArray<string> {
  const firstVar = values.findIndex(
    (val) => val.startsWith('var(') && val.endsWith(')'),
  );
  const lastVar = values.findLastIndex(
    (val) => val.startsWith('var(') && val.endsWith(')'),
  );

  const valuesBeforeFirstVar = values.slice(0, firstVar);
  let varValues = values.slice(firstVar, lastVar + 1).reverse();
  const valuesAfterLastVar = values.slice(lastVar + 1);

  if (varValues.find((val) => !val.startsWith('var(') || !val.endsWith(')'))) {
    throw new Error(messages.NON_CONTIGUOUS_VARS);
  }
  varValues = varValues.map((val) => val.slice(4, -1));

  return [
    ...(valuesBeforeFirstVar.length > 0
      ? valuesBeforeFirstVar.map((val) => composeVars(...varValues, val))
      : [composeVars(...varValues)]),
    ...valuesAfterLastVar,
  ];
}

function composeVars(...vars: $ReadOnlyArray<string>): $FlowFixMe {
  const [first, ...rest] = vars;
  if (rest.length > 0) {
    return `var(${first},${composeVars(...rest)})`;
  } else if (first.startsWith('--')) {
    return `var(${first})`;
  } else {
    return first;
  }
}
