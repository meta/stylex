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
import parser from 'postcss-value-parser';

const cursorFlip: $ReadOnly<{ [string]: string }> = {
  'e-resize': 'w-resize',
  'w-resize': 'e-resize',
  'ne-resize': 'nw-resize',
  'nesw-resize': 'nwse-resize',
  'nw-resize': 'ne-resize',
  'nwse-resize': 'nesw-resize',
  'se-resize': 'sw-resize',
  'sw-resize': 'se-resize',
};

function splitByDivisor(value: string) {
  const ast = parser(value);
  const groups = [];

  let currGroup: Array<PostCSSValueASTNode> = [];
  function push() {
    if (currGroup.length === 0) {
      return;
    }

    groups.push(parser.stringify(currGroup));
    currGroup = [];
  }

  for (const node of ast.nodes) {
    if (node.type === 'div') {
      push();
    } else {
      currGroup.push(node);
    }
  }

  push();

  return groups;
}

function flipSign(value: string) {
  if (value === '0') {
    return value;
  } else {
    return value[0] === '-' ? value.slice(1) : '-' + value;
  }
}

function flipShadow(value: string) {
  const defs = splitByDivisor(value);
  const builtDefs = [];

  for (const def of defs) {
    const parts = def.split(' ');
    const index = parser.unit(parts[0]) === false ? 1 : 0;
    if (index < parts.length) {
      parts[index] = flipSign(parts[index]);
    }
    builtDefs.push(parts.join(' '));
  }

  const rtl = builtDefs.join(',');
  if (rtl !== value) {
    return rtl;
  }
}

const logicalToPhysical: $ReadOnly<{ [string]: string }> = {
  start: 'right',
  end: 'left',
  'inline-start': 'right',
  'inline-end': 'left',
};

// These properties are kept for a polyfill that is only used with `legacy-expand-shorthands`
const inlinePropertyToRTL: $ReadOnly<{
  [key: string]: ($ReadOnly<[string, string]>) => $ReadOnly<[string, string]>,
}> = {
  'margin-inline-start': ([_key, val]) => ['margin-right', val],
  'margin-inline-end': ([_key, val]) => ['margin-left', val],
  'padding-inline-start': ([_key, val]) => ['padding-right', val],
  'padding-inline-end': ([_key, val]) => ['padding-left', val],
  'border-inline-start': ([_key, val]) => ['border-right', val],
  'border-inline-end': ([_key, val]) => ['border-left', val],
  'border-inline-start-width': ([_key, val]) => ['border-right-width', val],
  'border-inline-end-width': ([_key, val]) => ['border-left-width', val],
  'border-inline-start-color': ([_key, val]) => ['border-right-color', val],
  'border-inline-end-color': ([_key, val]) => ['border-left-color', val],
  'border-inline-start-style': ([_key, val]) => ['border-right-style', val],
  'border-inline-end-style': ([_key, val]) => ['border-left-style', val],
  'inset-inline-start': ([_key, val]) => ['right', val],
  'inset-inline-end': ([_key, val]) => ['left', val],
};

const propertyToRTL: $ReadOnly<{
  [key: string]: (
    $ReadOnly<[string, string]>,
    options: StyleXOptions,
  ) => $ReadOnly<[string, string]> | null,
}> = {
  'margin-start': ([_key, val]) => ['margin-right', val],
  'margin-end': ([_key, val]) => ['margin-left', val],
  'padding-start': ([_key, val]) => ['padding-right', val],
  'padding-end': ([_key, val]) => ['padding-left', val],
  'border-start': ([_key, val]) => ['border-right', val],
  'border-end': ([_key, val]) => ['border-left', val],
  'border-start-width': ([_key, val]) => ['border-right-width', val],
  'border-end-width': ([_key, val]) => ['border-left-width', val],
  'border-start-color': ([_key, val]) => ['border-right-color', val],
  'border-end-color': ([_key, val]) => ['border-left-color', val],
  'border-start-style': ([_key, val]) => ['border-right-style', val],
  'border-end-style': ([_key, val]) => ['border-left-style', val],
  'border-top-start-radius': ([_key, val]) => ['border-top-right-radius', val],
  'border-bottom-start-radius': ([_k, v]) => ['border-bottom-right-radius', v],
  'border-top-end-radius': ([_key, val]) => ['border-top-left-radius', val],
  'border-bottom-end-radius': ([_k, val]) => ['border-bottom-left-radius', val],

  float: ([key, val]) =>
    logicalToPhysical[val] != null ? [key, logicalToPhysical[val]] : null,

  clear: ([key, val]) =>
    logicalToPhysical[val] != null ? [key, logicalToPhysical[val]] : null,

  start: ([_key, val]) => ['right', val],
  // 'inset-inline-start': ([key, val]) => ['right', val],

  end: ([_key, val]) => ['left', val],
  // 'inset-inline-end': ([key, val]) => ['left', val],

  'background-position': ([key, val]) => {
    const words = val.split(' ');
    if (!words.includes('start') && !words.includes('end')) {
      return null;
    }
    return [
      key,
      words
        .map((word) =>
          word === 'start' || word === 'insetInlineStart'
            ? 'right'
            : word === 'end' || word === 'insetInlineEnd'
              ? 'left'
              : word,
        )
        .join(' '),
    ];
  },

  // Legacy / Incorrect value flipping
  cursor: ([key, val], options = defaultOptions) => {
    if (!options.enableLegacyValueFlipping) {
      return null;
    }
    return cursorFlip[val] != null ? [key, cursorFlip[val]] : null;
  },
  'box-shadow': ([key, val], options = defaultOptions) => {
    if (!options.enableLegacyValueFlipping) {
      return null;
    }
    const rtlVal = flipShadow(val);
    return rtlVal ? [key, rtlVal] : null;
  },
  'text-shadow': ([key, val], options = defaultOptions) => {
    if (!options.enableLegacyValueFlipping) {
      return null;
    }
    const rtlVal = flipShadow(val);
    return rtlVal ? [key, rtlVal] : null;
  },
};

export default function generateRTL(
  pair: $ReadOnly<[string, string]>,
  options: StyleXOptions = defaultOptions,
): ?$ReadOnly<[string, string]> {
  const { enableLogicalStylesPolyfill, styleResolution } = options;
  const [key] = pair;

  if (styleResolution === 'legacy-expand-shorthands') {
    if (!enableLogicalStylesPolyfill) {
      return null;
    }
    if (inlinePropertyToRTL[key]) {
      return inlinePropertyToRTL[key](pair);
    }
  }

  if (!propertyToRTL[key]) {
    return null;
  }

  return propertyToRTL[key](pair, options);
}
