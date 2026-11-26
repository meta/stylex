/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { InjectableStyle, StyleXOptions } from './common-types';

import createHash from './hash';
import {
  applyMediaQueryOrderToValue,
  collectVarsByAtRule,
  priorityForAtRule,
  wrapWithAtRules,
} from './stylex-vars-utils';
import { isCSSType } from './types';
import { compareStringsWithDefaultFirst } from './utils/compare-strings';
import { defaultOptions } from './utils/default-options';

// It takes an object of variables with their values and the original set of variables to override
// and returns a hashed className with variables overrides.
//
export default function styleXCreateTheme(
  themeVars: { +__varGroupHash__: string, +[string]: string },
  variables: { +[string]: string | { default: string, +[string]: string } },
  options?: StyleXOptions,
): [{ $$css: true, +[string]: string }, { [string]: InjectableStyle }] {
  if (typeof themeVars.__varGroupHash__ !== 'string') {
    throw new Error(
      'Can only override variables theme created with defineVars().',
    );
  }

  const { classNamePrefix, enableMediaQueryOrder } = {
    ...defaultOptions,
    ...options,
  };

  // Create a map of @-rule names and values
  const rulesByAtRule: { [string]: Array<string> } = {};

  // Sort the set of variables to get a consistent unique hash value
  const sortedKeys = Object.keys(variables).sort();

  for (const key of sortedKeys) {
    const rawValue = isCSSType(variables[key])
      ? variables[key].value
      : variables[key];
    const value = enableMediaQueryOrder
      ? applyMediaQueryOrderToValue(rawValue)
      : rawValue;
    const nameHash = themeVars[key].slice(6, -1);
    collectVarsByAtRule(key, { nameHash, value }, rulesByAtRule);
  }

  // Sort @-rules to get a consistent unique hash value
  // But also put "default" first
  const sortedAtRules = Object.keys(rulesByAtRule).sort(
    compareStringsWithDefaultFirst,
  );

  const atRulesStringForHash = sortedAtRules
    .map((atRule) => wrapWithAtRules(rulesByAtRule[atRule].join(''), atRule))
    .join('');

  const overrideClassName = classNamePrefix + createHash(atRulesStringForHash);

  const stylesToInject: { [string]: InjectableStyle } = {};

  for (const atRule of sortedAtRules) {
    const decls = rulesByAtRule[atRule].join('');
    const rule = `.${overrideClassName}, .${overrideClassName}:root{${decls}}`;

    const priority = 0.4 + priorityForAtRule(atRule) / 10;
    const suffix = atRule === 'default' ? '' : `-${createHash(atRule)}`;
    const ltr = atRule === 'default' ? rule : wrapWithAtRules(rule, atRule);

    stylesToInject[overrideClassName + suffix] = {
      ltr: ltr,
      priority: priority,
      rtl: null,
    };
  }

  const themeClass = `${overrideClassName} ${themeVars.__varGroupHash__}`;

  return [
    {
      [themeVars.__varGroupHash__]: themeClass,
      $$css: true,
    },
    stylesToInject,
  ];
}
