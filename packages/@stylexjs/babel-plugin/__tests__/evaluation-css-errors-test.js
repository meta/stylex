/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { transform } = require('./__fixtures__/css-arithmetic');

describe('Invalid CSS token operations', () => {
  test('rejects a non-numeric arithmetic operand', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          width: constants.gutter * 'auto',
        },
      });
    `),
    ).toThrow();
  });
  test('rejects Math calls with an imported token in a later argument', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      const local = constants.a + 1;
      export const styles = stylex.create({
        root: {
          zIndex: Math.max(2, local),
        },
      });
    `),
    ).toThrow();
  });

  test('rejects non-finite operand Infinity', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: constants.a * Infinity,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects arithmetic that cannot be expressed with calc', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      const local = constants.a;
      export const styles = stylex.create({
        root: {
          zIndex: local % 2,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects token misuse in a static part of a dynamic style', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      export const styles = stylex.create({
        root: (opacity) => ({
          opacity,
          zIndex: Math.round(local),
        }),
      });
    `),
    ).toThrow();
  });

  test('rejects arithmetic used as a CSS property key', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      const key = 'calc(100% - 10px)';
      export const styles = stylex.create({
        root: {
          [key]: 1,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects arithmetic used as a keyframe selector', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const key = constants.a + 1;
      export const result = stylex.keyframes({
        [key]: {
          opacity: 1,
        },
      });
    `),
    ).toThrow();
  });
});
