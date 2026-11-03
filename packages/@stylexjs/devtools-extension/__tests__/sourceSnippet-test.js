/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { formatSourceSnippet } = require('../src/utils/sourceSnippet');

test('previews exactly the style object identified by the source line', () => {
  const content = [
    'const styles = stylex.create({',
    '  previous: {',
    '    color: "red",',
    '  },',
    '  wordInnerDiv: {',
    '    alignItems: "flex-start",',
    '    display: "flex",',
    '    height: "100%",',
    '    justifyContent: "center",',
    '    overflow: "hidden",',
    '    position: "relative",',
    '  },',
    '  next: {',
    '    color: "blue",',
    '  },',
    '});',
  ].join('\n');

  expect(formatSourceSnippet(content, 5)).toBe(
    [
      '>  5 |   wordInnerDiv: {',
      '   6 |     alignItems: "flex-start",',
      '   7 |     display: "flex",',
      '   8 |     height: "100%",',
      '   9 |     justifyContent: "center",',
      '  10 |     overflow: "hidden",',
      '  11 |     position: "relative",',
      '  12 |   },',
    ].join('\n'),
  );
});

test('ignores braces in nested values, strings, templates, and comments', () => {
  const content = [
    'const styles = stylex.create({',
    '  complex: {',
    '    color: {',
    '      default: "red",',
    '      "@media (width > 10px)": "blue",',
    '    },',
    '    content: "}",',
    '    label: `value ${condition ? { nested: true } : {}} }`,',
    '    // } does not close the object',
    '    /* neither does } */',
    '  },',
    '  next: {},',
    '});',
  ].join('\n');

  const snippet = formatSourceSnippet(content, 2);

  expect(snippet).toContain('>  2 |   complex: {');
  expect(snippet).toContain('  11 |   },');
  expect(snippet).not.toContain('next: {}');
});

test('anchors to the style object when a nested object starts on the same line', () => {
  const content = [
    'const styles = stylex.create({',
    '  compact: { color: { default: "red" },',
    '    padding: 4,',
    '  },',
    '  next: {},',
    '});',
  ].join('\n');

  expect(formatSourceSnippet(content, 2)).toBe(
    [
      '> 2 |   compact: { color: { default: "red" },',
      '  3 |     padding: 4,',
      '  4 |   },',
    ].join('\n'),
  );
});

test('never includes lines before the numbered source line', () => {
  const content = ['before', 'target without an object', 'after'].join('\n');

  expect(formatSourceSnippet(content, 2)).toBe(
    ['> 2 | target without an object', '  3 | after'].join('\n'),
  );
});
