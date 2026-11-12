/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { TokenHash } from '@csstools/css-tokenizer';

import { TokenParser } from '../token-parser';
import { AlphaValue } from './alpha-value';
import { Angle } from './angle';
import { Percentage } from './common-types';

export class Color {
  static get parser(): TokenParser<Color> {
    return TokenParser.oneOf<Color>(
      NamedColor.parser,
      HashColor.parser,
      Rgb.parser,
      Rgba.parser,
      Hsl.parser,
      Hsla.parser,
      Lch.parser,
      Oklch.parser,
      Oklab.parser,
    );
  }
}

export class NamedColor extends Color {
  +value: string;
  constructor(value: string) {
    super();
    this.value = value;
  }
  toString(): string {
    return this.value;
  }
  static parser: TokenParser<NamedColor> = TokenParser.tokens.Ident.map(
    (token) => token[4].value,
  )
    .where((str: string): implies str is string =>
      [
        'aliceblue',
        'antiquewhite',
        'aqua',
        'aquamarine',
        'azure',
        'beige',
        'bisque',
        'black',
        'blanchedalmond',
        'blue',
        'blueviolet',
        'brown',
        'burlywood',
        'cadetblue',
        'chartreuse',
        'chocolate',
        'coral',
        'cornflowerblue',
        'cornsilk',
        'crimson',
        'cyan',
        'darkblue',
        'darkcyan',
        'darkgoldenrod',
        'darkgray',
        'darkgreen',
        'darkgrey',
        'darkkhaki',
        'darkmagenta',
        'darkolivegreen',
        'darkorange',
        'darkorchid',
        'darkred',
        'darksalmon',
        'darkseagreen',
        'darkslateblue',
        'darkslategray',
        'darkslategrey',
        'darkturquoise',
        'darkviolet',
        'deeppink',
        'deepskyblue',
        'dimgray',
        'dimgrey',
        'dodgerblue',
        'firebrick',
        'floralwhite',
        'forestgreen',
        'fuchsia',
        'gainsboro',
        'ghostwhite',
        'gold',
        'goldenrod',
        'gray',
        'green',
        'greenyellow',
        'grey',
        'honeydew',
        'hotpink',
        'indianred',
        'indigo',
        'ivory',
        'khaki',
        'lavender',
        'lavenderblush',
        'lawngreen',
        'lemonchiffon',
        'lightblue',
        'lightcoral',
        'lightcyan',
        'lightgoldenrodyellow',
        'lightgray',
        'lightgreen',
        'lightgrey',
        'lightpink',
        'lightsalmon',
        'lightseagreen',
        'lightskyblue',
        'lightslategray',
        'lightslategrey',
        'lightsteelblue',
        'lightyellow',
        'lime',
        'limegreen',
        'linen',
        'magenta',
        'maroon',
        'mediumaquamarine',
        'mediumblue',
        'mediumorchid',
        'mediumpurple',
        'mediumseagreen',
        'mediumslateblue',
        'mediumspringgreen',
        'mediumturquoise',
        'mediumvioletred',
        'midnightblue',
        'mintcream',
        'mistyrose',
        'moccasin',
        'navajowhite',
        'navy',
        'oldlace',
        'olive',
        'olivedrab',
        'orange',
        'orangered',
        'orchid',
        'palegoldenrod',
        'palegreen',
        'paleturquoise',
        'palevioletred',
        'papayawhip',
        'peachpuff',
        'peru',
        'pink',
        'plum',
        'powderblue',
        'purple',
        'rebeccapurple',
        'red',
        'rosybrown',
        'royalblue',
        'saddlebrown',
        'salmon',
        'sandybrown',
        'seagreen',
        'seashell',
        'sienna',
        'silver',
        'skyblue',
        'slateblue',
        'slategray',
        'slategrey',
        'snow',
        'springgreen',
        'steelblue',
        'tan',
        'teal',
        'thistle',
        'tomato',
        'transparent',
        'turquoise',
        'violet',
        'wheat',
        'white',
        'whitesmoke',
        'yellow',
        'yellowgreen',
      ].includes(str),
    )
    .map((value) => new NamedColor(value));
}

