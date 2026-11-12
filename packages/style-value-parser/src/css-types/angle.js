/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { TokenDimension } from '@csstools/css-tokenizer';

import { TokenParser } from '../token-parser';

export class Angle {
  +value: number;
  +unit: string;
  constructor(value: number, unit: this['unit']) {
    this.value = value;
    this.unit = unit;
  }
  toString(): string {
    return `${this.value}${this.unit}`;
  }
  static UNITS: $ReadOnlyArray<string> = ['deg', 'grad', 'rad', 'turn'];
  static get parser(): TokenParser<Angle> {
    const withUnit = TokenParser.tokens.Dimension.map((v) => v[4])
      .where((v: TokenDimension[4]): implies v is TokenDimension[4] =>
        Angle.UNITS.includes(v.unit),
      )
      .map((v) => new Angle(v.value, v.unit));

    return TokenParser.oneOf(
      withUnit,
      TokenParser.tokens.Number.map((v) =>
        v[4].value === 0 ? new Angle(0, 'deg') : null,
      ).where((v) => v != null),
    );
  }
}
