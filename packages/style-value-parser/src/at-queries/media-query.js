/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { TokenParser } from '../token-parser';
import type { TokenAtKeyword, TokenDimension } from '@csstools/css-tokenizer';

import { Calc } from '../css-types/calc';
import { MediaQueryErrors } from './messages';

type Fraction = [number, '/', number];
type WordRule = 'color' | 'monochrome' | 'grid' | 'color-index';
type Length = TokenDimension[4];

type MediaRuleValue = number | Length | string | Fraction;

type MediaKeyword = {
  type: 'media-keyword',
  key: 'screen' | 'print' | 'all',
  not: boolean,
  only?: boolean,
};
type MediaWordRule = { type: 'word-rule', keyValue: WordRule };
type MediaRulePair = {
  type: 'pair',
  key: string,
  value: MediaRuleValue,
};
type MediaNotRule = { type: 'not', rule: MediaQueryRule };
type MediaAndRules = { type: 'and', rules: $ReadOnlyArray<MediaQueryRule> };
type MediaOrRules = { type: 'or', rules: $ReadOnlyArray<MediaQueryRule> };

export type MediaQueryRule =
  | MediaKeyword
  | MediaWordRule
  | MediaRulePair
  | MediaNotRule
  | MediaAndRules
  | MediaOrRules;

// helper to adjust the numeric value when no equality sign is present.
function adjustDimension(
  dimension: Length,
  op: string,
  eq: string | void,
  isMaxWidth: boolean = false,
): Length {
  let adjustedValue = dimension.value;
  const epsilon = 0.01;
  if (eq !== '=') {
    if (isMaxWidth) {
      adjustedValue -= epsilon;
    } else {
      adjustedValue += epsilon;
    }
  }
  return { ...dimension, value: adjustedValue };
}

const basicMediaTypeParser: TokenParser<'screen' | 'print' | 'all'> =
  TokenParser.tokens.Ident.map((token) => token[4].value, '.stringValue').where(
    (key) => key === 'screen' || key === 'print' || key === 'all',
  );

// updated to support optional "not" and "only"
const mediaKeywordParser: TokenParser<MediaKeyword> = TokenParser.sequence(
  TokenParser.string('not').optional,
  TokenParser.string('only').optional,
  basicMediaTypeParser,
)
  .separatedBy(TokenParser.tokens.Whitespace)
  .map(([not, only, keyword]) => ({
    type: 'media-keyword',
    key: keyword,
    not: not === 'not',
    only: only === 'only',
  }));

const mediaWordRuleParser: TokenParser<MediaWordRule> =
  TokenParser.tokens.Ident.map((token) => token[4].value, '.stringValue')
    .surroundedBy(TokenParser.tokens.OpenParen, TokenParser.tokens.CloseParen)
    .where(
      (key): implies key is WordRule =>
        key === 'color' ||
        key === 'monochrome' ||
        key === 'grid' ||
        key === 'color-index',
    )
    .map((key) => ({
      type: 'word-rule',
      keyValue: key,
    }));

// modified mediaRuleValueParser to include calc support; this branch uses Calc.parser
// and maps the output to its toString() representation.
const mediaRuleValueParser: TokenParser<MediaRuleValue> = TokenParser.oneOf(
  Calc.parser.map((calc) => calc.toString()),
  TokenParser.tokens.Dimension.map((token) => token[4]),
  TokenParser.tokens.Ident.map((token) => token[4].value),
  TokenParser.sequence(
    TokenParser.tokens.Number.map((token) => token[4].value),
    TokenParser.tokens.Delim.where(
      (token): token is any => token[4].value === '/',
    ).map(() => '/'),
    TokenParser.tokens.Number.map((token) => token[4].value),
  ).separatedBy(TokenParser.tokens.Whitespace.optional),
  TokenParser.tokens.Number.map((token) => token[4].value),
);

const simplePairParser: TokenParser<MediaRulePair> = TokenParser.sequence(
  TokenParser.tokens.OpenParen,
  TokenParser.tokens.Ident.map((token) => token[4].value, '.stringValue'),
  TokenParser.tokens.Colon,
  mediaRuleValueParser,
  TokenParser.tokens.CloseParen,
)
  .separatedBy(TokenParser.tokens.Whitespace.optional)
  .map(([_openParen, key, _colon, value, _closeParen]) => ({
    type: 'pair',
    key,
    value,
  }));