export class HashColor extends Color {
  +value: string;
  constructor(value: string) {
    super();
    this.value = value;
  }
  toString(): string {
    return `#${this.value}`;
  }

  // Expand 3-digit shorthand (e.g. 'f0a' -> 'ff00aa') so the channel getters
  // can slice fixed 2-char windows. 6- and 8-digit values are returned as-is.
  #expanded(): string {
    return this.value.length === 3
      ? this.value
          .split('')
          .map((c) => c + c)
          .join('')
      : this.value;
  }

  get r(): number {
    return parseInt(this.#expanded().slice(0, 2), 16);
  }

  get g(): number {
    return parseInt(this.#expanded().slice(2, 4), 16);
  }

  get b(): number {
    return parseInt(this.#expanded().slice(4, 6), 16);
  }

  get a(): number {
    const value = this.#expanded();
    return value.length === 8 ? parseInt(value.slice(6, 8), 16) / 255 : 1;
  }

  static get parser(): TokenParser<HashColor> {
    return TokenParser.tokens.Hash.map((token: TokenHash) => token[4].value)
      .where(
        (value: string): implies value is string =>
          [3, 6, 8].includes(value.length) && /^[0-9a-fA-F]+$/.test(value),
      )
      .map((value) => new HashColor(value));
  }
}

const rgbNumberParser = TokenParser.tokens.Number.map(
  (token) => token[4].value,
).where((value): implies value is number => value >= 0 && value <= 255);

const alphaAsNumber = AlphaValue.parser.map((alpha) => alpha.value);

const slashParser = TokenParser.tokens.Delim.map((token) => token[4].value)
  .where((value): implies value is string => value === '/')
  .surroundedBy(TokenParser.tokens.Whitespace);

export class Rgb extends Color {
  +r: number;
  +g: number;
  +b: number;
  constructor(r: number, g: number, b: number) {
    super();
    this.r = r;
    this.g = g;
    this.b = b;
  }
  toString(): string {
    return `rgb(${this.r},${this.g},${this.b})`;
  }
  static get parser(): TokenParser<Rgb> {
    const rgbCommaSeparated = TokenParser.sequence(
      rgbNumberParser,
      rgbNumberParser,
      rgbNumberParser,
    )
      .separatedBy(TokenParser.tokens.Comma)
      .separatedBy(TokenParser.tokens.Whitespace.optional);

    const commaParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'rgb',
      ),
      rgbCommaSeparated,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(([_fn, [r, g, b], _closeParen]) => new Rgb(r, g, b));

    const spaceSeparatedRGB = TokenParser.sequence(
      rgbNumberParser,
      rgbNumberParser,
      rgbNumberParser,
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .surroundedBy(TokenParser.tokens.Whitespace.optional);

    const spaceParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'rgb',
      ),
      spaceSeparatedRGB,
      TokenParser.tokens.CloseParen,
    ).map(([_fn, [r, g, b], _closeParen]) => new Rgb(r, g, b));

    return TokenParser.oneOf(commaParser, spaceParser);
  }
}

export class Rgba extends Color {
  +r: number;
  +g: number;
  +b: number;
  +a: number;
  constructor(r: number, g: number, b: number, a: number) {
    super();
    this.r = r;
    this.g = g;
    this.b = b;
    this.a = a;
  }
  toString(): string {
    return `rgba(${this.r},${this.g},${this.b},${this.a})`;
  }
  static get parser(): TokenParser<Rgba> {
    const commaParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'rgba',
      ),
      rgbNumberParser, // r
      TokenParser.tokens.Comma,
      rgbNumberParser, // g
      TokenParser.tokens.Comma,
      rgbNumberParser, // b
      TokenParser.tokens.Comma,
      alphaAsNumber, // a
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(
        ([_fn, r, _comma, g, _comma2, b, _comma3, a, _closeParen]) =>
          new Rgba(r, g, b, a),
      );

