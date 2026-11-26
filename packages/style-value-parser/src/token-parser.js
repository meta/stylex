/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type {
  CSSToken,
  TokenAtKeyword,
  TokenBadString,
  TokenBadURL,
  TokenCDC,
  TokenCDO,
  TokenColon,
  TokenComma,
  TokenComment,
  TokenDelim,
  TokenDimension,
  TokenEOF,
  TokenFunction,
  TokenHash,
  TokenIdent,
  TokenNumber,
  TokenPercentage,
  TokenSemicolon,
  TokenString,
  TokenURL,
  TokenWhitespace,
  TokenOpenParen,
  TokenCloseParen,
  TokenOpenSquare,
  TokenCloseSquare,
  TokenOpenCurly,
  TokenCloseCurly,
  TokenUnicodeRange,
} from '@csstools/css-tokenizer';

import { TokenList } from './token-types';
import { TokenType } from '@csstools/css-tokenizer';

type TokenNameToTokenType = {
  Comment: TokenComment,
  AtKeyword: TokenAtKeyword,
  BadString: TokenBadString,
  BadURL: TokenBadURL,
  CDC: TokenCDC,
  CDO: TokenCDO,
  Colon: TokenColon,
  Comma: TokenComma,
  Delim: TokenDelim,
  Dimension: TokenDimension,
  EOF: TokenEOF,
  Function: TokenFunction,
  Hash: TokenHash,
  Ident: TokenIdent,
  Number: TokenNumber,
  Percentage: TokenPercentage,
  Semicolon: TokenSemicolon,
  String: TokenString,
  URL: TokenURL,
  Whitespace: TokenWhitespace,
  OpenParen: TokenOpenParen,
  CloseParen: TokenCloseParen,
  OpenSquare: TokenOpenSquare,
  CloseSquare: TokenCloseSquare,
  OpenCurly: TokenOpenCurly,
  CloseCurly: TokenCloseCurly,
  UnicodeRange: TokenUnicodeRange,
};

let canSetStackTraceLimit = true;

function parseError(message: string): Error {
  const { stackTraceLimit } = Error;
  if (!canSetStackTraceLimit || typeof stackTraceLimit !== 'number') {
    return new Error(message);
  }
  try {
    Error.stackTraceLimit = 0;
  } catch {
    canSetStackTraceLimit = false;
    return new Error(message);
  }
  try {
    return new Error(message);
  } finally {
    Error.stackTraceLimit = stackTraceLimit;
  }
}

export class TokenParser<+T> {
  +run: (input: TokenList) => T | Error;
  +label: string;

  constructor(
    parser: (input: TokenList) => T | Error,
    label: string = 'UnknownParser',
  ) {
    this.run = parser;
    this.label = label;
  }

  parse(css: string): T | Error {
    const tokens = new TokenList(css);
    return this.run(tokens);
  }

  parseToEnd(css: string): T {
    const tokens = new TokenList(css);
    const initialIndex = tokens.currentIndex;

    const output = this.run(tokens);
    if (output instanceof Error) {
      const consumedTokens = tokens.slice(initialIndex);
      tokens.setCurrentIndex(initialIndex);
      throw new Error(
        `Expected ${this.toString()} but got ${output.message}\n` +
          `Consumed tokens: ${consumedTokens.map((token) => token[0]).join(', ')}`,
      );
    }
    if (tokens.peek() != null) {
      const token = tokens.peek();
      if (token == null) {
        return output;
      }
      const consumedTokens = tokens.slice(initialIndex);
      throw new Error(
        `Expected end of input, got ${token[0]} instead\n` +
          `Consumed tokens: ${consumedTokens.map((token) => token[0]).join(', ')}`,
      );
    }
    return output;
  }

  map<NewT>(f: (value: T) => NewT, label?: string): TokenParser<NewT> {
    return new TokenParser(
      (input): NewT | Error => {
        const currentIndex = input.currentIndex;
        const result = this.run(input);
        if (result instanceof Error) {
          input.setCurrentIndex(currentIndex);
          return result;
        }
        return f(result);
      },
      `${this.label}.map(${label ?? ''})`,
    );
  }

