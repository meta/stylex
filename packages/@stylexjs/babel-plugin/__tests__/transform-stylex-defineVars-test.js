/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();

import { transformSync } from '@babel/core';
import stylexPlugin from '../src/index';

function transform(source, opts = {}) {
  const { code, metadata } = transformSync(source, {
    filename: opts.filename || '/stylex/packages/vars.stylex.js',
    parserOpts: {
      flow: 'all',
    },
    babelrc: false,
    plugins: [
      [
        stylexPlugin,
        {
          unstable_moduleResolution: {
            rootDir: '/stylex/packages/',
            type: 'commonJS',
          },
          ...opts,
        },
      ],
    ],
  });
  return { code, metadata };
}

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] stylex.defineVars()', () => {
    test('tokens as null', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: null,
          nextColor: {
            default: null
          },
          otherColor: {
            default: null,
            '@media (prefers-color-scheme: dark)': null,
            '@media print': null,
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          nextColor: "var(--xk6xtqk)",
          otherColor: "var(--xaaua2w)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [],
        }
      `);
    });

    test('tokens object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red',
          nextColor: {
            default: 'green'
          },
          otherColor: {
            default: 'blue',
            '@media (prefers-color-scheme: dark)': 'lightblue',
            '@media print': 'white',
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          nextColor: "var(--xk6xtqk)",
          otherColor: "var(--xaaua2w)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xaaua2w:white;}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('tokens object (haste)', () => {
      const options = {
        unstable_moduleResolution: { type: 'haste' },
      };

      const { code, metadata } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red',
          nextColor: {
            default: 'green'
          },
          otherColor: {
            default: 'blue',
            '@media (prefers-color-scheme: dark)': 'lightblue',
            '@media print': 'white',
          },
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          nextColor: "var(--xk6xtqk)",
          otherColor: "var(--xaaua2w)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xaaua2w:white;}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('tokens object deep in file tree', () => {
      const options = {
        filename: '/stylex/packages/src/css/vars.stylex.js',
      };
      const { code, metadata } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red'
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xt4ziaz)",
          __varGroupHash__: "x1xohuxq"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1xohuxq",
              {
                "ltr": ":root, .x1xohuxq{--xt4ziaz:red;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('tokens object with nested @-rules', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: {
            default: 'blue',
            '@media (prefers-color-scheme: dark)': {
              default: 'lightblue',
              '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
            }
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.3,
            ],
          ],
        }
      `);
    });

    test('literal tokens object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          '--color': 'red',
          '--otherColor': {
            default: 'blue',
            ':hover': 'lightblue',
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          "--color": "var(--color)",
          "--otherColor": "var(--otherColor)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--color:red;--otherColor:blue;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-1tdxo4z",
              {
                "ltr": ":hover{:root, .xop34xu{--otherColor:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('local variable tokens object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const tokens = {
          '--color': 'red',
          '--nextColor': {
            default: 'green'
          },
          '--otherColor': {
            default: 'blue',
            '@media (prefers-color-scheme: dark)': 'lightblue',
            '@media print': 'white',
          },
        };
        export const vars = stylex.defineVars(tokens)
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const tokens = {
          '--color': 'red',
          '--nextColor': {
            default: 'green'
          },
          '--otherColor': {
            default: 'blue',
            '@media (prefers-color-scheme: dark)': 'lightblue',
            '@media print': 'white'
          }
        };
        export const vars = {
          "--color": "var(--color)",
          "--nextColor": "var(--nextColor)",
          "--otherColor": "var(--otherColor)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--color:red;--nextColor:green;--otherColor:blue;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--otherColor:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--otherColor:white;}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('local variables used in tokens objects', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const COLOR = 'red';
        export const vars = stylex.defineVars({
          color: COLOR
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const COLOR = 'red';
        export const vars = {
          color: "var(--xwx8imx)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('template literals used in tokens objects', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const NUMBER = 10;
        export const vars = stylex.defineVars({
          size: \`\${NUMBER}rem\`
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const NUMBER = 10;
        export const vars = {
          size: "var(--xu6xznv)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xu6xznv:10rem;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('expressions used in tokens objects', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const NUMBER = 10;
        export const vars = stylex.defineVars({
          radius: NUMBER * 2
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const NUMBER = 10;
        export const vars = {
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xbbre8:20;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('stylex.types used in tokens object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: stylex.types.color({
            default: 'red',
            '@media (prefers-color-scheme: dark)': 'white',
            '@media print': 'black',
          })
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          __varGroupHash__: "xop34xu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xwx8imx",
              {
                "ltr": "@property --xwx8imx { syntax: "<color>"; inherits: true; initial-value: red }",
                "rtl": null,
              },
              0,
            ],
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:black;}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('same-group references inside function values', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          text: 'black',
          textMuted: () => \`color-mix(\${colors.text}, transparent 50%)\`,
          textSubtle: () => \`color-mix(\${colors.textMuted}, white 10%)\`,
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const colors = {
          text: "var(--x16qwlje)",
          textMuted: "var(--xdqjk6h)",
          textSubtle: "var(--x1d8pfhh)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--x16qwlje:black;--xdqjk6h:color-mix(var(--x16qwlje), transparent 50%);--x1d8pfhh:color-mix(var(--xdqjk6h), white 10%);}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('same-group references can point to later keys', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          textMuted: () => \`color-mix(\${colors.text}, transparent 50%)\`,
          text: 'black',
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const colors = {
          textMuted: "var(--xdqjk6h)",
          text: "var(--x16qwlje)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--xdqjk6h:color-mix(var(--x16qwlje), transparent 50%);--x16qwlje:black;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('nested function leaves resolve same-group refs inside at-rules', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          text: 'black',
          textMuted: () => ({
            default: \`color-mix(\${colors.text}, transparent 50%)\`,
            '@media (prefers-color-scheme: dark)': \`color-mix(\${colors.text}, white 20%)\`,
          }),
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const colors = {
          text: "var(--x16qwlje)",
          textMuted: "var(--xdqjk6h)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--x16qwlje:black;--xdqjk6h:color-mix(var(--x16qwlje), transparent 50%);}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xbqm7bo-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xbqm7bo{--xdqjk6h:color-mix(var(--x16qwlje), white 20%);}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('function values can mix same-group and same-file references', () => {
      const { code, metadata } = transform(
        `
          import * as stylex from '@stylexjs/stylex';

          export const base = stylex.defineVars({
            overlay: 'rgba(0 0 0 / 0.3)',
          });

          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: () =>
              \`color-mix(\${colors.text}, \${base.overlay} 20%)\`,
          });
        `,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const base = {
          overlay: "var(--x7fhb9m)",
          __varGroupHash__: "xxudsav"
        };
        export const colors = {
          text: "var(--x16qwlje)",
          textMuted: "var(--xdqjk6h)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xxudsav",
              {
                "ltr": ":root, .xxudsav{--x7fhb9m:rgba(0 0 0 / 0.3);}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--x16qwlje:black;--xdqjk6h:color-mix(var(--x16qwlje), var(--x7fhb9m) 20%);}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('function values can reference typed vars', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          accent: stylex.types.color('red'),
          accentGlow: () => \`color-mix(\${colors.accent}, white 15%)\`,
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const colors = {
          accent: "var(--xgpbgsq)",
          accentGlow: "var(--x1ntk1pn)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xgpbgsq",
              {
                "ltr": "@property --xgpbgsq { syntax: "<color>"; inherits: true; initial-value: red }",
                "rtl": null,
              },
              0,
            ],
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--xgpbgsq:red;--x1ntk1pn:color-mix(var(--xgpbgsq), white 15%);}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('function values can return stylex.types', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          accent: () => stylex.types.color('red'),
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const colors = {
          accent: "var(--xgpbgsq)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xgpbgsq",
              {
                "ltr": "@property --xgpbgsq { syntax: "<color>"; inherits: true; initial-value: red }",
                "rtl": null,
              },
              0,
            ],
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--xgpbgsq:red;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('function values can return nested objects', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          accent: () => ({
            default: 'red',
            '@media (prefers-color-scheme: dark)': 'blue',
          }),
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const colors = {
          accent: "var(--xgpbgsq)",
          __varGroupHash__: "xbqm7bo"
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xbqm7bo",
              {
                "ltr": ":root, .xbqm7bo{--xgpbgsq:red;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xbqm7bo-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xbqm7bo{--xgpbgsq:blue;}}",
                "rtl": null,
              },
              0.2,
            ],
          ],
        }
      `);
    });

    test('multiple variables objects (same file)', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red'
        });
        export const otherVars = stylex.defineVars({
          otherColor: 'orange'
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          __varGroupHash__: "xop34xu"
        };
        export const otherVars = {
          otherColor: "var(--xnjepv0)",
          __varGroupHash__: "x1pfrffu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "x1pfrffu",
              {
                "ltr": ":root, .x1pfrffu{--xnjepv0:orange;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('multiple variables objects (dependency)', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red'
        });
        export const otherVars = stylex.defineVars({
          otherColor: vars.color
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          __varGroupHash__: "xop34xu"
        };
        export const otherVars = {
          otherColor: "var(--xnjepv0)",
          __varGroupHash__: "x1pfrffu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "x1pfrffu",
              {
                "ltr": ":root, .x1pfrffu{--xnjepv0:var(--xwx8imx);}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    test('multiple variables objects (different files)', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red'
        });
      `);

      const { code: code2, metadata: metadata2 } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const otherVars = stylex.defineVars({
          otherColor: 'orange'
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          __varGroupHash__: "xop34xu"
        };"
      `);
      expect(code2).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const otherVars = {
          otherColor: "var(--xnjepv0)",
          __varGroupHash__: "x1pfrffu"
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:red;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
      expect(metadata2).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1pfrffu",
              {
                "ltr": ":root, .x1pfrffu{--xnjepv0:orange;}",
                "rtl": null,
              },
              0.1,
            ],
          ],
        }
      `);
    });

    describe('options `debug:true`', () => {
      test('tokens object uses hashed variable names', () => {
        const options = { debug: true };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            color: {
              default: 'blue',
              '@media (prefers-color-scheme: dark)': {
                default: 'lightblue',
                '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
              }
            },
            otherColor: 'green'
          });
        `,
          options,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            __varGroupHash__: "xop34xu"
          };"
        `);

        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:green;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1e6ryz3",
                {
                  "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:oklab(0.7 -0.3 -0.4);}}}",
                  "rtl": null,
                },
                0.3,
              ],
            ],
          }
        `);
      });

      test('tokens with special-character keys use hashed variable names', () => {
        const options = { debug: true };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            '10': 'green',
            '1.5 pixels': 'blue',
            'corner#radius': 'purple',
            '@@primary': 'pink'
          });
        `,
          options,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            "10": "var(--x187fpdw)",
            "1.5 pixels": "var(--x15ahj5d)",
            "corner#radius": "var(--x2ajqv2)",
            "@@primary": "var(--x13tvx0f)",
            __varGroupHash__: "xop34xu"
          };"
        `);

        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--x187fpdw:green;--x15ahj5d:blue;--x2ajqv2:purple;--x13tvx0f:pink;}",
                  "rtl": null,
                },
                0.1,
              ],
            ],
          }
        `);
      });
    });

    describe('options `dev:true`', () => {
      test('tokens object', () => {
        const options = { dev: true };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            color: 'red',
            nextColor: 'green',
            otherColor: 'blue'
          });
        `,
          options,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            nextColor: "var(--xk6xtqk)",
            otherColor: "var(--xaaua2w)",
            __varGroupHash__: "xop34xu"
          };"
        `);

        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}",
                  "rtl": null,
                },
                0.1,
              ],
            ],
          }
        `);
      });
    });

    describe('options `runtimeInjection:true`', () => {
      test('tokens object', () => {
        const options = { runtimeInjection: true };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            color: 'red',
            nextColor: 'green',
            otherColor: 'blue'
          });
        `,
          options,
        );

        expect(code).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import * as stylex from '@stylexjs/stylex';
          _inject2({
            ltr: ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}",
            priority: 0.1
          });
          export const vars = {
            color: "var(--xwx8imx)",
            nextColor: "var(--xk6xtqk)",
            otherColor: "var(--xaaua2w)",
            __varGroupHash__: "xop34xu"
          };"
        `);

        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:red;--xk6xtqk:green;--xaaua2w:blue;}",
                  "rtl": null,
                },
                0.1,
              ],
            ],
          }
        `);
      });
    });

    describe('options `themeFileExtension`', () => {
      test('processes tokens in files with configured extension', () => {
        const options = {
          debug: true,
          filename: '/stylex/packages/src/vars/default.cssvars.js',
          unstable_moduleResolution: {
            rootDir: '/stylex/packages/',
            themeFileExtension: 'cssvars',
            type: 'commonJS',
          },
        };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            color: 'red'
          });
        `,
          options,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--x1lzcbr1)",
            __varGroupHash__: "x1bxutiz"
          };"
        `);

        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x1bxutiz",
                {
                  "ltr": ":root, .x1bxutiz{--x1lzcbr1:red;}",
                  "rtl": null,
                },
                0.1,
              ],
            ],
          }
        `);
      });
    });
  });
});