    const spaceParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'rgb',
      ),
      TokenParser.tokens.Whitespace.optional,
      rgbNumberParser, // r
      TokenParser.tokens.Whitespace,
      rgbNumberParser, // g
      TokenParser.tokens.Whitespace,
      rgbNumberParser, // b
      slashParser,
      alphaAsNumber, // a
      TokenParser.tokens.Whitespace.optional,
      TokenParser.tokens.CloseParen,
    ).map(
      ([
        _fn,
        _preSpace,
        r,
        _space,
        g,
        _space2,
        b,
        _slash,
        a,
        _postSpace,
        _closeParen,
      ]) => new Rgba(r, g, b, a),
    );

    return TokenParser.oneOf(commaParser, spaceParser);
  }
}

export class Hsl extends Color {
  +h: Angle;
  +s: Percentage;
  +l: Percentage;
  constructor(h: Angle, s: Percentage, l: Percentage) {
    super();
    this.h = h;
    this.s = s;
    this.l = l;
  }
  toString(): string {
    return `hsl(${this.h.toString()},${this.s.toString()},${this.l.toString()})`;
  }
  static get parser(): TokenParser<Hsl> {
    const commaParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'hsl',
      ),
      Angle.parser, // h
      TokenParser.tokens.Comma,
      Percentage.parser, // s
      TokenParser.tokens.Comma,
      Percentage.parser, // l
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map((tokens) => new Hsl(tokens[1], tokens[3], tokens[5]));

    const spaceParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'hsl',
      ),
      Angle.parser, // h
      TokenParser.tokens.Whitespace,
      Percentage.parser, // s
      TokenParser.tokens.Whitespace,
      Percentage.parser, // l
      TokenParser.tokens.Whitespace,
      TokenParser.tokens.CloseParen,
    ).map((tokens) => new Hsl(tokens[1], tokens[3], tokens[5]));

    return TokenParser.oneOf(commaParser, spaceParser);
  }
}

export class Hsla extends Color {
  +h: Angle;
  +s: Percentage;
  +l: Percentage;
  +a: number;
  constructor(h: Angle, s: Percentage, l: Percentage, a: number) {
    super();
    this.h = h;
    this.s = s;
    this.l = l;
    this.a = a;
  }
  toString(): string {
    return `hsla(${this.h.toString()},${this.s.toString()},${this.l.toString()},${this.a})`;
  }
  static get parser(): TokenParser<Hsla> {
    const commaParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'hsla',
      ),
      Angle.parser,
      TokenParser.tokens.Comma,
      Percentage.parser,
      TokenParser.tokens.Comma,
      Percentage.parser,
      TokenParser.tokens.Comma,
      alphaAsNumber,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(
        ([_fn, h, _comma, s, _comma2, l, _comma3, a, _closeParen]) =>
          new Hsla(h, s, l, a),
      );

    const spaceParser = TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'hsl',
      ),
      Angle.parser, // h
      TokenParser.tokens.Whitespace,
      Percentage.parser, // s
      TokenParser.tokens.Whitespace,
      Percentage.parser, // l
      slashParser,
      alphaAsNumber, // a
      TokenParser.tokens.Whitespace.optional,
      TokenParser.tokens.CloseParen,
    ).map(
      ([_fn, h, _space, s, _space2, l, _slash, a, _postSpace, _closeParen]) =>
        new Hsla(h, s, l, a),
    );

    return TokenParser.oneOf(commaParser, spaceParser);
  }
}

