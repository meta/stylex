/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { tokenize, TokenType } from '@csstools/css-tokenizer';

// Each offset marks a one-character numeric placeholder for an interpolated
// value. It must form a whole token, unless it is inside string/URL/comment
// text. In particular, a dimension such as `0px` joins the value to a unit.
export function hasValidTokenBoundaries(
  css: string,
  offsets: $ReadOnlyArray<number>,
): boolean {
  const tokens = tokenize({ css });
  return offsets.every((offset) => {
    const token = tokens.find(
      (token) => token[2] <= offset && offset <= token[3],
    );
    return (
      token != null &&
      ((token[2] === offset && token[3] === offset) ||
        token[0] === TokenType.String ||
        token[0] === TokenType.URL ||
        token[0] === TokenType.Comment)
    );
  });
}