// forward inequality: (width <= 1250px) or (width < 1250px)
const mediaInequalityRuleParser: TokenParser<MediaRulePair> =
  TokenParser.sequence(
    TokenParser.tokens.OpenParen,
    TokenParser.tokens.Ident.map(
      (token) => token[4].value,
      '.stringValue',
    ).where((val) => val === 'width' || val === 'height'),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '>' || val === '<',
    ),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '=',
    ).optional,
    TokenParser.tokens.Dimension.map((token) => token[4]),
    TokenParser.tokens.CloseParen,
  )
    .separatedBy(TokenParser.tokens.Whitespace.optional)
    .map(([_openParen, key, op, eq, dimension, _closeParen]) => {
      // for forward inequality, e.g. (width < 1250px) becomes max-width
      const finalKey = op === '>' ? `min-${key}` : `max-${key}`;
      const isMaxWidth = finalKey.startsWith('max-');
      const adjustedDimension = adjustDimension(dimension, op, eq, isMaxWidth);
      return {
        type: 'pair',
        key: finalKey,
        value: adjustedDimension,
      };
    });

// reversed inequality: (1250px >= width) or (1250px > width)
const mediaInequalityRuleParserReversed: TokenParser<MediaRulePair> =
  TokenParser.sequence(
    TokenParser.tokens.OpenParen,
    TokenParser.tokens.Dimension.map((token) => token[4]),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '>' || val === '<',
    ),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '=',
    ).optional,
    TokenParser.tokens.Ident.map(
      (token) => token[4].value,
      '.stringValue',
    ).where((val) => val === 'width' || val === 'height'),
    TokenParser.tokens.CloseParen,
  )
    .separatedBy(TokenParser.tokens.Whitespace.optional)
    .map(([_openParen, dimension, op, eq, key, _closeParen]) => {
      // reversed inequality: (1250px > width) becomes max-width
      const finalKey = op === '>' ? `max-${key}` : `min-${key}`;
      const isMaxWidth = finalKey.startsWith('max-');
      const adjustedDimension = adjustDimension(dimension, op, eq, isMaxWidth);
      return {
        type: 'pair',
        key: finalKey,
        value: adjustedDimension,
      };
    });

// combine both inequality forms
const combinedInequalityParser: TokenParser<MediaRulePair> = TokenParser.oneOf(
  mediaInequalityRuleParser,
  mediaInequalityRuleParserReversed,
);

const doubleInequalityRuleParser: TokenParser<MediaAndRules> =
  TokenParser.sequence(
    TokenParser.tokens.OpenParen,
    TokenParser.tokens.Dimension.map((token) => token[4]),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '>' || val === '<',
    ),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '=',
    ).optional,
    TokenParser.tokens.Ident.map(
      (token) => token[4].value,
      '.stringValue',
    ).where((val) => val === 'width' || val === 'height'),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '>' || val === '<',
    ),
    TokenParser.tokens.Delim.map((token) => token[4].value).where(
      (val) => val === '=',
    ).optional,
    TokenParser.tokens.Dimension.map((token) => token[4]),
    TokenParser.tokens.CloseParen,
  )
    .separatedBy(TokenParser.tokens.Whitespace.optional)
    .map(([_openParen, lower, op, eq, key, op2, eq2, upper, _closeParen]) => {
      const lowerKey = op === '>' ? `max-${key}` : `min-${key}`;
      const upperKey = op2 === '>' ? `min-${key}` : `max-${key}`;
      const lowerIsMaxWidth = lowerKey.startsWith('max-');
      const upperIsMaxWidth = upperKey.startsWith('max-');
      const lowerValue = adjustDimension(lower, op, eq, lowerIsMaxWidth);
      const upperValue = adjustDimension(upper, op2, eq2, upperIsMaxWidth);
      return {
        type: 'and',
        rules: [
          { type: 'pair', key: lowerKey, value: lowerValue },
          { type: 'pair', key: upperKey, value: upperValue },
        ],
      };
    });

