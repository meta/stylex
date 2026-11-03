/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { TokenParser } from '../token-parser';
import type {
  TokenIdent,
  TokenNumber,
  TokenPercentage,
} from '@csstools/css-tokenizer';

import { TokenType } from '@csstools/css-tokenizer';

// Purposely not exported
// StyleX will not support this value
// export const revertLayer: TokenParser<string> = TokenParser.token<TokenIdent>(
//   TokenType.Ident,
// )
//   .where((v) => v[4].value === 'revert-layer')
//   .map(() => 'revert-layer');

export type CSSWideKeyword = 'inherit' | 'initial' | 'unset' | 'revert';

export const cssWideKeywords: TokenParser<CSSWideKeyword> =
  TokenParser.tokens.Ident.map((v): string => v[4].value).where<CSSWideKeyword>(
    (v): v is CSSWideKeyword =>
      v === 'inherit' || v === 'initial' || v === 'unset' || v === 'revert',
  );

export const inherit: TokenParser<'inherit'> = cssWideKeywords.where<'inherit'>(
  (v): v is 'inherit' => v === 'inherit',
);

export const initial: TokenParser<'initial'> = cssWideKeywords.where<'initial'>(
  (v): v is 'initial' => v === 'initial',
);

export const unset: TokenParser<'unset'> = cssWideKeywords.where<'unset'>(
  (v): v is 'unset' => v === 'unset',
);

export const revert: TokenParser<'revert'> = cssWideKeywords.where<'revert'>(
  (v): v is 'revert' => v === 'revert',
);

export const auto: TokenParser<string> = TokenParser.token<TokenIdent>(
  TokenType.Ident,
)
  .map((v): string => v[4].value)
  .where<string>((v): v is 'auto' => v === 'auto');

export class CssVariable {
  +name: string;
  constructor(name: string) {
    this.name = name;
  }
  toString(): string {
    return `var(${this.name})`;
  }
  static parse: TokenParser<CssVariable> = TokenParser.sequence(
    TokenParser.tokens.Function.map((v): string => v[4].value).where<string>(
      (v): v is 'var' => v === 'var',
    ),
    TokenParser.tokens.Ident.map((v): string => v[4].value).where<string>(
      (v): implies v is string => v.startsWith('--'),
    ),
    TokenParser.tokens.CloseParen,
  ).map(
    ([_, name, __]: $ReadOnly<[mixed, string, mixed]>) => new CssVariable(name),
  );
}

export class Percentage {
  +value: number;
  constructor(value: number) {
    this.value = value;
  }
  toString(): string {
    return `${this.value}%`;
  }
  static get parser(): TokenParser<Percentage> {
    return TokenParser.token<TokenPercentage>(TokenType.Percentage).map(
      (v) => new Percentage(v[4].value),
    );
  }
}

export const numberOrPercentage: TokenParser<number | Percentage> =
  TokenParser.oneOf(
    Percentage.parser,
    TokenParser.token<TokenNumber>(TokenType.Number).map((v) => v[4].value),
  );
