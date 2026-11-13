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
 * Turn millisecond values to seconds (shorter), except when < 10ms
 */

export default function normalizeTimings(
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
    if (!dimension || dimension.unit !== 'ms' || value < 10) {
      return;
    }
    // `33.3 / 1000` is 0.033299999999999996 in floating point; 15 significant
    // digits drop that noise without losing any digit a CSS time can carry.
    node.value = Number((value / 1000).toPrecision(15)) + 's';
  });
  return ast;
}
