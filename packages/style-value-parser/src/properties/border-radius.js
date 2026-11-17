/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { LengthPercentage } from '../css-types/length-percentage';

import { TokenParser } from '../token-parser';
import { lengthPercentage } from '../css-types/length-percentage';

export class BorderRadiusIndividual {
  horizontal: LengthPercentage;
  vertical: LengthPercentage;

  constructor(horizontal: LengthPercentage, vertical?: LengthPercentage) {
    this.horizontal = horizontal;
    this.vertical = vertical ?? horizontal;
  }

  toString(): string {
    const horizontal = this.horizontal.toString();
    const vertical = this.vertical.toString();
    if (horizontal === vertical) {
      return horizontal;
    }
    return `${horizontal} ${vertical}`;
  }

  static get parse(): TokenParser<BorderRadiusIndividual> {
    return TokenParser.oneOf(
      TokenParser.sequence(lengthPercentage, lengthPercentage).separatedBy(
        TokenParser.tokens.Whitespace,
      ),
      lengthPercentage.map((p) => [p, p]),
    ).map(
      ([horizontal, vertical]) =>
        new BorderRadiusIndividual(horizontal, vertical),
    );
  }
}

export class BorderRadiusShorthand {
  horizontalTopLeft: LengthPercentage;
  horizontalTopRight: LengthPercentage;
  horizontalBottomRight: LengthPercentage;
  horizontalBottomLeft: LengthPercentage;

  verticalTopLeft: LengthPercentage;
  verticalTopRight: LengthPercentage;
  verticalBottomRight: LengthPercentage;
  verticalBottomLeft: LengthPercentage;

  constructor(
    horizontalTopLeft: LengthPercentage,
    horizontalTopRight: LengthPercentage = horizontalTopLeft,
    horizontalBottomRight: LengthPercentage = horizontalTopLeft,
    horizontalBottomLeft: LengthPercentage = horizontalTopRight,
    verticalTopLeft?: LengthPercentage,
    verticalTopRight?: LengthPercentage,
    verticalBottomRight?: LengthPercentage,
    verticalBottomLeft?: LengthPercentage,
  ) {
    this.horizontalTopLeft = horizontalTopLeft;
    this.horizontalTopRight = horizontalTopRight;
    this.horizontalBottomRight = horizontalBottomRight;
    this.horizontalBottomLeft = horizontalBottomLeft;
    if (verticalTopLeft == null) {
      // No vertical radii (no `/`): each corner is circular
      this.verticalTopLeft = horizontalTopLeft;
      this.verticalTopRight = horizontalTopRight;
      this.verticalBottomRight = horizontalBottomRight;
      this.verticalBottomLeft = horizontalBottomLeft;
    } else {
      this.verticalTopLeft = verticalTopLeft;
      this.verticalTopRight = verticalTopRight ?? verticalTopLeft;
      this.verticalBottomRight = verticalBottomRight ?? verticalTopLeft;
      this.verticalBottomLeft = verticalBottomLeft ?? this.verticalTopRight;
    }
  }

  // The shortest possible version of the border-radius
  toString(): string {
    const horizontalTopLeft = this.horizontalTopLeft.toString();
    const horizontalTopRight = this.horizontalTopRight.toString();
    const horizontalBottomRight = this.horizontalBottomRight.toString();
    const horizontalBottomLeft = this.horizontalBottomLeft.toString();

    let pStr = `${horizontalTopLeft} ${horizontalTopRight} ${horizontalBottomRight} ${horizontalBottomLeft}`;

    // All four are the same
    if (
      horizontalTopLeft === horizontalTopRight &&
      horizontalTopRight === horizontalBottomRight &&
      horizontalBottomRight === horizontalBottomLeft
    ) {
      pStr = horizontalTopLeft;
      // TopLeft === BottomRight && TopRight === BottomLeft
    } else if (
      horizontalTopLeft === horizontalBottomRight &&
      horizontalTopRight === horizontalBottomLeft
    ) {
      pStr = `${horizontalTopLeft} ${horizontalTopRight}`;
      // TopRight === BottomLeft
    } else if (horizontalTopRight === horizontalBottomLeft) {
      pStr = `${horizontalTopLeft} ${horizontalTopRight} ${horizontalBottomRight}`;
    }

    const verticalTopLeft = this.verticalTopLeft.toString();
    const verticalTopRight = this.verticalTopRight.toString();
    const verticalBottomRight = this.verticalBottomRight.toString();
    const verticalBottomLeft = this.verticalBottomLeft.toString();

    let sStr = `${verticalTopLeft} ${verticalTopRight} ${verticalBottomRight} ${verticalBottomLeft}`;
    // All three are the same
    if (
      verticalTopLeft === verticalTopRight &&
      verticalTopRight === verticalBottomRight &&
      verticalBottomRight === verticalBottomLeft
    ) {
      sStr = verticalTopLeft;
      // TopLeft === BottomRight && TopRight === BottomLeft
    } else if (
      verticalTopLeft === verticalBottomRight &&
      verticalTopRight === verticalBottomLeft
    ) {
      sStr = `${verticalTopLeft} ${verticalTopRight}`;
      // TopRight === BottomLeft
    } else if (verticalTopRight === verticalBottomLeft) {
      sStr = `${verticalTopLeft} ${verticalTopRight} ${verticalBottomRight}`;
    }

    if (pStr === sStr) {
      return pStr;
    }

    return `${pStr} / ${sStr}`;
  }

  static get parse(): TokenParser<BorderRadiusShorthand> {
    const spaceSeparatedRadii = TokenParser.sequence(
      lengthPercentage,
      lengthPercentage.prefix(TokenParser.tokens.Whitespace).optional,
      lengthPercentage.prefix(TokenParser.tokens.Whitespace).optional,
      lengthPercentage.prefix(TokenParser.tokens.Whitespace).optional,
    ).map(
      ([
        topLeft,
        topRight = topLeft,
        bottomRight = topLeft,
        bottomLeft = topRight,
      ]) => [topLeft, topRight, bottomRight, bottomLeft],
    );

    const assymtricBorder = TokenParser.sequence(
      spaceSeparatedRadii,
      spaceSeparatedRadii,
    )
      .separatedBy(
        TokenParser.tokens.Delim.map((delim) => delim[4].value)
          .where((d) => d === '/')
          .surroundedBy(TokenParser.tokens.Whitespace),
      )
      .map(
        ([pRadii, sRadii = pRadii]) =>
          new BorderRadiusShorthand(...pRadii, ...sRadii),
      );

    return TokenParser.oneOf(
      assymtricBorder,
      spaceSeparatedRadii.map(
        (borders) => new BorderRadiusShorthand(...borders),
      ),
    );
  }
}
