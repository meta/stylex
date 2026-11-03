/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { InjectableConstStyle, StyleXOptions } from './common-types';
import type { ConstsConfig } from './stylex-consts-utils';

import { defaultOptions } from './utils/default-options';
import createHash from './hash';

export default function styleXDefineConsts<Vars: ConstsConfig>(
  constants: Vars,
  options: $ReadOnly<{ ...Partial<StyleXOptions>, exportId: string, ... }>,
): [
  { [string]: string | number }, // jsOutput JS output
  { [string]: InjectableConstStyle }, // metadata for registerinjectableStyles
] {
  const { classNamePrefix, exportId } = {
    ...defaultOptions,
    ...options,
  };

  const jsOutput: { [string]: string | number } = {};
  const injectableStyles: { [string]: InjectableConstStyle } = {};

  for (const [key, value] of Object.entries(constants)) {
    const constKey = key.startsWith('--')
      ? key.slice(2)
      : `${classNamePrefix}${createHash(`${exportId}.${key}`)}`;

    jsOutput[key] = value;
    injectableStyles[constKey] = {
      constKey,
      constVal: value,
      priority: 0,
      ltr: '',
      rtl: null,
    };
  }

  return [jsOutput, injectableStyles];
}