// update mediaAndRulesParser to use lazy notParser reference
const mediaAndRulesParser: TokenParser<MediaAndRules | MediaQueryRule> =
  TokenParser.oneOrMore(
    TokenParser.oneOf(
      mediaKeywordParser,
      () =>
        mediaOrRulesParser.surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        ),
      () =>
        mediaAndRulesParser.surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        ),
      () => notParser, // lazy reference
      doubleInequalityRuleParser,
      combinedInequalityParser,
      simplePairParser,
      mediaWordRuleParser,
    ),
  )
    .separatedBy(
      TokenParser.string('and').surroundedBy(TokenParser.tokens.Whitespace),
    )
    .where(
      <T>(rules: $ReadOnlyArray<T>): implies rules is $ReadOnlyArray<T> =>
        Array.isArray(rules) && rules.length > 1,
    )
    .map((rules) => (rules.length === 1 ? rules[0] : { type: 'and', rules }));

// update mediaOrRulesParser to use lazy notParser reference
const mediaOrRulesParser: TokenParser<MediaOrRules | MediaQueryRule> =
  TokenParser.oneOrMore(
    TokenParser.oneOf(
      mediaKeywordParser,
      () =>
        mediaOrRulesParser.surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        ),
      () =>
        mediaAndRulesParser.surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        ),
      () => notParser, // lazy reference
      doubleInequalityRuleParser,
      combinedInequalityParser,
      simplePairParser,
      mediaWordRuleParser,
    ),
  )
    .separatedBy(
      TokenParser.string('or').surroundedBy(TokenParser.tokens.Whitespace),
    )
    .where(
      <T>(rules: $ReadOnlyArray<T>): implies rules is $ReadOnlyArray<T> =>
        rules.length > 1,
    )
    .map((rules) => (rules.length === 1 ? rules[0] : { type: 'or', rules }));

// forward declaration for notParser to handle nested not expressions
// eslint-disable-next-line prefer-const
let notParser: TokenParser<MediaNotRule>;

// helper that returns a parser for any valid media query rule
const getNormalRuleParser = () =>
  TokenParser.oneOf(
    // new branch: allow a media keyword only if enclosed in parentheses
    () =>
      basicMediaTypeParser
        .surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        )
        .map((keyword) => ({
          type: 'media-keyword',
          key: keyword,
          not: false,
        })),
    mediaAndRulesParser,
    mediaOrRulesParser,
    simplePairParser,
    mediaWordRuleParser,
    () => notParser,
    () =>
      mediaOrRulesParser.surroundedBy(
        TokenParser.tokens.OpenParen,
        TokenParser.tokens.CloseParen,
      ),
    () =>
      mediaAndRulesParser.surroundedBy(
        TokenParser.tokens.OpenParen,
        TokenParser.tokens.CloseParen,
      ),
  ).skip(TokenParser.tokens.Whitespace.optional);

// now define notParser using the helper
notParser = TokenParser.sequence(
  TokenParser.tokens.OpenParen,
  TokenParser.string('not'),
  TokenParser.tokens.Whitespace,
  getNormalRuleParser(),
  TokenParser.tokens.CloseParen,
).map(([_openParen, _not, _space, rule, _closeParen]): MediaNotRule => ({
  type: 'not',
  rule: rule as $FlowFixMe,
}));

function isNumericLength(val: mixed): boolean {
  return (
    typeof val === 'object' &&
    val !== null &&
    !Array.isArray(val) &&
    typeof val.value === 'number' &&
    typeof val.unit === 'string' &&
    (val.type === 'integer' || val.type === 'number')
  );
}

