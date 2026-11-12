/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { TokenParser } from '../token-parser';

export class AlphaValue {
  +value: number;
  constructor(value: number) {
    this.value = value;
  }
  toString(): string {
    return this.value.toString();
  }
  static parser: TokenParser<AlphaValue> = TokenParser.oneOf(
    TokenParser.tokens.Percentage.map((v) => new AlphaValue(v[4].value / 100)),
    TokenParser.tokens.Number.map((v) => new AlphaValue(v[4].value)),
  );
}
