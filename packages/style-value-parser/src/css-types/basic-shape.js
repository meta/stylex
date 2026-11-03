/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { Position } from './position';

// import { BorderRadiusShorthand } from '../properties/border-radius';
import { TokenParser } from '../token-parser';
import { type LengthPercentage, lengthPercentage } from './length-percentage';

class BasicShape {
  toString(): string {
    throw new Error('Not implemented. Use a sub-class instead.');
  }
}

export class Inset extends BasicShape {
  +top: LengthPercentage;
  +right: LengthPercentage;
  +bottom: LengthPercentage;
  +left: LengthPercentage;
  +round: ?LengthPercentage; //?BorderRadiusShorthand;
  constructor(
    top: LengthPercentage,
    right: LengthPercentage,
    bottom: LengthPercentage,
    left: LengthPercentage,
    round: ?LengthPercentage, //?BorderRadiusShorthand,
  ) {
    super();
    this.top = top;
    this.right = right;
    this.bottom = bottom;
    this.left = left;
    this.round = round;
  }
  // Stringify the shortest possible version of the inset
  toString(): string {
    const { top, right, bottom, left } = this;
    const roundStr =
      this.round != null ? ` round ${this.round.toString()}` : '';
    if (top === right && right === bottom && bottom === left && left === top) {
      return `inset(${top.toString()}${roundStr})`;
    }
    if (top === bottom && left === right) {
      return `inset(${top.toString()} ${right.toString()}${roundStr})`;
    }
    if (left === right) {
      return `inset(${top.toString()} ${right.toString()} ${bottom.toString()}${roundStr})`;
    }
    return `inset(${top.toString()} ${right.toString()} ${bottom.toString()} ${left.toString()}${roundStr})`;
  }

  static get parser(): TokenParser<Inset> {
    type Insets = [
      LengthPercentage,
      LengthPercentage,
      LengthPercentage,
      LengthPercentage,
    ];

    const insets: TokenParser<Insets> = TokenParser.sequence(
      lengthPercentage,
      lengthPercentage.optional,
      lengthPercentage.optional,
      lengthPercentage.optional,
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([t, r = t, b = t, l = r]) => [t, r, b, l]);

    const round: TokenParser<LengthPercentage> = TokenParser.sequence(
      TokenParser.string('round'),
      lengthPercentage,
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([, v]) => v);

    const args: TokenParser<[Insets, void | LengthPercentage]> =
      TokenParser.sequence(insets, round.optional).separatedBy(
        TokenParser.tokens.Whitespace,
      );

    return TokenParser.sequence(
      TokenParser.fn('inset'),
      args,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(([_, [[t, r, b, l], round]]) => new Inset(t, r, b, l, round));
  }
}

export type TCircleRadius = LengthPercentage | 'closest-side' | 'farthest-side';
export class Circle extends BasicShape {
  +radius: TCircleRadius;
  +position: ?Position;
  constructor(radius: TCircleRadius, position: ?Position) {
    super();
    this.radius = radius;
    this.position = position;
  }
  toString(): string {
    const { radius, position } = this;
    const positionStr = position != null ? ` at ${position.toString()}` : '';
    return `circle(${radius.toString()}${positionStr})`;
  }
  static get parser(): TokenParser<Circle> {
    const radius: TokenParser<TCircleRadius> = TokenParser.oneOf(
      lengthPercentage,
      TokenParser.string('closest-side'),
      TokenParser.string('farthest-side'),
    );

    const position: TokenParser<Position> = TokenParser.sequence(
      TokenParser.string('at'),
      Position.parser,
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([, v]) => v);

    const args: TokenParser<[TCircleRadius, void | Position]> =
      TokenParser.sequence(radius, position.optional).separatedBy(
        TokenParser.tokens.Whitespace,
      );

    return TokenParser.sequence(
      TokenParser.fn('circle'),
      args,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(([_, [radius, position]]) => new Circle(radius, position));
  }
}

export class Ellipse extends BasicShape {
  +radiusX: TCircleRadius;
  +radiusY: TCircleRadius;
  +position: ?Position;
  constructor(
    radiusX: TCircleRadius,
    radiusY: TCircleRadius,
    position: ?Position,
  ) {
    super();
    this.radiusX = radiusX;
    this.radiusY = radiusY;
    this.position = position;
  }
  toString(): string {
    const { radiusX, radiusY, position } = this;
    const positionStr = position != null ? ` at ${position.toString()}` : '';
    return `ellipse(${radiusX.toString()} ${radiusY.toString()}${positionStr})`;
  }