function mergeIntervalsForAnd(
  rules: Array<MediaQueryRule>,
): Array<MediaQueryRule> {
  const epsilon: number = 0.01;
  // Chromium rounds a max-width ending in .99px up to the next whole pixel.
  // Keep the generated range exclusive at integer breakpoints without
  // widening the gap more than necessary.
  const maxWidthEpsilon: number = 0.02;
  const dimensions = ['width', 'height'];
  const intervals: { [dim: string]: Array<[number, number]> } = {
    width: [],
    height: [],
  };

  const units: { [dim: string]: string } = {};

  let hasAnyUnitConflicts = false;

  for (const rule of rules) {
    if (rule.type === 'not' && rule.rule.type === 'and') {
      const inner = rule.rule.rules;
      if (inner.length === 2) {
        const [left, right] = inner;

        const leftBranch = mergeIntervalsForAnd([
          ...rules.filter((r) => r !== rule),
          { type: 'not', rule: left },
        ]);
        const rightBranch = mergeIntervalsForAnd([
          ...rules.filter((r) => r !== rule),
          { type: 'not', rule: right },
        ]);

        return [
          {
            type: 'or',
            rules: [leftBranch, rightBranch]
              .filter((branch) => branch.length > 0)
              .map((branch) =>
                branch.length === 1
                  ? branch[0]
                  : { type: 'and', rules: branch },
              ),
          },
        ];
      }
    }
  }

  for (const rule: MediaQueryRule of rules) {
    for (const dim of dimensions) {
      if (
        rule.type === 'pair' &&
        (rule.key === `min-${dim}` || rule.key === `max-${dim}`) &&
        isNumericLength(rule.value)
      ) {
        const val = rule.value as any;

        if (intervals[dim].length === 0) {
          units[dim] = val.unit;
        } else if (units[dim] !== val.unit) {
          hasAnyUnitConflicts = true;
        }
        intervals[dim].push(
          rule.key === `min-${dim}`
            ? [val.value, Infinity]
            : [-Infinity, val.value],
        );
        break;
      } else if (
        rule.type === 'not' &&
        rule.rule &&
        rule.rule.type === 'pair' &&
        (rule.rule.key === `min-${dim}` || rule.rule.key === `max-${dim}`) &&
        isNumericLength(rule.rule.value)
      ) {
        const val = rule.rule.value as any;
        if (intervals[dim].length === 0) {
          units[dim] = val.unit;
        } else if (units[dim] !== val.unit) {
          hasAnyUnitConflicts = true;
        }
        if (rule.rule.key === `min-${dim}`) {
          intervals[dim].push([
            -Infinity,
            val.value -
              (dim === 'width' && val.unit.toLowerCase() === 'px'
                ? maxWidthEpsilon
                : epsilon),
          ]);
        } else {
          intervals[dim].push([val.value + epsilon, Infinity]);
        }
        break;
      }
    }
    if (
      !(
        (rule.type === 'pair' &&
          (rule.key === 'min-width' ||
            rule.key === 'max-width' ||
            rule.key === 'min-height' ||
            rule.key === 'max-height') &&
          isNumericLength(rule.value)) ||
        (rule.type === 'not' &&
          rule.rule &&
          rule.rule.type === 'pair' &&
          (rule.rule.key === 'min-width' ||
            rule.rule.key === 'max-width' ||
            rule.rule.key === 'min-height' ||
            rule.rule.key === 'max-height') &&
          isNumericLength(rule.rule.value))
      )
    ) {
      return rules;
    }
  }

  const result: Array<MediaQueryRule> = [];

  if (hasAnyUnitConflicts) {
    return rules;
  }

  for (const dim of dimensions) {
    const dimIntervals = intervals[dim];
    if (dimIntervals.length === 0) continue;

    let lower: number = -Infinity;
    let upper: number = Infinity;
    for (const [l, u]: [number, number] of dimIntervals) {
      if (l > lower) lower = l;
      if (u < upper) upper = u;
    }
    if (lower > upper) {
      return [];
    }
    if (lower !== -Infinity) {
      result.push({
        type: 'pair',
        key: `min-${dim}`,
        value: { value: lower, unit: units[dim], type: 'integer' } as any,
      });
    }
    if (upper !== Infinity) {
      result.push({
        type: 'pair',
        key: `max-${dim}`,
        value: { value: upper, unit: units[dim], type: 'integer' } as any,
      });
    }
  }
  return result.length > 0 ? result : rules;
}

function mergeAndSimplifyRanges(
  rules: Array<MediaQueryRule>,
): Array<MediaQueryRule> {
  try {
    return mergeIntervalsForAnd(rules);
  } catch (e) {
    return rules;
  }
}

