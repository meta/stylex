/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import {
  findMatchingParenthesis,
  splitTopLevel,
  walkCssSyntax,
} from './valueSyntax';

export type CssVariableReference = {
  end: number,
  name: string,
  start: number,
};

const CUSTOM_PROPERTY_NAME = /^--(?:[-_a-zA-Z0-9]|\\.)+$/;
const IDENTIFIER_CHARACTER = /[-_a-zA-Z0-9\\]/;

export function findCssVariableReferences(
  value: string,
): Array<CssVariableReference> {
  const references: Array<CssVariableReference> = [];

  walkCssSyntax(value, 0, (character, index) => {
    if (
      character.toLowerCase() !== 'v' ||
      value.slice(index, index + 4).toLowerCase() !== 'var(' ||
      IDENTIFIER_CHARACTER.test(value[index - 1] ?? '')
    ) {
      return true;
    }

    const openIndex = index + 3;
    const closeIndex = findMatchingParenthesis(value, openIndex);
    if (closeIndex === -1) return true;

    const body = value.slice(openIndex + 1, closeIndex);
    const name = splitTopLevel(body, (candidate) => candidate === ',')[0]
      ?.value;
    if (name == null || !CUSTOM_PROPERTY_NAME.test(name)) return true;

    const bodyOffset = body.indexOf(name);
    references.push({
      name,
      start: openIndex + 1 + bodyOffset,
      end: openIndex + 1 + bodyOffset + name.length,
    });
    return true;
  });

  return references;
}
