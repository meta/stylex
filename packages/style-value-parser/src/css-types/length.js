/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { TokenParser } from '../token-parser';

export const UNITS_BASED_ON_FONT = [
  'em',
  'rem',
  'ex',
  'rex',
  'ch',
  'rch',
  'cap',
  'rcap',
  'ic',
  'ric',
  'lh',
  'rlh',
];

export const UNITS_BASED_ON_VIEWPORT = [
  'vb',
  'vi',
  'svb',
  'svi',
  'lvb',
  'lvi',
  'dvb',
  'dvi',
  'vh',
  'svh',
  'lvh',
  'dvh',
  'vw',
  'svw',
  'lvw',
  'dvw',
  'vmin',
  'svmin',
  'lvmin',
  'dvmin',
  'vmax',
  'svmax',
  'lvmax',
  'dvmax',
];

export const UNITS_BASED_ON_CONTAINER = [
  'cqw',
  'cqi',
  'cqh',
  'cqb',
  'cqmin',
  'cqmax',
];

export const UNITS_BASED_ON_ABSOLUTE_UNITS = [
  'px',
  'cm',
  'mm',
  'in',
  'pt',
  'pc',
  'q',
];
export class Length {
  +value: number;
  +unit: string;

  constructor(value: number, unit: string) {
    this.value = value;
    this.unit = unit;
  }

  toString(): string {
    return `${this.value}${this.unit}`;
  }

  static UNITS: $ReadOnlyArray<string> = [
    ...UNITS_BASED_ON_FONT,
    ...UNITS_BASED_ON_VIEWPORT,
    ...UNITS_BASED_ON_CONTAINER,
    ...UNITS_BASED_ON_ABSOLUTE_UNITS,
  ];

  static get parser(): TokenParser<Length> {
    const united = TokenParser.tokens.Dimension.map(
      (token): $ReadOnly<[number, string]> => [token[4].value, token[4].unit],
    )
      .where(
        (
          tuple: $ReadOnly<[number, string]>,
        ): implies tuple is $ReadOnly<[number, string]> =>
          Length.UNITS.includes(tuple[1].toLowerCase()),
      )
      .map(([value, unit]): Length => new Length(value, unit));

    return TokenParser.oneOf(
      united,
      TokenParser.tokens.Number.map((token): ?Length =>
        token[4].value === 0 ? new Length(0, '') : null,
      ).where((value) => value != null),
    );
  }
}

// Additional classes for other units can be added similarly