// StyleX models the `screen | print | all` media types. A media *type* cannot
// be used as a boolean feature (e.g. `(print)` or `(not (print))`) — browsers
// treat that as an unknown feature and the whole query fails to match. The
// helpers below resolve negated/combined media types into a single positive
// anchoring media type so they can be serialized correctly, e.g.
// `(max-width: 900px) and (not print)` becomes `screen and (max-width: 900px)`.
function complementMediaType(key: string): 'screen' | 'print' | null {
  if (key === 'print') return 'screen';
  if (key === 'screen') return 'print';
  return null; // `not all` matches nothing
}

// A query that never matches.
const NEVER_RULE: MediaQueryRule = {
  type: 'media-keyword',
  key: 'all',
  not: true,
};
// A query that always matches.
const ALWAYS_RULE: MediaQueryRule = {
  type: 'media-keyword',
  key: 'all',
  not: false,
};

function isNeverRule(rule: MediaQueryRule): boolean {
  return (
    rule.type === 'media-keyword' && rule.key === 'all' && rule.not === true
  );
}

function isAlwaysRule(rule: MediaQueryRule): boolean {
  return (
    rule.type === 'media-keyword' &&
    rule.key === 'all' &&
    rule.not !== true &&
    rule.only !== true
  );
}

// The positive media type a rule requires, or `undefined` when it does not
// constrain the media type, or `'never'` when it can never match.
function mediaTypeRequirement(
  rule: MediaQueryRule,
): 'screen' | 'print' | 'all' | 'never' | void {
  if (rule.type === 'media-keyword') {
    if (!rule.not) {
      return rule.key;
    }
    const comp = complementMediaType(rule.key);
    return comp == null ? 'never' : comp;
  }
  if (
    rule.type === 'not' &&
    rule.rule.type === 'media-keyword' &&
    !rule.rule.not
  ) {
    const comp = complementMediaType(rule.rule.key);
    return comp == null ? 'never' : comp;
  }
  return undefined;
}

function negateRule(rule: MediaQueryRule): MediaQueryRule {
  switch (rule.type) {
    case 'media-keyword':
      return { ...rule, not: !rule.not };
    case 'not':
      return rule.rule;
    case 'and':
      return { type: 'or', rules: rule.rules.map(negateRule) };
    case 'or':
      return { type: 'and', rules: rule.rules.map(negateRule) };
    default:
      return { type: 'not', rule };
  }
}

function flattenOrRule(rule: MediaQueryRule): Array<MediaQueryRule> {
  return rule.type === 'or' ? [...rule.rules] : [rule];
}

// Whether a subtree references any media type (`screen`/`print`/`all`). When it
// does not, there is nothing to resolve and the AST is left untouched so that
// media-type-free queries (e.g. plain width ranges) keep their existing shape.
function containsMediaKeyword(node: MediaQueryRule): boolean {
  switch (node.type) {
    case 'media-keyword':
      return true;
    case 'not':
      return containsMediaKeyword(node.rule);
    case 'and':
    case 'or':
      return node.rules.some(containsMediaKeyword);
    default:
      return false;
  }
}

