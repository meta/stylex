/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { InjectableStyle, StyleXOptions } from './common-types';
import type { VarsConfig, VarsConfigValue } from './stylex-vars-utils';
import { type CSSType, isCSSType } from './types';

import createHash from './hash';
import { objMap } from './utils/object-utils';
import { defaultOptions } from './utils/default-options';
import {
  collectVarsByAtRule,
  getDefaultValue,
  priorityForAtRule,
  wrapWithAtRules,
} from './stylex-vars-utils';

type VarsKeysWithStringValues<Vars: VarsConfig> = $ReadOnly<{
  [$Keys<Vars>]: string,
}>;

type VarsObject<Vars: VarsConfig> = $ReadOnly<{
  ...VarsKeysWithStringValues<Vars>,
  __varGroupHash__: string,
}>;

// Similar to `stylex.create` it takes an object of variables with their values
// and returns a string after hashing it.
export default function styleXDefineVars<Vars: VarsConfig>(
  variables: Vars,
  options: $ReadOnly<{ ...Partial<StyleXOptions>, exportId: string, ... }>,
): [VarsObject<Vars>, { [string]: InjectableStyle }] {
  const { classNamePrefix, exportId } = {
    ...defaultOptions,
    ...options,
  };

  const varGroupHash = classNamePrefix + createHash(exportId);

  const typedVariables: {
    [string]: $ReadOnly<{
      initialValue: ?string,
      syntax: string,
    }>,
  } = {};

  const variablesMap: {
    +[string]: { +nameHash: string, +value: VarsConfigValue },
  } = objMap(variables, (value, key) => {
    const nameHash = key.startsWith('--')
      ? key.slice(2)
      : classNamePrefix + createHash(`${exportId}.${key}`);

    if (isCSSType(value)) {
      const v: CSSType<> = value;
      typedVariables[nameHash] = {
        initialValue: getDefaultValue(v.value),
        syntax: v.syntax,
      };
      return { nameHash, value: v.value };
    }
    return { nameHash, value: value as $FlowFixMe };
  });

  const themeVariablesObject = objMap(
    variablesMap,
    ({ nameHash }) => `var(--${nameHash})`,
  );

  const injectableStyles = constructCssVariablesString(
    variablesMap,
    varGroupHash,
  );

  const injectableTypes: { +[string]: InjectableStyle } = objMap(
    typedVariables,
    ({ initialValue: iv, syntax }, nameHash) => ({
      ltr: `@property --${nameHash} { syntax: "${syntax}"; inherits: true;${iv != null ? ` initial-value: ${iv}` : ''} }`,
      rtl: null,
      priority: 0,
    }),
  );

  return [
    {
      ...themeVariablesObject,
      __varGroupHash__: varGroupHash,
    },
    { ...injectableTypes, ...injectableStyles },
  ];
}

function constructCssVariablesString(
  variables: { +[string]: { +nameHash: string, +value: VarsConfigValue } },
  varGroupHash: string,
): { [string]: InjectableStyle } {
  const rulesByAtRule: { [string]: Array<string> } = {};

  for (const [key, { nameHash, value }] of Object.entries(variables)) {
    collectVarsByAtRule(key, { nameHash, value }, rulesByAtRule);
  }

  const result: { [string]: InjectableStyle } = {};
  for (const [atRule, value] of Object.entries(rulesByAtRule)) {
    const suffix = atRule === 'default' ? '' : `-${createHash(atRule)}`;

    const selector = `:root, .${varGroupHash}`;

    let ltr = `${selector}{${value.join('')}}`;
    if (atRule !== 'default') {
      ltr = wrapWithAtRules(ltr, atRule);
    }

    result[varGroupHash + suffix] = {
      ltr,
      rtl: null,
      priority: priorityForAtRule(atRule) / 10,
    };
  }

  return result;
}
