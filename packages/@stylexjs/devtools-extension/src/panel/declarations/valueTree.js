/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { SyntaxPart } from '../../utils/valueSyntax';

import {
  findMatchingParenthesis,
  splitTopLevel,
} from '../../utils/valueSyntax';

export type ValueNode =
  | { type: 'raw', value: string }
  | {
      type: 'function',
      args: $ReadOnlyArray<ValueNode>,
      name: string,
      suffix: string,
    }
  | { type: 'group', value: ValueNode }
  | {
      type: 'operation',
      parts: $ReadOnlyArray<{
        operator: ?string,
        value: ValueNode,
      }>,
    };

type ParsedFunction = {
  args: $ReadOnlyArray<string>,
  name: string,
  suffix: string,
};

const MATH_FUNCTIONS = new Set([
  'abs',
  'acos',
  'asin',
  'atan',
  'atan2',
  'calc',
  'clamp',
  'cos',
  'exp',
  'hypot',
  'log',
  'max',
  'min',
  'mod',
  'pow',
  'rem',
  'round',
  'sign',
  'sin',
  'sqrt',
  'tan',
]);

function raw(value: string): ValueNode {
  return { type: 'raw', value };
}

function parseOuterFunction(value: string): ?ParsedFunction {
  const functionMatch = /^([_a-zA-Z-][_a-zA-Z0-9-]*)\(/.exec(value);
  if (functionMatch == null || functionMatch[1].toLowerCase() === 'url') {
    return null;
  }

  const openIndex = functionMatch[0].length - 1;
  const closeIndex = findMatchingParenthesis(value, openIndex);
  if (closeIndex === -1) return null;

  const rawSuffix = value.slice(closeIndex + 1).trim();
  if (rawSuffix !== '' && !/^!important$/i.test(rawSuffix)) return null;

  const args = splitTopLevel(
    value.slice(openIndex + 1, closeIndex),
    (character) => character === ',',
  ).map(({ value: argument }) => argument);
  if (args.some((argument) => argument === '')) return null;

  return {
    args,
    name: functionMatch[1],
    suffix: rawSuffix === '' ? '' : ` ${rawSuffix}`,
  };
}

function parseOuterGroup(value: string): ?string {
  if (value[0] !== '(') return null;
  const closeIndex = findMatchingParenthesis(value, 0);
  if (closeIndex !== value.length - 1) return null;

  const inner = value.slice(1, -1).trim();
  return inner === '' ? null : inner;
}

function isAdditiveOperator(
  character: string,
  index: number,
  value: string,
): boolean {
  if (character !== '+' && character !== '-') return false;
  return /\s/.test(value[index - 1] ?? '') && /\s/.test(value[index + 1] ?? '');
}

function isMultiplicativeOperator(character: string): boolean {
  return character === '*' || character === '/';
}

function operation(
  parts: $ReadOnlyArray<SyntaxPart>,
  parsePart: (value: string) => ValueNode,
): ?ValueNode {
  if (parts.length < 2 || parts.some(({ value }) => value === '')) return null;

  return {
    type: 'operation',
    parts: parts.map(({ separator, value }) => ({
      operator: separator,
      value: parsePart(value),
    })),
  };
}

function parseFunction(parsed: ParsedFunction): ValueNode {
  const parseArgument = MATH_FUNCTIONS.has(parsed.name.toLowerCase())
    ? parseMathValue
    : parseCssValue;

  return {
    type: 'function',
    args: parsed.args.map(parseArgument),
    name: parsed.name,
    suffix: parsed.suffix,
  };
}

function parseMathAtom(value: string): ValueNode {
  const parsedFunction = parseOuterFunction(value);
  if (parsedFunction != null) return parseFunction(parsedFunction);

  const groupValue = parseOuterGroup(value);
  if (groupValue != null) {
    return { type: 'group', value: parseMathValue(groupValue) };
  }

  return raw(value);
}

function parseMultiplicativeValue(value: string): ValueNode {
  const parsed = operation(
    splitTopLevel(value, isMultiplicativeOperator),
    parseMathAtom,
  );
  return parsed ?? parseMathAtom(value);
}

function parseMathValue(value: string): ValueNode {
  const trimmed = value.trim();
  const parsedFunction = parseOuterFunction(trimmed);
  if (parsedFunction != null) return parseFunction(parsedFunction);

  const groupValue = parseOuterGroup(trimmed);
  if (groupValue != null) {
    return { type: 'group', value: parseMathValue(groupValue) };
  }

  const parsed = operation(
    splitTopLevel(trimmed, isAdditiveOperator),
    parseMultiplicativeValue,
  );
  return parsed ?? parseMultiplicativeValue(trimmed);
}

export function parseCssValue(value: string): ValueNode {
  const trimmed = value.trim();
  const parsedFunction = parseOuterFunction(trimmed);
  return parsedFunction == null ? raw(trimmed) : parseFunction(parsedFunction);
}