// Rewrites an already-normalized media AST so that media types are expressed as
// valid CSS: a single positive anchoring media type per query, distributing
// over `or` branches and dropping branches that can never match.
function resolveMediaTypes(node: MediaQueryRule): MediaQueryRule {
  if (!containsMediaKeyword(node)) {
    return node;
  }
  switch (node.type) {
    case 'or': {
      const rules: Array<MediaQueryRule> = [];
      for (const child of node.rules) {
        for (const r of flattenOrRule(resolveMediaTypes(child))) {
          if (!isNeverRule(r)) {
            rules.push(r);
          }
        }
      }
      if (rules.length === 0) return NEVER_RULE;
      if (rules.some(isAlwaysRule)) return ALWAYS_RULE;
      if (rules.length === 1) return rules[0];
      return { type: 'or', rules };
    }
    case 'and': {
      const resolved = node.rules.map(resolveMediaTypes);

      // Distribute over a nested `or` so each branch anchors its own media type.
      const orIndex = resolved.findIndex((r) => r.type === 'or');
      if (orIndex !== -1) {
        const orNode: $FlowFixMe = resolved[orIndex];
        const rest = resolved.filter((_, i) => i !== orIndex);
        return resolveMediaTypes({
          type: 'or',
          rules: orNode.rules.map((branch) => ({
            type: 'and',
            rules: [branch, ...rest],
          })),
        });
      }

      let mediaType: 'screen' | 'print' | 'all' = 'all';
      let anchorNode: MediaQueryRule | null = null;
      const others: Array<MediaQueryRule> = [];

      for (const rule of resolved) {
        if (rule.type === 'media-keyword' && !rule.not) {
          if (rule.key !== 'all') {
            if (mediaType === 'all') {
              mediaType = rule.key;
              anchorNode = rule;
            } else if (mediaType !== rule.key) {
              return NEVER_RULE;
            } else if (anchorNode == null) {
              anchorNode = rule;
            }
          }
          continue;
        }
        const req = mediaTypeRequirement(rule);
        if (req === undefined) {
          others.push(rule);
          continue;
        }
        if (req === 'never') return NEVER_RULE;
        if (req !== 'all') {
          if (mediaType === 'all') {
            mediaType = req;
          } else if (mediaType !== req) {
            return NEVER_RULE;
          }
        }
      }

      const rules: Array<MediaQueryRule> = [];
      if (mediaType !== 'all') {
        rules.push(
          anchorNode ?? { type: 'media-keyword', key: mediaType, not: false },
        );
      }
      rules.push(...others);
      if (rules.length === 0) return ALWAYS_RULE;
      if (rules.length === 1) return rules[0];
      return { type: 'and', rules };
    }
    case 'not': {
      const inner = resolveMediaTypes(node.rule);
      if (inner.type === 'media-keyword') {
        return resolveMediaTypes({ ...inner, not: !inner.not });
      }
      if (inner.type === 'and' || inner.type === 'or') {
        return resolveMediaTypes(negateRule(inner));
      }
      return { type: 'not', rule: inner };
    }
    default:
      return node;
  }
}

export class MediaQuery {
  queries: MediaQueryRule;
  constructor(queries: MediaQueryRule) {
    this.queries = resolveMediaTypes(MediaQuery.normalize(queries));
  }
  toString(): string {
    return `@media ${this.#toString(this.queries, true)}`;
  }
  #toString(queries: MediaQueryRule, isTopLevel: boolean = false): string {
    switch (queries.type) {
      case 'media-keyword': {
        // A media type is always serialized as a bare keyword. Wrapping it in
        // parentheses (`(screen)`) would make browsers treat it as an unknown
        // boolean feature, so the query would never match.
        const prefix = queries.not ? 'not ' : queries.only ? 'only ' : '';
        return prefix + queries.key;
      }
      case 'word-rule':
        return `(${queries.keyValue})`;
      case 'pair': {
        const { key, value } = queries;

        if (Array.isArray(value)) {
          return `(${key}: ${value[0]} / ${value[2]})`;
        }

        if (typeof value === 'string') {
          return `(${key}: ${value})`;
        }

        if (
          value != null &&
          typeof value === 'object' &&
          typeof (value as any).value === 'number' &&
          typeof (value as any).unit === 'string'
        ) {
          const len = value as Length;
          return `(${key}: ${len.value}${len.unit})`;
        }

        if (value != null && typeof value.toString === 'function') {
          return `(${key}: ${value.toString()})`;
        }

        throw new Error(
          `cannot serialize media-pair value for key "${key}": ${String(value)}`,
        );
      }
      case 'not':
        return queries.rule &&
          (queries.rule.type === 'and' ||
            queries.rule.type === 'or' ||
            queries.rule.type === 'not')
          ? `(not (${this.#toString(queries.rule)}))`
          : `(not ${this.#toString(queries.rule)})`;
      case 'and':
        return queries.rules.map((rule) => this.#toString(rule)).join(' and ');
      case 'or': {
        const validRules = queries.rules.filter(
          (r) => !(r.type === 'or' && r.rules.length === 0),
        );
        if (validRules.length === 0) return 'not all';
        if (validRules.length === 1)
          return this.#toString(validRules[0], isTopLevel);

        const formattedRules = validRules.map((rule) => {
          if (rule.type === 'and' || rule.type === 'or') {
            const ruleString = this.#toString(rule);
            const result = !isTopLevel ? `(${ruleString})` : ruleString;
            return result;
          }
          return this.#toString(rule);
        });

        return isTopLevel
          ? formattedRules.join(', ')
          : formattedRules.join(' or ');
      }

      default:
        return '';
    }
  }

