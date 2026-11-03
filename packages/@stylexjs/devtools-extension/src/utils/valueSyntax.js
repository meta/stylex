/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

export type SyntaxPart = {
  separator: ?string,
  value: string,
};

type SyntaxDepth = {
  curly: number,
  paren: number,
  square: number,
};

type SyntaxVisitor = (
  character: string,
  index: number,
  depth: SyntaxDepth,
) => boolean;

export function walkCssSyntax(
  value: string,
  startIndex: number,
  visit: SyntaxVisitor,
): void {
  let comment = false;
  let curly = 0;
  let escaped = false;
  let paren = 0;
  let quote = null;
  let square = 0;

  for (let index = startIndex; index < value.length; index += 1) {
    const character = value[index];
    const nextCharacter = value[index + 1];

    if (comment) {
      if (character === '*' && nextCharacter === '/') {
        comment = false;
        index += 1;
      }
      continue;
    }
    if (quote != null) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (escaped) {
      escaped = false;
      continue;
    }
    if (character === '\\') {
      escaped = true;
      continue;
    }
    if (character === '/' && nextCharacter === '*') {
      comment = true;
      index += 1;
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
      continue;
    }

    if (character === '(') paren += 1;
    else if (character === ')') paren = Math.max(0, paren - 1);
    else if (character === '[') square += 1;
    else if (character === ']') square = Math.max(0, square - 1);
    else if (character === '{') curly += 1;
    else if (character === '}') curly = Math.max(0, curly - 1);

    if (!visit(character, index, { curly, paren, square })) return;
  }
}

export function findMatchingParenthesis(
  value: string,
  openIndex: number,
): number {
  let closeIndex = -1;
  walkCssSyntax(value, openIndex, (character, index, depth) => {
    if (character === ')' && depth.paren === 0) {
      closeIndex = index;
      return false;
    }
    return true;
  });
  return closeIndex;
}

export function splitTopLevel(
  value: string,
  isSeparator: (character: string, index: number, value: string) => boolean,
): Array<SyntaxPart> {
  const parts: Array<SyntaxPart> = [];
  let partStart = 0;
  let separator: ?string = null;

  walkCssSyntax(value, 0, (character, index, depth) => {
    if (
      depth.paren !== 0 ||
      depth.square !== 0 ||
      depth.curly !== 0 ||
      !isSeparator(character, index, value)
    ) {
      return true;
    }

    parts.push({ separator, value: value.slice(partStart, index).trim() });
    partStart = index + 1;
    separator = character;
    return true;
  });
  parts.push({ separator, value: value.slice(partStart).trim() });
  return parts;
}