  flatMap<U>(f: (value: T) => TokenParser<U>, label?: string): TokenParser<U> {
    return new TokenParser(
      (input): U | Error => {
        const currentIndex = input.currentIndex;
        const output1 = this.run(input);
        if (output1 instanceof Error) {
          input.setCurrentIndex(currentIndex);
          return output1;
        }
        const secondParser = f(output1);
        const output2: U | Error = secondParser.run(input);
        if (output2 instanceof Error) {
          input.setCurrentIndex(currentIndex);
          return output2;
        }
        return output2;
      },
      `${this.label}.flatMap(${label ?? ''})`,
    );
  }

  or<U>(parser2: TokenParser<U>): TokenParser<T | U> {
    return new TokenParser(
      (input): T | U | Error => {
        const currentIndex = input.currentIndex;
        const output1 = this.run(input);
        if (output1 instanceof Error) {
          input.setCurrentIndex(currentIndex);
          const output2 = parser2.run(input);
          if (output2 instanceof Error) {
            input.setCurrentIndex(currentIndex);
          }
          return output2;
        }
        return output1;
      },
      parser2.label === 'optional'
        ? `Optional<${this.label}>`
        : `OneOf<${this.label}, ${parser2.label}>`,
    );
  }

  surroundedBy(
    prefix: TokenParser<mixed>,
    suffix: TokenParser<mixed> = prefix,
  ): TokenParser<T> {
    return TokenParser.sequence(prefix, this, suffix).map(
      ([_prefix, value, _suffix]) => value,
    );
  }

  skip(skipParser: TokenParser<mixed>): TokenParser<T> {
    return this.flatMap((output) => skipParser.map(() => output));
  }

  get optional(): TokenParser<void | T> {
    return new TokenOptionalParser(this);
  }

  prefix(prefixParser: TokenParser<mixed>): TokenParser<T> {
    return prefixParser.flatMap(() => this);
  }

  suffix(suffixParser: TokenParser<mixed>): TokenParser<T> {
    return this.flatMap((output) => suffixParser.map(() => output));
  }

  // $FlowFixMe[incompatible-variance]
  where<Refined: T = T>(
    predicate: (value: T) => implies value is Refined,
    label?: string = '',
  ): TokenParser<Refined> {
    return this.flatMap((output) => {
      if (predicate(output)) {
        return TokenParser.always(output);
      }
      return TokenParser.never();
    }, label);
  }

  toString(): string {
    return this.label;
  }

  static never<T>(): TokenParser<T> {
    return new TokenParser(() => parseError('Never'), 'Never');
  }

  static always<T>(output: T): TokenParser<T> {
    return new TokenParser(
      () => output,
      output === undefined ? 'optional' : `Always<${String(output)}>`,
    );
  }

  static token<TT: CSSToken>(
    tokenType: TT[0],
    label: string = tokenType,
  ): TokenParser<TT> {
    return new TokenParser((input): TT | Error => {
      const currentIndex = input.currentIndex;
      const token = input.consumeNextToken();
      if (token == null) {
        input.setCurrentIndex(currentIndex);
        return parseError('Expected token');
      }
      if (token[0] !== tokenType) {
        input.setCurrentIndex(currentIndex);
        return parseError(`Expected token type ${tokenType}, got ${token[0]}`);
      }
      // $FlowFixMe[incompatible-type]
      return token as TT;
    }, label);
  }

  static string<S: string>(str: S): TokenParser<S> {
    return TokenParser.tokens.Ident.map(
      (token) => token[4].value,
      '.value',
    ).where(
      // $FlowFixMe[incompatible-type-guard]
      (value: string): implies value is S => value === str,
      `=== ${str}`,
    );
  }

  static fn(name: string): TokenParser<string> {
    return TokenParser.tokens.Function.map(
      (token) => token[4].value,
      '.value',
    ).where((value): implies value is string => value === name, `=== ${name}`);
  }

