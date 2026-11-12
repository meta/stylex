/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { Doc } from './documentPrinter';
import type { ValueNode } from './valueTree';

import {
  align,
  concat,
  group,
  ifBreak,
  line,
  printDoc,
  text,
} from './documentPrinter';
import { parseCssValue } from './valueTree';

const LONG_VALUE_MIN_LENGTH = 80;
const EMPTY = text('');

export function isLongCssValue(value: string): boolean {
  return value.length >= LONG_VALUE_MIN_LENGTH;
}

function delimited(open: string, value: Doc, close: string): Doc {
  return group(
    concat([
      text(open),
      align(
        -1,
        concat([ifBreak(text(' '), EMPTY), value, line(), text(close)]),
      ),
    ]),
  );
}

function operationToDoc(node: ValueNode): Doc {
  if (node.type !== 'operation') return valueToDoc(node);

  const [first, ...rest] = node.parts;
  return group(
    concat([
      valueToDoc(first.value),
      ...rest.flatMap(({ operator, value }) => [
        line(' '),
        text(`${operator ?? ''} `),
        valueToDoc(value),
      ]),
    ]),
  );
}

function functionToDoc(node: ValueNode): Doc {
  if (node.type !== 'function') return valueToDoc(node);

  const [first, ...rest] = node.args;
  const argumentsDoc = concat([
    valueToDoc(first),
    ...rest.flatMap((argument) => [line(), text(', '), valueToDoc(argument)]),
  ]);

  return concat([
    text(node.name),
    delimited('(', argumentsDoc, `)${node.suffix}`),
  ]);
}

function valueToDoc(node: ValueNode): Doc {
  switch (node.type) {
    case 'raw':
      return text(node.value);
    case 'function':
      return functionToDoc(node);
    case 'group':
      return delimited('(', valueToDoc(node.value), ')');
    case 'operation':
      return operationToDoc(node);
    default:
      throw new Error('Unknown CSS value node.');
  }
}

export function formatCssValueForDisplay(
  value: string,
  maxLineLength: number = LONG_VALUE_MIN_LENGTH,
): string {
  if (value.length < maxLineLength || value.includes('\n')) return value;
  return printDoc(valueToDoc(parseCssValue(value)), maxLineLength);
}
