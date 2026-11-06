/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { transform } = require('./__fixtures__/css-arithmetic');

describe('CSS tokens in JavaScript conditions', () => {
  test('rejects nullish coalescing on an imported constant', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: constants.a ?? 0,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects nullish coalescing through an imported alias in a dynamic style', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.missing;
      export const styles = stylex.create({
        root: (fallback) => ({
          zIndex: local ?? fallback,
        }),
      });
    `),
    ).toThrow();
  });

  test('rejects a null comparison on an imported constant', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: constants.a == null ? 0 : 1,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects an undefined comparison with an imported alias on the right', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      export const styles = stylex.create({
        root: {
          zIndex: undefined !== local ? 1 : 0,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects an imported constant as a ternary condition', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const fallback = 0;
      export const styles = stylex.create({
        root: {
          zIndex: constants.a ? 1 : fallback,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects an arithmetic alias as the left operand of &&', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a * 2;
      export const styles = stylex.create({
        root: {
          zIndex: local && 0.5,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects an imported alias as the left operand of ||', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      export const styles = stylex.create({
        root: {
          zIndex: local || 0.5,
        },
      });
    `),
    ).toThrow();
  });

  test('rejects typeof on unresolved imported arithmetic', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a * 2;
      export const styles = stylex.create({
        root: {
          zIndex: typeof local === 'number' ? 1 : 0,
        },
      });
    `),
    ).toThrow();
  });

  test('allows a token selected by a known ternary condition', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: true ? constants.a : 0,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kY2c9j: "xz3gssr",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "xz3gssr",
          {
            "ltr": ".xz3gssr{z-index:var(--xkc4to4)}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('allows a token selected by a known logical condition', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: false || constants.a,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kY2c9j: "xz3gssr",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "xz3gssr",
          {
            "ltr": ".xz3gssr{z-index:var(--xkc4to4)}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('allows a token as the fallback for a known null value', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: null ?? constants.a,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kY2c9j: "xz3gssr",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "xz3gssr",
          {
            "ltr": ".xz3gssr{z-index:var(--xkc4to4)}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('does not evaluate an unused nullish fallback', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: String(0 ?? (constants.a || 0.5)),
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kY2c9j: "x1ja2u2z",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x1ja2u2z",
          {
            "ltr": ".x1ja2u2z{z-index:0}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('does not evaluate an unused ternary branch', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: String(true ? 1 : typeof constants.a),
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kY2c9j: "x1vjfegm",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x1vjfegm",
          {
            "ltr": ".x1vjfegm{z-index:1}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });
});