  static tokens: {
    [Key in keyof typeof TokenType]: TokenParser<TokenNameToTokenType[Key]>,
  } = {
    Comment: TokenParser.token<TokenComment>(TokenType.Comment, 'Comment'),
    AtKeyword: TokenParser.token<TokenAtKeyword>(
      TokenType.AtKeyword,
      'AtKeyword',
    ),
    BadString: TokenParser.token<TokenBadString>(
      TokenType.BadString,
      'BadString',
    ),
    BadURL: TokenParser.token<TokenBadURL>(TokenType.BadURL, 'BadURL'),
    CDC: TokenParser.token<TokenCDC>(TokenType.CDC, 'CDC'),
    CDO: TokenParser.token<TokenCDO>(TokenType.CDO, 'CDO'),
    Colon: TokenParser.token<TokenColon>(TokenType.Colon, 'Colon'),
    Comma: TokenParser.token<TokenComma>(TokenType.Comma, 'Comma'),
    Delim: TokenParser.token<TokenDelim>(TokenType.Delim, 'Delim'),
    Dimension: TokenParser.token<TokenDimension>(
      TokenType.Dimension,
      'Dimension',
    ),
    EOF: TokenParser.token<TokenEOF>(TokenType.EOF, 'EOF'),
    Function: TokenParser.token<TokenFunction>(TokenType.Function, 'Function'),
    Hash: TokenParser.token<TokenHash>(TokenType.Hash, 'Hash'),
    Ident: TokenParser.token<TokenIdent>(TokenType.Ident, 'Ident'),
    Number: TokenParser.token<TokenNumber>(TokenType.Number, 'Number'),
    Percentage: TokenParser.token<TokenPercentage>(
      TokenType.Percentage,
      'Percentage',
    ),
    Semicolon: TokenParser.token<TokenSemicolon>(
      TokenType.Semicolon,
      'Semicolon',
    ),
    String: TokenParser.token<TokenString>(TokenType.String, 'String'),
    URL: TokenParser.token<TokenURL>(TokenType.URL, 'URL'),
    Whitespace: TokenParser.token<TokenWhitespace>(
      TokenType.Whitespace,
      'Whitespace',
    ),
    OpenParen: TokenParser.token<TokenOpenParen>(
      TokenType.OpenParen,
      'OpenParen',
    ),
    CloseParen: TokenParser.token<TokenCloseParen>(
      TokenType.CloseParen,
      'CloseParen',
    ),
    OpenSquare: TokenParser.token<TokenOpenSquare>(
      TokenType.OpenSquare,
      'OpenSquare',
    ),
    CloseSquare: TokenParser.token<TokenCloseSquare>(
      TokenType.CloseSquare,
      'CloseSquare',
    ),
    OpenCurly: TokenParser.token<TokenOpenCurly>(
      TokenType.OpenCurly,
      'OpenCurly',
    ),
    CloseCurly: TokenParser.token<TokenCloseCurly>(
      TokenType.CloseCurly,
      'CloseCurly',
    ),
    UnicodeRange: TokenParser.token<TokenUnicodeRange>(
      TokenType.UnicodeRange,
      'UnicodeRange',
    ),
  };

  // T will be a union of the output types of the parsers
  static oneOf<T>(
    ...parsers: $ReadOnlyArray<TokenParser<T> | (() => TokenParser<T>)>
  ): TokenParser<T> {
    return new TokenParser((input): T | Error => {
      const errors = [];
      const index = input.currentIndex;
      for (const parser of parsers) {
        const output =
          typeof parser === 'function'
            ? parser().run(input)
            : parser.run(input);
        if (!(output instanceof Error)) {
          return output;
        }
        input.setCurrentIndex(index);
        errors.push(output);
      }
      return parseError(
        'No parser matched\n' +
          errors.map((err) => '- ' + err.toString()).join('\n'),
      );
    });
  }

  static sequence<T: ConstrainedTuple<TokenParser<mixed>>>(
    ...parsers: T
  ): TokenParserSequence<T> {
    return new TokenParserSequence<T>(parsers);
  }

  static setOf<T: ConstrainedTuple<TokenParser<mixed>>>(
    ...parsers: T
  ): TokenParserSet<T> {
    return new TokenParserSet<T>(parsers);
  }

  static zeroOrMore<T>(parser: TokenParser<T>): TokenZeroOrMoreParsers<T> {
    return new TokenZeroOrMoreParsers(parser);
  }

  static oneOrMore<T>(parser: TokenParser<T>): TokenOneOrMoreParsers<T> {
    return new TokenOneOrMoreParsers(parser);
  }
}