  static normalize(rule: MediaQueryRule): MediaQueryRule {
    switch (rule.type) {
      case 'and': {
        const flattened: Array<MediaQueryRule> = [];
        for (const r of rule.rules) {
          const norm = MediaQuery.normalize(r);
          if (norm.type === 'and') {
            flattened.push(...norm.rules);
          } else {
            flattened.push(norm);
          }
        }
        const merged = mergeAndSimplifyRanges(flattened);
        if (merged.length === 0)
          return { type: 'media-keyword', key: 'all', not: true };
        return { type: 'and', rules: merged };
      }
      case 'or':
        return {
          type: 'or',
          rules: rule.rules.map((r) => MediaQuery.normalize(r)),
        };

      case 'not': {
        const normalizedOperand = MediaQuery.normalize(rule.rule);

        if (
          normalizedOperand.type === 'media-keyword' &&
          normalizedOperand.key === 'all' &&
          normalizedOperand.not
        ) {
          return { type: 'media-keyword', key: 'all', not: false };
        }

        if (normalizedOperand.type === 'not') {
          return MediaQuery.normalize(normalizedOperand.rule);
        }

        return { type: 'not', rule: normalizedOperand };
      }

      default:
        return rule;
    }
  }

  static get parser(): TokenParser<MediaQuery> {
    const leadingNotParser = TokenParser.sequence(
      TokenParser.tokens.Ident.map(
        (token) => token[4].value,
        '.stringValue',
      ).where((key) => key === 'not'),
      TokenParser.oneOf(
        () =>
          mediaOrRulesParser.surroundedBy(
            TokenParser.tokens.OpenParen,
            TokenParser.tokens.CloseParen,
          ),
        () =>
          mediaAndRulesParser.surroundedBy(
            TokenParser.tokens.OpenParen,
            TokenParser.tokens.CloseParen,
          ),
        () => notParser,
        combinedInequalityParser,
        simplePairParser,
        mediaWordRuleParser,
      ),
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([_not, queries]) => ({ type: 'not', rule: queries }));

    const normalRuleParser = TokenParser.oneOf(
      mediaAndRulesParser,
      mediaOrRulesParser,
      mediaKeywordParser,
      () => notParser,
      doubleInequalityRuleParser,
      combinedInequalityParser,
      simplePairParser,
      mediaWordRuleParser,
      () =>
        mediaOrRulesParser.surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        ),
      () =>
        mediaAndRulesParser.surroundedBy(
          TokenParser.tokens.OpenParen,
          TokenParser.tokens.CloseParen,
        ),
    );

    return TokenParser.sequence(
      TokenParser.tokens.AtKeyword.where(
        (token: TokenAtKeyword): implies token is TokenAtKeyword =>
          token[4].value === 'media',
      ),
      TokenParser.oneOrMore(
        TokenParser.oneOf(leadingNotParser, normalRuleParser),
      ).separatedBy(
        TokenParser.tokens.Comma.surroundedBy(
          TokenParser.tokens.Whitespace.optional,
        ),
      ),
    )
      .separatedBy(TokenParser.tokens.Whitespace)
      .map(([_at, querySets]) => {
        const rule =
          querySets.length > 1
            ? { type: 'or', rules: querySets }
            : querySets[0];
        return new MediaQuery(rule as $FlowFixMe as MediaQueryRule);
      });
  }
}

function _hasBalancedParens(str: string): boolean {
  let count = 0;
  for (const char of Array.from(str)) {
    if (char === '(') count++;
    if (char === ')') count--;
    if (count < 0) return false;
  }
  return count === 0;
}

export function validateMediaQuery(input: string): MediaQuery {
  if (!_hasBalancedParens(input)) {
    throw new Error(MediaQueryErrors.UNBALANCED_PARENS);
  }

  try {
    return MediaQuery.parser.parseToEnd(input);
  } catch (err) {
    throw new Error(MediaQueryErrors.SYNTAX_ERROR);
  }
}