export class Lch extends Color {
  +l: number;
  +c: number;
  +h: Angle | number;
  +alpha: ?number;
  constructor(l: this['l'], c: this['c'], h: this['h'], alpha?: this['alpha']) {
    super();
    this.l = l;
    this.c = c;
    this.h = h;
    this.alpha = alpha;
  }
  toString(): string {
    return `lch(${this.l} ${this.c} ${this.h.toString()}${this.alpha ? ` / ${this.alpha}` : ''})`;
  }
  static get parser(): TokenParser<Lch> {
    const l: TokenParser<number> = TokenParser.oneOf(
      Percentage.parser.map((p) => p.value),
      TokenParser.tokens.Number.map((token) => token[4].value).where(
        (value): implies value is number => value >= 0,
      ),
      TokenParser.tokens.Ident.map((token) => token[4].value)
        .where((value): implies value is string => value === 'none')
        .map(() => 0),
    );

    const c: TokenParser<number> = TokenParser.oneOf(
      // `c` 100% -> 150
      Percentage.parser.map((p) => (150 * p.value) / 100),
      TokenParser.tokens.Number.map((token) => token[4].value).where(
        (value): implies value is number => value >= 0,
      ),
    );

    const h: TokenParser<Angle | number> = TokenParser.oneOf(
      Angle.parser,
      TokenParser.tokens.Number.map((token) => token[4].value),
      // lc.map((num: number) => new Angle(num * 360, 'deg')),
    );

    const a: TokenParser<number> = TokenParser.sequence(
      slashParser,
      alphaAsNumber,
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([_, a]) => a);

    return TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'lch',
      ),
      TokenParser.sequence(
        l,
        c,
        h, // h
      ).separatedBy(TokenParser.tokens.Whitespace),
      a.suffix(TokenParser.tokens.Whitespace.optional).optional,
      TokenParser.tokens.CloseParen,
    ).map(([_fn, [l, c, h], a, _closeParen]) => new Lch(l, c, h, a));
  }
}

export class Oklch extends Color {
  +l: number;
  +c: number;
  +h: Angle;
  +alpha: ?number;
  constructor(l: number, c: number, h: Angle, alpha?: ?number) {
    super();
    this.l = l;
    this.c = c;
    this.h = h;
    this.alpha = alpha;
  }
  toString(): string {
    return `oklch(${this.l} ${this.c} ${this.h.toString()}${this.alpha ? ` / ${this.alpha}` : ''})`;
  }

  static get parser(): TokenParser<Lch> {
    const lc: TokenParser<number> = TokenParser.oneOf(
      alphaAsNumber,
      TokenParser.tokens.Ident.map((token) => token[4].value)
        .where((value): implies value is string => value === 'none')
        .map(() => 0),
    ).prefix(TokenParser.tokens.Whitespace.optional);

    const h: TokenParser<Angle> = TokenParser.oneOf(
      Angle.parser,
      lc.map((num: number) => new Angle(num * 360, 'deg')),
    );

    const a: TokenParser<number> = TokenParser.sequence(
      slashParser,
      alphaAsNumber,
    ).map(([_, a]) => a);

    return TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'oklch',
      ),
      lc, // l
      TokenParser.tokens.Whitespace,
      lc, // c
      TokenParser.tokens.Whitespace,
      h, // h
      a.suffix(TokenParser.tokens.Whitespace.optional).optional,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(
        ([_fn, l, _comma, c, _comma2, h, a, _closeParen]) =>
          new Lch(l, c, h, a),
      );
  }
}

export class Oklab extends Color {
  +l: number;
  +a: number;
  +b: number;
  +alpha: ?number;
  constructor(l: number, a: number, b: number, alpha?: ?number) {
    super();
    this.l = l;
    this.a = a;
    this.b = b;
    this.alpha = alpha;
  }
  toString(): string {
    return `oklab(${this.l} ${this.a} ${this.b}${this.alpha ? ` / ${this.alpha}` : ''})`;
  }
  static get parser(): TokenParser<Oklab> {
    const lab: TokenParser<number> = TokenParser.oneOf(
      alphaAsNumber,
      TokenParser.tokens.Ident.map((token) => token[4].value)
        .where((value): implies value is string => value === 'none')
        .map(() => 0),
    ).prefix(TokenParser.tokens.Whitespace.optional);

    const a: TokenParser<number> = TokenParser.sequence(
      slashParser,
      alphaAsNumber,
    ).map(([_, a]) => a);

    return TokenParser.sequence(
      TokenParser.tokens.Function.map((token) => token[4].value).where(
        (value): implies value is string => value === 'oklab',
      ),
      lab, // l
      TokenParser.tokens.Whitespace,
      lab, // c
      TokenParser.tokens.Whitespace,
      lab, // h
      a.suffix(TokenParser.tokens.Whitespace.optional).optional,
      TokenParser.tokens.CloseParen,
    )
      .separatedBy(TokenParser.tokens.Whitespace.optional)
      .map(
        ([_fn, l, _comma, a, _comma2, b, alpha, _closeParen]) =>
          new Oklab(l, a, b, alpha),
      );
  }
}