class TokenZeroOrMoreParsers<+T> extends TokenParser<$ReadOnlyArray<T>> {
  +parser: TokenParser<T>;
  +separator: ?TokenParser<void>;

  constructor(parser: TokenParser<T>, separator?: TokenParser<void>) {
    super((input): $ReadOnlyArray<T> => {
      const output: Array<T> = [];
      for (let i = 0; true; i++) {
        if (i > 0 && separator) {
          const currentIndex = input.currentIndex;
          const result = separator.run(input);
          if (result instanceof Error) {
            input.setCurrentIndex(currentIndex);
            return output;
          }
        }
        const currentIndex = input.currentIndex;
        const result = parser.run(input);
        if (result instanceof Error) {
          input.setCurrentIndex(currentIndex);
          return output;
        }
        output.push(result);
      }
      // eslint-disable-next-line no-unreachable
      return output;
    }, `ZeroOrMore<${parser.label}>`);

    this.parser = parser;
    this.separator = separator;
  }

  separatedBy(separator: TokenParser<mixed>): TokenZeroOrMoreParsers<T> {
    const voidedSeparator = separator.map(() => undefined);
    const newSeparator =
      this.separator?.surroundedBy(voidedSeparator) ?? voidedSeparator;

    return new TokenZeroOrMoreParsers(this.parser, newSeparator);
  }
}

export class TokenOneOrMoreParsers<+T> extends TokenParser<$ReadOnlyArray<T>> {
  +parser: TokenParser<T>;
  +separator: ?TokenParser<void>;

  constructor(parser: TokenParser<T>, separator?: TokenParser<void>) {
    super((input): $ReadOnlyArray<T> | Error => {
      const output: Array<T> = [];
      for (let i = 0; true; i++) {
        if (i > 0 && separator) {
          const currentIndex = input.currentIndex;
          const result = separator.run(input);
          if (result instanceof Error) {
            input.setCurrentIndex(currentIndex);
            return output;
          }
        }
        const currentIndex = input.currentIndex;
        const result = parser.run(input);
        if (result instanceof Error) {
          if (i === 0) {
            input.setCurrentIndex(currentIndex);
            return result;
          }
          return output;
        }
        output.push(result);
      }
      // eslint-disable-next-line no-unreachable
      return output;
    }, `OneOrMore<${parser.label}>`);

    this.parser = parser;
    this.separator = separator;
  }

  separatedBy(separator: TokenParser<mixed>): TokenOneOrMoreParsers<T> {
    const voidedSeparator = separator.map(() => undefined);
    const newSeparator =
      this.separator?.surroundedBy(voidedSeparator) ?? voidedSeparator;

    return new TokenOneOrMoreParsers(this.parser, newSeparator);
  }
}

class TokenParserSequence<
  +T: ConstrainedTuple<TokenParser<mixed>>,
> extends TokenParser<ValuesFromParserTuple<T>> {
  +parsers: T;
  +separator: ?TokenParser<void>;

  constructor(parsers: T, _separator?: TokenParser<mixed>) {
    const separator = _separator?.map(() => undefined);
    super(
      (input: TokenList): ValuesFromParserTuple<T> | Error => {
        const currentIndex = input.currentIndex;
        let failed: Error | null = null;

        // $FlowFixMe[incompatible-type]
        const output: ValuesFromParserTuple<T> | Error = parsers.map(
          <X>(_parser: TokenParser<X>): X | Error => {
            if (failed) {
              return parseError('already failed');
            }
            let parser = _parser;

            if (separator != null && input.currentIndex > currentIndex) {
              if (parser instanceof TokenOptionalParser) {
                // X === void | X
                // $FlowFixMe[incompatible-type]
                parser = TokenParser.sequence(separator, parser.parser).map(
                  ([_separator, value]) => value,
                ).optional;
              } else {
                parser = TokenParser.sequence(separator, parser).map(
                  ([_separator, value]) => value,
                );
              }
            }

            const result = parser.run(input);
            if (result instanceof Error) {
              failed = result;
            }
            return result;
          },
        );

        if (failed) {
          const errorToReturn = failed;
          input.setCurrentIndex(currentIndex);
          return errorToReturn;
        }

        return output;
      },
      `Sequence<${parsers.map((parser) => parser.label).join(', ')}>`,
    );

    this.parsers = parsers;
    this.separator = separator;
  }

  separatedBy(separator: TokenParser<mixed>): TokenParserSequence<T> {
    const newSeparator =
      this.separator?.surroundedBy(separator.map(() => undefined)) ??
      separator.map(() => undefined);

    return new TokenParserSequence(this.parsers, newSeparator);
  }
}

