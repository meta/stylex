/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { StyleXOptions } from '../common-types';
import { defaultOptions } from '../utils/default-options';

const logicalToPhysical: $ReadOnly<{ [string]: string }> = {
  start: 'left',
  end: 'right',
  'inline-start': 'left',
  'inline-end': 'right',
};

// These properties are kept for a polyfill that is only used with `legacy-expand-shorthands`
const inlinePropertyToLTR: $ReadOnly<{
  [key: string]: ($ReadOnly<[string, string]>) => $ReadOnly<[string, string]>,
}> = {
  'margin-inline-start': ([_k, val]) => ['margin-left', val],
  'margin-inline-end': ([_k, val]) => ['margin-right', val],
  'padding-inline-start': ([_k, val]) => ['padding-left', val],
  'padding-inline-end': ([_k, val]) => ['padding-right', val],
  'border-inline-start': ([_k, val]) => ['border-left', val],
  'border-inline-end': ([_k, val]) => ['border-right', val],
  'border-inline-start-width': ([_k, val]) => ['border-left-width', val],
  'border-inline-end-width': ([_k, val]) => ['border-right-width', val],
  'border-inline-start-color': ([_key, val]) => ['border-left-color', val],
  'border-inline-end-color': ([_key, val]) => ['border-right-color', val],
  'border-inline-start-style': ([_key, val]) => ['border-left-style', val],
  'border-inline-end-style': ([_key, val]) => ['border-right-style', val],
  'inset-inline-start': ([_key, val]) => ['left', val],
  'inset-inline-end': ([_key, val]) => ['right', val],
};

const propertyToLTR: $ReadOnly<{
  [key: string]: ($ReadOnly<[string, string]>) => $ReadOnly<[string, string]>,
}> = {
  'margin-start': ([_key, val]) => ['margin-left', val],
  'margin-end': ([_key, val]) => ['margin-right', val],
  'padding-start': ([_key, val]) => ['padding-left', val],
  'padding-end': ([_key, val]) => ['padding-right', val],
  'border-start': ([_key, val]) => ['border-left', val],
  'border-end': ([_key, val]) => ['border-right', val],
  'border-start-width': ([_key, val]) => ['border-left-width', val],
  'border-end-width': ([_key, val]) => ['border-right-width', val],
  'border-start-color': ([_key, val]) => ['border-left-color', val],
  'border-end-color': ([_key, val]) => ['border-right-color', val],
  'border-start-style': ([_key, val]) => ['border-left-style', val],
  'border-end-style': ([_key, val]) => ['border-right-style', val],
  'border-top-start-radius': ([_key, val]) => ['border-top-left-radius', val],
  'border-bottom-start-radius': ([_k, v]) => ['border-bottom-left-radius', v],
  'border-top-end-radius': ([_key, v]) => ['border-top-right-radius', v],
  'border-bottom-end-radius': ([_k, v]) => ['border-bottom-right-radius', v],
  float: ([key, val]) => [key, logicalToPhysical[val] ?? val],
  clear: ([key, val]) => [key, logicalToPhysical[val] ?? val],
  start: ([_k, val]) => ['left', val],
  // 'inset-inline-start': ([key, val]) => ['left', val],
  end: ([_k, val]) => ['right', val],
  // 'inset-inline-end': ([key, val]) => ['right', val],
  'background-position': ([key, val]) => [
    key,
    val
      .split(' ')
      .map((word) =>
        word === 'start' || word === 'insetInlineStart'
          ? 'left'
          : word === 'end' || word === 'insetInlineEnd'
            ? 'right'
            : word,
      )
      .join(' '),
  ],
};

export default function generateLTR(
  pair: $ReadOnly<[string, string]>,
  options: StyleXOptions = defaultOptions,
): $ReadOnly<[string, string]> {
  const { enableLogicalStylesPolyfill, styleResolution } = options;
  const [key] = pair;

  if (styleResolution === 'legacy-expand-shorthands') {
    if (!enableLogicalStylesPolyfill) {
      return pair;
    }

    if (inlinePropertyToLTR[key]) {
      return inlinePropertyToLTR[key](pair);
    }
  }

  if (!propertyToLTR[key]) {
    return pair;
  }

  return propertyToLTR[key](pair);
}
