/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import {
  formatCssValueForDisplay,
  isLongCssValue,
} from '../src/panel/declarations/formatValue';

test('leaves short values unchanged', () => {
  expect(formatCssValueForDisplay('var(--color)')).toBe('var(--color)');
  expect(isLongCssValue('var(--color)')).toBe(false);
});

test('uses leading, parenthesis-aligned commas for long functions', () => {
  const first = 'calc(var(--width) / var(--scale) * 100vw)';
  const second = 'calc(var(--height) / var(--scale) * 100vh)';
  const third = 'calc(var(--depth) / var(--scale) * 100vmin)';
  const value = `clamp(${first}, ${second}, ${third})`;

  expect(isLongCssValue(value)).toBe(true);
  expect(formatCssValueForDisplay(value)).toBe(
    [`clamp( ${first}`, `     , ${second}`, `     , ${third}`, '     )'].join(
      '\n',
    ),
  );
});

test('recursively aligns nested math, groups, commas, and closers', () => {
  const low = 'calc(var(--x1ak3ttg) / var(--x1rlgl0p))';
  const high = 'calc(var(--x1r8612k) / var(--x897lgi))';
  const progress = `((${high} - ${low}) / 880px)`;
  const middle = `calc(${low} - ${progress} * 360px + (${progress} * 100) * 1vw)`;
  const value = `clamp(${low}, ${middle}, ${high})`;

  expect(formatCssValueForDisplay(value)).toBe(
    [
      `clamp( ${low}`,
      '     , calc( calc(var(--x1ak3ttg) / var(--x1rlgl0p))',
      '           - ( ( calc(var(--x1r8612k) / var(--x897lgi))',
      '               - calc(var(--x1ak3ttg) / var(--x1rlgl0p))',
      '               )',
      '             / 880px',
      '             )',
      '           * 360px',
      '           + ( ( ( calc(var(--x1r8612k) / var(--x897lgi))',
      '                 - calc(var(--x1ak3ttg) / var(--x1rlgl0p))',
      '                 )',
      '               / 880px',
      '               )',
      '             * 100',
      '             )',
      '           * 1vw',
      '           )',
      `     , ${high}`,
      '     )',
    ].join('\n'),
  );
});

test('keeps lower-precedence math groups independently flat', () => {
  const value =
    'calc(var(--inline-size) / var(--scale) + var(--block-size) / var(--scale) - 1px)';

  expect(formatCssValueForDisplay(value, 48)).toBe(
    [
      'calc( var(--inline-size) / var(--scale)',
      '    + var(--block-size) / var(--scale)',
      '    - 1px',
      '    )',
    ].join('\n'),
  );
});

test('only treats operators at the current math depth as delimiters', () => {
  const value =
    'calc(var(--foo-bar, 1e-3) * (100% - var(--inset)) + var(--escaped\\+name))';

  expect(formatCssValueForDisplay(value, 52)).toBe(
    [
      'calc( var(--foo-bar, 1e-3) * (100% - var(--inset))',
      '    + var(--escaped\\+name)',
      '    )',
    ].join('\n'),
  );
});

test('does not split nested commas, quoted commas, or comments', () => {
  const fallback = 'color-mix(in oklab, rgb(0, 0, 0) 20%, rgb(255, 255, 255))';
  const value = `var(--a-very-long-custom-property-name, ${fallback})`;

  expect(formatCssValueForDisplay(value)).toBe(
    [
      'var( --a-very-long-custom-property-name',
      `   , ${fallback}`,
      '   )',
    ].join('\n'),
  );
  expect(
    formatCssValueForDisplay(
      'rgb(from var(--long-color-name, "a,b" /* c,d */) r g b / calc(alpha * 0.75))',
      48,
    ),
  ).toContain('"a,b" /* c,d */');
});

test('leaves URL and malformed function values untouched', () => {
  const url =
    'url("https://example.com/a-very-long-path/image.png?crop=1,2,3,4")';
  const malformed =
    'clamp(calc(var(--long-name) / var(--scale)), calc(var(--other-long-name) / var(--scale))';

  expect(formatCssValueForDisplay(url, 40)).toBe(url);
  expect(formatCssValueForDisplay(malformed, 40)).toBe(malformed);
});

test('keeps important attached to the aligned closing parenthesis', () => {
  const value =
    'clamp(calc(var(--minimum-width) / var(--scale)), calc(var(--preferred-width) / var(--scale)), calc(var(--maximum-width) / var(--scale))) !important';

  expect(formatCssValueForDisplay(value)).toBe(
    [
      'clamp( calc(var(--minimum-width) / var(--scale))',
      '     , calc(var(--preferred-width) / var(--scale))',
      '     , calc(var(--maximum-width) / var(--scale))',
      '     ) !important',
    ].join('\n'),
  );
});
