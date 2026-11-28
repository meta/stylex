/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { TokenParser } from '../token-parser';
import { Length } from './length';
import { numberOrPercentage } from './common-types';
// import { Color } from './color';
import { Angle } from './angle';

export class FilterFunction {
  toString(): string {
    return '';
  }
  static get parser(): TokenParser<FilterFunction> {
    return TokenParser.oneOf(
      BlurFilterFunction.parser,
      BrightnessFilterFunction.parser,
      ContrastFilterFunction.parser,
      // DropShadowFilterFunction.parser,
      GrayscaleFilterFunction.parser,
      HueRotateFilterFunction.parser,
      InverFilterFunction.parser,
      OpacityFilterFunction.parser,
      SaturateFilterFunction.parser,
      SepiaFilterFunction.parser,
    );
  }
}

export class BlurFilterFunction extends FilterFunction {
  +radius: Length;
  constructor(radius: Length) {
    super();
    this.radius = radius;
  }
  toString(): string {
    return `blur(${this.radius.toString()})`;
  }
  static get parser(): TokenParser<BlurFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('blur'),
      Length.parser.surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, radius, _1]) => new BlurFilterFunction(radius));
  }
}

export class BrightnessFilterFunction extends FilterFunction {
  +percentage: number;
  constructor(percentage: number) {
    super();
    this.percentage = percentage;
  }
  toString(): string {
    return `brightness(${this.percentage})`;
  }
  static get parser(): TokenParser<BrightnessFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('brightness'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, percentage, _1]) => new BrightnessFilterFunction(percentage));
  }
}

export class ContrastFilterFunction extends FilterFunction {
  +amount: number;
  constructor(amount: number) {
    super();
    this.amount = amount;
  }
  toString(): string {
    return `contrast(${this.amount})`;
  }
  static get parser(): TokenParser<ContrastFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('contrast'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, amount, _1]) => new ContrastFilterFunction(amount));
  }
}

// export class DropShadowFilterFunction extends FilterFunction {
//   +offsetX: Length;
//   +offsetY: Length;
//   +blurRadius: Length;
//   +color: ?Color;
//   constructor(
//     offsetX: Length,
//     offsetY: Length,
//     blurRadius: Length,
//     color: ?Color,
//   ) {
//     super();
//     this.offsetX = offsetX;
//     this.offsetY = offsetY;
//     this.blurRadius = blurRadius;
//     this.color = color;
//   }
//   toString(): string {
//     const args = [
//       this.offsetX.toString(),
//       this.offsetY.toString(),
//       this.blurRadius.value !== 0 ? this.blurRadius.toString() : null,
//       this.color?.toString(),
//     ]
//       .filter(Boolean)
//       .join(' ');
//     return `drop-shadow(${args})`;
//   }
//   static get parser(): TokenParser<DropShadowFilterFunction> {
//     return TokenParser.sequence(
//       TokenParser.tokens.Function,
//       TokenParser.sequence(
//         Length.parser,
//         Length.parser,
//         Length.parser.optional,
//         Color.parser.optional,
//       )
//         .separatedBy(TokenParser.tokens.Whitespace)
//         .surroundedBy(TokenParser.tokens.Whitespace.optional),
//       TokenParser.tokens.CloseParen,
//     ).map(
//       ([_, [offsetX, offsetY, blurRadius = new Length(0), color], _1]) =>
//         new DropShadowFilterFunction(offsetX, offsetY, blurRadius, color),
//     );
//   }
// }

export class GrayscaleFilterFunction extends FilterFunction {
  +amount: number;
  constructor(amount: number) {
    super();
    this.amount = amount;
  }
  toString(): string {
    return `grayscale(${this.amount})`;
  }
  static get parser(): TokenParser<GrayscaleFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('grayscale'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, amount, _1]) => new GrayscaleFilterFunction(amount));
  }
}

export class HueRotateFilterFunction extends FilterFunction {
  +angle: Angle;
  constructor(angle: Angle) {
    super();
    this.angle = angle;
  }
  toString(): string {
    return `hue-rotate(${this.angle.toString()})`;
  }
  static get parser(): TokenParser<HueRotateFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('hue-rotate'),
      Angle.parser,
      TokenParser.tokens.CloseParen,
    ).map(([_, angle, _1]) => new HueRotateFilterFunction(angle));
  }
}

export class InverFilterFunction extends FilterFunction {
  +amount: number;
  constructor(amount: number) {
    super();
    this.amount = amount;
  }
  toString(): string {
    return `invert(${this.amount})`;
  }
  static get parser(): TokenParser<InverFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('invert'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, amount, _1]) => new InverFilterFunction(amount));
  }
}

export class OpacityFilterFunction extends FilterFunction {
  +amount: number;
  constructor(amount: number) {
    super();
    this.amount = amount;
  }
  toString(): string {
    return `opacity(${this.amount})`;
  }
  static get parser(): TokenParser<OpacityFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('opacity'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, amount, _1]) => new OpacityFilterFunction(amount));
  }
}

export class SaturateFilterFunction extends FilterFunction {
  +amount: number;
  constructor(amount: number) {
    super();
    this.amount = amount;
  }
  toString(): string {
    return `saturate(${this.amount})`;
  }
  static get parser(): TokenParser<SaturateFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('saturate'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, amount, _1]) => new SaturateFilterFunction(amount));
  }
}

export class SepiaFilterFunction extends FilterFunction {
  +amount: number;
  constructor(amount: number) {
    super();
    this.amount = amount;
  }
  toString(): string {
    return `sepia(${this.amount})`;
  }
  static get parser(): TokenParser<SepiaFilterFunction> {
    return TokenParser.sequence(
      TokenParser.fn('sepia'),
      numberOrPercentage
        .map((p) => (typeof p === 'number' ? p : p.value / 100))
        .surroundedBy(TokenParser.tokens.Whitespace.optional),
      TokenParser.tokens.CloseParen,
    ).map(([_, amount, _1]) => new SepiaFilterFunction(amount));
  }
}
