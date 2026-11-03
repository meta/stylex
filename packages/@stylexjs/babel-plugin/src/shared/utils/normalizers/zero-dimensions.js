/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import parser from 'postcss-value-parser';

const angles = ['deg', 'grad', 'turn', 'rad'];
const timings = ['ms', 's'];
const fraction = 'fr';
const percentage = '%';

/**
 * Remove units in zero values, except when required: in angles, timings, fractions, and percentages,
 * in which case make them consistent 0deg, 0s, 0fr, and 0%.
 */

export default function normalizeZeroDimensions(
  ast: PostCSSValueAST,
  key: mixed,
): PostCSSValueAST {
  if (typeof key === 'string' && key.startsWith('--')) {
    return ast;
  }

  let endFunction = 0;

  ast.walk((node) => {
    if (node.type === 'function') {
      endFunction = Math.max(endFunction, node.sourceEndIndex ?? 0);
    }
    if (endFunction > 0 && node.sourceIndex > endFunction) {
      endFunction = 0;
    }
    if (node.type === 'function' && node.value.toLowerCase() === 'url') {
      return false;
    }
    if (node.type !== 'word') {
      return;
    }
    const dimension = parser.unit(node.value);
    if (!dimension || dimension.number !== '0') {
      return;
    }
    if (angles.indexOf(dimension.unit) !== -1) {
      node.value = '0deg';
    } else if (timings.indexOf(dimension.unit) !== -1) {
      node.value = '0s';
    } else if (dimension.unit === fraction) {
      node.value = '0fr';
    } else if (dimension.unit === percentage) {
      node.value = '0%';
    } else if (!endFunction) {
      node.value = '0';
    }
  });
  return ast;
}
