/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

export type CssDeclaration = {
  property: string,
  value: string,
  important: boolean,
};

function readCssomDeclarations(
  style: CSSStyleDeclaration,
): Array<CssDeclaration> {
  const declarations = [];
  for (let index = 0; index < style.length; index += 1) {
    const property = style.item(index);
    if (property === '') continue;
    declarations.push({
      property,
      value: style.getPropertyValue(property).trim(),
      important: style.getPropertyPriority(property) === 'important',
    });
  }
  return declarations;
}

function splitTopLevel(input: string, delimiter: string): Array<string> {
  const parts = [];
  let start = 0;
  let quote = '';
  let parentheses = 0;
  let brackets = 0;
  let braces = 0;

  for (let index = 0; index < input.length; index += 1) {
    const character = input[index];
    const next = input[index + 1];

    if (character === '\\') {
      index += 1;
      continue;
    }
    if (quote !== '') {
      if (character === quote) quote = '';
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
      continue;
    }
    if (character === '/' && next === '*') {
      const commentEnd = input.indexOf('*/', index + 2);
      if (commentEnd === -1) break;
      index = commentEnd + 1;
      continue;
    }

    if (character === '(') parentheses += 1;
    else if (character === ')') parentheses = Math.max(0, parentheses - 1);
    else if (character === '[') brackets += 1;
    else if (character === ']') brackets = Math.max(0, brackets - 1);
    else if (character === '{') braces += 1;
    else if (character === '}') braces = Math.max(0, braces - 1);
    else if (
      character === delimiter &&
      parentheses === 0 &&
      brackets === 0 &&
      braces === 0
    ) {
      parts.push(input.slice(start, index));
      start = index + 1;
    }
  }

  parts.push(input.slice(start));
  return parts;
}

export function parseDeclarationList(cssText: string): Array<CssDeclaration> {
  const declarations: Array<CssDeclaration> = [];
  for (const statement of splitTopLevel(cssText, ';')) {
    const [propertyPart, ...valueParts] = splitTopLevel(statement, ':');
    const property = propertyPart?.trim() ?? '';
    if (property === '' || valueParts.length === 0) continue;

    let value = valueParts.join(':').trim();
    if (value === '') continue;
    const important = /\s*!\s*important\s*$/i.test(value);
    if (important) {
      value = value.replace(/\s*!\s*important\s*$/i, '').trim();
    }
    declarations.push({ property, value, important });
  }
  return declarations;
}

export function readAuthoredDeclarations(
  style: CSSStyleDeclaration,
): Array<CssDeclaration> {
  const declarations = parseDeclarationList(style.cssText);
  return declarations.length > 0 ? declarations : readCssomDeclarations(style);
}
