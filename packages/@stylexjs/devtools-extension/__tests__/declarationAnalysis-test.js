/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

import {
  parseDeclarationList,
  readAuthoredDeclarations,
} from '../src/inspected/declarationAnalysis';

function createChromiumStyle(cssText, normalizedProperties) {
  return {
    cssText,
    length: normalizedProperties.length,
    item: (index) => normalizedProperties[index] ?? '',
    getPropertyPriority: () => '',
    getPropertyValue: () => 'canvastext',
  };
}

test('preserves an authored shorthand that CSSOM expands into longhands', () => {
  const style = createChromiumStyle('border-color: canvastext;', [
    'border-top-color',
    'border-right-color',
    'border-bottom-color',
    'border-left-color',
  ]);

  expect(readAuthoredDeclarations(style)).toEqual([
    {
      property: 'border-color',
      value: 'canvastext',
      important: false,
    },
  ]);
});

test('parses important values containing declaration delimiters', () => {
  const style = createChromiumStyle(
    '--token: "a;b:c"; background-image: url("data:image/svg+xml;a:b"); color: red !important;',
    ['--token', 'background-image', 'color'],
  );

  expect(readAuthoredDeclarations(style)).toEqual([
    { property: '--token', value: '"a;b:c"', important: false },
    {
      property: 'background-image',
      value: 'url("data:image/svg+xml;a:b")',
      important: false,
    },
    { property: 'color', value: 'red', important: true },
  ]);
});

test('handles escaped delimiters, comments, and nested blocks', () => {
  expect(
    parseDeclarationList(
      '--escaped: value\\;still-value; --block: { nested: value; }; --commented: a/* ;: */b; color: blue;',
    ),
  ).toEqual([
    {
      property: '--escaped',
      value: 'value\\;still-value',
      important: false,
    },
    {
      property: '--block',
      value: '{ nested: value; }',
      important: false,
    },
    {
      property: '--commented',
      value: 'a/* ;: */b',
      important: false,
    },
    {
      property: 'color',
      value: 'blue',
      important: false,
    },
  ]);
});