  static get parser(): TokenParser<Ellipse> {
    const radius: TokenParser<TCircleRadius> = TokenParser.oneOf(
      lengthPercentage,
      TokenParser.string('closest-side'),
      TokenParser.string('farthest-side'),
    );

    const position: TokenParser<Position> = TokenParser.sequence(
      TokenParser.string('at'),
      Position.parser,
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([_at, v]) => v);

    const args = TokenParser.sequence(
      radius,
      radius,
      position.optional,
    ).separatedBy(TokenParser.tokens.Whitespace);

    return TokenParser.sequence(
      TokenParser.fn('ellipse'),
      args,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(
        ([_, [radiusX, radiusY, position]]) =>
          new Ellipse(radiusX, radiusY, position),
      );
  }
}

type FillRule = 'nonzero' | 'evenodd';
const fillRule: TokenParser<FillRule> = TokenParser.oneOf(
  TokenParser.string('nonzero'),
  TokenParser.string('evenodd'),
);

type Point = $ReadOnly<[LengthPercentage, LengthPercentage]>;

export class Polygon extends BasicShape {
  +fillRule: ?FillRule;
  +points: $ReadOnlyArray<Point>;

  constructor(points: this['points'], fillRule: this['fillRule']) {
    super();
    this.points = points;
    this.fillRule = fillRule;
  }
  toString(): string {
    const fillRule = this.fillRule != null ? `${this.fillRule}, ` : '';
    return `polygon(${fillRule}${this.points
      .map(([x, y]) => `${x.toString()} ${y.toString()}`)
      .join(', ')})`;
  }
  static get parser(): TokenParser<Polygon> {
    const point: TokenParser<Point> = TokenParser.sequence(
      lengthPercentage,
      lengthPercentage,
    ).separatedBy(TokenParser.tokens.Whitespace);

    const points = TokenParser.oneOrMore(point)
      .separatedBy(TokenParser.tokens.Comma)
      .separatedBy(TokenParser.tokens.Whitespace.optional);

    const args = TokenParser.sequence(fillRule.optional, points)
      .separatedBy(TokenParser.tokens.Comma)
      .separatedBy(TokenParser.tokens.Whitespace.optional);

    return TokenParser.sequence(
      TokenParser.fn('polygon'),
      args,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(([_, [fillRule, points]]) => new Polygon(points, fillRule));
  }
}

export class Path extends BasicShape {
  +fillRule: ?FillRule;
  +path: string;
  constructor(path: this['path'], fillRule: this['fillRule']) {
    super();
    this.path = path;
    this.fillRule = fillRule;
  }
  toString(): string {
    const fillRule = this.fillRule != null ? `${this.fillRule}, ` : '';
    return `path(${fillRule}"${this.path}")`;
  }
  static get parser(): TokenParser<Path> {
    const args = TokenParser.sequence(
      fillRule.optional,
      TokenParser.tokens.String.map((v) => v[4].value),
    )
      .separatedBy(TokenParser.tokens.Comma)
      .separatedBy(TokenParser.tokens.Whitespace.optional);

    return TokenParser.sequence(
      TokenParser.fn('path'),
      args,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(([_, [fillRule, path]]) => new Path(path, fillRule));
  }
}
