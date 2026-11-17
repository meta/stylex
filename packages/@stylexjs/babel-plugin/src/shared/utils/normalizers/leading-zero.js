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

/**
 * Remove leading zeros from numbers
 */
export default function normalizeLeadingZero(
  ast: PostCSSValueAST,
  _: mixed,
): PostCSSValueAST {
  ast.walk((node) => {
    if (node.type === 'function' && node.value.toLowerCase() === 'url') {
      return false;
    }
    if (node.type !== 'word') {
      return;
    }
    const value = Number.parseFloat(node.value);
    if (Number.isNaN(value)) {
      return;
    }
    const dimension = parser.unit(node.value);
    if (value < 1 && value >= 0) {
      node.value =
        value.toString().replace('0.', '.') + (dimension ? dimension.unit : '');
    }
  });
  return ast;
}
