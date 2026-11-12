/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import * as messages from '../../messages';

/**
 * Detect unclosed strings in stylex property definitions
 */
export default function detectUnclosedStrings(
  ast: PostCSSValueAST,
  _: mixed,
): PostCSSValueAST {
  ast.walk((node) => {
    if (node.type === 'string' && node.unclosed) {
      throw new Error(messages.LINT_UNCLOSED_STRING);
    }
  });
  return ast;
}