class TokenOptionalParser<+T> extends TokenParser<T | void> {
  +parser: TokenParser<T>;

  constructor(parser: TokenParser<T>) {
    super(
      parser.or(TokenParser.always(undefined)).run,
      `Optional<${parser.label}>`,
    );
    this.parser = parser;
  }
}

class TokenParserSet<
  +T: ConstrainedTuple<TokenParser<mixed>>,
> extends TokenParser<ValuesFromParserTuple<T>> {
  +parsers: T;
  +separator: ?TokenParser<void>;

  constructor(_parsers: T, separator?: ?TokenParser<void>) {
    super((input: TokenList): ValuesFromParserTuple<T> | Error => {
      const parsers = _parsers
        .map((parser, i) => [parser, i])
        .sort(([a], [b]) => {
          if (a instanceof TokenOptionalParser) {
            return 1;
          }
          if (b instanceof TokenOptionalParser) {
            return -1;
          }
          return 0;
        });
      const currentIndex = input.currentIndex;
      let failed: Error | null = null;

      const output: [...ValuesFromParserTuple<T>] = [] as $FlowFixMe;
      const indices: Set<number> = new Set();

      for (let i = 0; i < parsers.length; i++) {
        let found = false;
        const errors = [];
        for (let j = 0; j < parsers.length; j++) {
          if (indices.has(j)) {
            continue;
          }

          // eslint-disable-next-line prefer-const
          let [parser, index] = parsers[j];

          if (separator != null && i > 0) {
            if (parser instanceof TokenOptionalParser) {
              // X === void | X
              // $FlowFixMe[incompatible-type-arg]
              parser = TokenParser.sequence(separator, parser.parser).map(
                ([_separator, value]) => value,
              ).optional;
            } else {
              parser = TokenParser.sequence(separator, parser).map(
                ([_separator, value]) => value,
              );
            }
          }

          const currentIndex = input.currentIndex;
          const result = parser.run(input);
          if (result instanceof Error) {
            input.setCurrentIndex(currentIndex);
            errors.push(result);
          } else {
            found = true;
            // $FlowFixMe[invalid-tuple-index]
            output[index] = result;
            indices.add(j);
            break;
          }
        }
        if (found) {
          continue;
        } else {
          failed = parseError(
            `Expected one of ${parsers
              .map((parser) => parser.toString())
              .join(', ')} but got ${errors
              .map((error) => error.message)
              .join(', ')}`,
          );
          break;
        }
      }

      if (failed instanceof Error) {
        input.setCurrentIndex(currentIndex);
        return failed;
      }

      return output as ValuesFromParserTuple<T>;
    });
    this.parsers = _parsers;
    this.separator = separator;
  }

  separatedBy(separator: TokenParser<mixed>): TokenParserSet<T> {
    const voidedSeparator = separator.map(() => undefined);
    const sep =
      this.separator?.surroundedBy(voidedSeparator) ?? voidedSeparator;

    return new TokenParserSet(this.parsers, sep);
  }
}

type ConstrainedTuple<+T> =
  | $ReadOnly<[T]>
  | $ReadOnly<[T, T]>
  | $ReadOnly<[T, T, T]>
  | $ReadOnly<[T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T, T, T, T, T, T]>
  | $ReadOnly<[T, T, T, T, T, T, T, T, T, T, T, T]>;

// prettier-ignore
export type FromParser<+T: TokenParser<mixed>, Fallback = empty> =
  | Fallback
  | T extends TokenParser<infer V> ? V : empty;

type ValuesFromParserTuple<
  +T: ConstrainedTuple<TokenParser<mixed>>,
  Fallback = empty,
> = {
  [Key in keyof T]: FromParser<T[Key], Fallback>,
};
