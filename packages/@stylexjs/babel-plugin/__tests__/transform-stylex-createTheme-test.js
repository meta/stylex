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
    filename: opts.filename,
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

/**
 * Fixture factory
 *
 * This is used to create a consistent fixture across all the tests for
 * different ways of importing StyleX. The fixture uses all of the
 * StyleX exports to make sure they are transformed as expected.
 */

function transformWithFixture(fixture, fixtureOptions, pluginOptions) {
  const { varsFilename, varsLiterals } = fixtureOptions || {};

  // Generate defineVars output first.
  // This is inlined into the fixture so that createTheme works.
  const createKey = (key) => (varsLiterals ? `'--${key}'` : key);
  const fixtureTransformOptions = {
    filename: varsFilename || '/stylex/packages/vars.stylex.js',
  };
  const { code: _code, metadata: _metadata } = transform(
    `
    import * as stylex from '@stylexjs/stylex';
    export const vars = stylex.defineVars({
      ${createKey('color')}: {
        default: 'blue',
        '@media (prefers-color-scheme: dark)': 'lightblue',
        '@media print': 'white',
      },
      ${createKey('otherColor')}: {
        default: 'grey',
        '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
      },
      ${createKey('radius')}: 10
    });
  `,
    fixtureTransformOptions,
  );

  // Generate the final transform
  const { code, metadata } = transform(
    `
    ${_code}
    ${fixture}
  `,
    pluginOptions,
  );

  _metadata.stylex.push(...metadata.stylex);

  return { code, metadata: _metadata };
}

const themeObject = `{
  color: {
    default: 'green',
    '@media (prefers-color-scheme: dark)': 'lightgreen',
    '@media print': 'transparent',
  },
  otherColor: {
    default: 'antiquewhite',
    '@media (prefers-color-scheme: dark)': {
      default: 'floralwhite',
      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
    }
  },
  radius: '6px'
}`;

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] stylex.createTheme()', () => {
    test('theme object', () => {
      const { code, metadata } = transformWithFixture(`
        export const theme = stylex.createTheme(vars, ${themeObject});
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        export const theme = {
          xop34xu: "x1odz0le xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1odz0le",
              {
                "ltr": ".x1odz0le, .x1odz0le:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1odz0le-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "x1odz0le-bdddrq",
              {
                "ltr": "@media print{.x1odz0le, .x1odz0le:root{--xwx8imx:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xwx8imx:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('theme object (haste)', () => {
      const pluginOptions = {
        unstable_moduleResolution: { type: 'haste' },
      };
      const { code, metadata } = transformWithFixture(
        `
        export const theme = stylex.createTheme(vars, ${themeObject});
      `,
        null,
        pluginOptions,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        export const theme = {
          xop34xu: "x1odz0le xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1odz0le",
              {
                "ltr": ".x1odz0le, .x1odz0le:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1odz0le-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "x1odz0le-bdddrq",
              {
                "ltr": "@media print{.x1odz0le, .x1odz0le:root{--xwx8imx:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xwx8imx:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('theme object deep in file tree', () => {
      const fixtureOptions = {
        varsFilename: '/stylex/packages/src/css/vars.stylex.js',
      };
      const { code, metadata } = transformWithFixture(
        `
        export const theme = stylex.createTheme(vars, ${themeObject});
      `,
        fixtureOptions,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xt4ziaz)",
          otherColor: "var(--x1e3it8h)",
          radius: "var(--x1onrunl)",
          __varGroupHash__: "x1xohuxq"
        };
        export const theme = {
          x1xohuxq: "xn547xa x1xohuxq",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1xohuxq",
              {
                "ltr": ":root, .x1xohuxq{--xt4ziaz:blue;--x1e3it8h:grey;--x1onrunl:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "x1xohuxq-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .x1xohuxq{--xt4ziaz:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1xohuxq-bdddrq",
              {
                "ltr": "@media print{:root, .x1xohuxq{--xt4ziaz:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1xohuxq-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .x1xohuxq{--x1e3it8h:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xn547xa",
              {
                "ltr": ".xn547xa, .xn547xa:root{--xt4ziaz:green;--x1e3it8h:antiquewhite;--x1onrunl:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "xn547xa-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.xn547xa, .xn547xa:root{--x1e3it8h:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "xn547xa-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.xn547xa, .xn547xa:root{--x1e3it8h:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "xn547xa-bdddrq",
              {
                "ltr": "@media print{.xn547xa, .xn547xa:root{--xt4ziaz:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "xn547xa-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.xn547xa, .xn547xa:root{--xt4ziaz:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('theme object with overlapping media queries', () => {
      const { metadata } = transformWithFixture(`
        export const theme = stylex.createTheme(vars, {
          radius: {
            default: '6px',
            '@media (min-height: 1px)': {
              default: '8px',
              '@media (min-width: 1px)': '10px',
            },
            '@media (min-width: 2px)': '12px',
          },
        });
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1fp6i2x",
              {
                "ltr": ".x1fp6i2x, .x1fp6i2x:root{--xbbre8:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1fp6i2x-1pknkiz",
              {
                "ltr": "@media (max-width: 1.98px) and (min-height: 1px){.x1fp6i2x, .x1fp6i2x:root{--xbbre8:8px;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1fp6i2x-1ouog6w",
              {
                "ltr": "@media (min-width: 1px){@media (max-width: 1.98px) and (min-height: 1px){.x1fp6i2x, .x1fp6i2x:root{--xbbre8:10px;}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "x1fp6i2x-w15jht",
              {
                "ltr": "@media (min-width: 2px){.x1fp6i2x, .x1fp6i2x:root{--xbbre8:12px;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('literal tokens theme object', () => {
      const fixtureOptions = { varsLiterals: true };

      const { code, metadata } = transformWithFixture(
        `
        export const theme = stylex.createTheme(vars, {
          '--color': 'green',
          '--otherColor': 'purple',
          '--radius': 6
        });
      `,
        fixtureOptions,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          "--color": "var(--color)",
          "--otherColor": "var(--otherColor)",
          "--radius": "var(--radius)",
          __varGroupHash__: "xop34xu"
        };
        export const theme = {
          xop34xu: "x1l2ihi1 xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--color:blue;--otherColor:grey;--radius:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--color:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--color:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--otherColor:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1l2ihi1",
              {
                "ltr": ".x1l2ihi1, .x1l2ihi1:root{--color:green;--otherColor:purple;--radius:6;}",
                "rtl": null,
              },
              0.5,
            ],
          ],
        }
      `);
    });

    test('local variable theme object', () => {
      const { code, metadata } = transformWithFixture(`
        const themeObj = ${themeObject};
        export const theme = stylex.createTheme(vars, themeObj);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        const themeObj = {
          color: {
            default: 'green',
            '@media (prefers-color-scheme: dark)': 'lightgreen',
            '@media print': 'transparent'
          },
          otherColor: {
            default: 'antiquewhite',
            '@media (prefers-color-scheme: dark)': {
              default: 'floralwhite',
              '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)'
            }
          },
          radius: '6px'
        };
        export const theme = {
          xop34xu: "x1odz0le xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1odz0le",
              {
                "ltr": ".x1odz0le, .x1odz0le:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1odz0le-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "x1odz0le-bdddrq",
              {
                "ltr": "@media print{.x1odz0le, .x1odz0le:root{--xwx8imx:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xwx8imx:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('local variables used in theme objects', () => {
      const { code, metadata } = transformWithFixture(`
        const RADIUS = 10;
        export const theme = stylex.createTheme(vars, {
          radius: RADIUS
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        const RADIUS = 10;
        export const theme = {
          xop34xu: "x1s6ff5p xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1s6ff5p",
              {
                "ltr": ".x1s6ff5p, .x1s6ff5p:root{--xbbre8:10;}",
                "rtl": null,
              },
              0.5,
            ],
          ],
        }
      `);
    });

    test('stylex.env.override for partial theme overrides', () => {
      const { code, metadata } = transformWithFixture(
        `
        export const theme = stylex.createTheme(vars,
          stylex.env.override(stylex.env.tokens, {
            color: 'red',
          })
        );
      `,
        undefined,
        {
          env: {
            tokens: {
              color: {
                default: 'blue',
                '@media (prefers-color-scheme: dark)': 'lightblue',
                '@media print': 'white',
              },
              otherColor: {
                default: 'grey',
                '@media (prefers-color-scheme: dark)': 'rgba(0, 0, 0, 0.8)',
              },
              radius: 10,
            },
            override: (base, overrides) => ({ ...base, ...overrides }),
          },
        },
      );
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        export const theme = {
          xop34xu: "x1ahfulb xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1ahfulb",
              {
                "ltr": ".x1ahfulb, .x1ahfulb:root{--xwx8imx:red;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1ahfulb-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1ahfulb, .x1ahfulb:root{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('template literals used in theme objects', () => {
      const { code, metadata } = transformWithFixture(`
        const name = 'light';
        export const theme = stylex.createTheme(vars, {
          color: \`\${name}green\`
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        const name = 'light';
        export const theme = {
          xop34xu: "xp8mj21 xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xp8mj21",
              {
                "ltr": ".xp8mj21, .xp8mj21:root{--xwx8imx:lightgreen;}",
                "rtl": null,
              },
              0.5,
            ],
          ],
        }
      `);
    });

    test('expressions used in theme objects', () => {
      const { code, metadata } = transformWithFixture(`
        const RADIUS = 10;
        export const theme = stylex.createTheme(vars, {
          radius: RADIUS * 2
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        const RADIUS = 10;
        export const theme = {
          xop34xu: "x1et03wi xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1et03wi",
              {
                "ltr": ".x1et03wi, .x1et03wi:root{--xbbre8:20;}",
                "rtl": null,
              },
              0.5,
            ],
          ],
        }
      `);
    });

    test('stylex.types used in theme object', () => {
      const { code, metadata } = transformWithFixture(`
       const RADIUS = 10;
        export const theme = stylex.createTheme(vars, {
          color: stylex.types.color({
            default: 'green',
            '@media (prefers-color-scheme: dark)': 'lightgreen',
            '@media print': 'transparent',
          }),
          otherColor: stylex.types.color({
            default: 'antiquewhite',
            '@media (prefers-color-scheme: dark)': 'floralwhite',
          }),
          radius: stylex.types.length({ default: RADIUS * 2 })
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        const RADIUS = 10;
        export const theme = {
          xop34xu: "x1ttfuy6 xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1ttfuy6",
              {
                "ltr": ".x1ttfuy6, .x1ttfuy6:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:20px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1ttfuy6-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1ttfuy6, .x1ttfuy6:root{--xaaua2w:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1ttfuy6-bdddrq",
              {
                "ltr": "@media print{.x1ttfuy6, .x1ttfuy6:root{--xwx8imx:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1ttfuy6-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.x1ttfuy6, .x1ttfuy6:root{--xwx8imx:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('multiple theme objects (same vars)', () => {
      const { code, metadata } = transformWithFixture(`
        export const theme = stylex.createTheme(vars, ${themeObject});
        export const otherTheme = stylex.createTheme(vars, {
          color: 'skyblue',
          radius: '8px',
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        export const theme = {
          xop34xu: "x1odz0le xop34xu",
          $$css: true
        };
        export const otherTheme = {
          xop34xu: "xw6msop xop34xu",
          $$css: true
        };"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1odz0le",
              {
                "ltr": ".x1odz0le, .x1odz0le:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1odz0le-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "x1odz0le-bdddrq",
              {
                "ltr": "@media print{.x1odz0le, .x1odz0le:root{--xwx8imx:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xwx8imx:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "xw6msop",
              {
                "ltr": ".xw6msop, .xw6msop:root{--xwx8imx:skyblue;--xbbre8:8px;}",
                "rtl": null,
              },
              0.5,
            ],
          ],
        }
      `);
    });

    test('multiple theme objects (different vars)', () => {
      const { code: code1, metadata: metadata1 } = transformWithFixture(`
        export const theme = stylex.createTheme(vars, ${themeObject});
      `);
      const { code: code2, metadata: metadata2 } = transformWithFixture(
        `
        export const theme = stylex.createTheme(vars, ${themeObject});
      `,
        { varsFilename: '/stylex/packages/otherVars.stylex.js' },
      );

      expect(code1).not.toEqual(code2);
      expect(metadata1).not.toEqual(metadata2);

      expect(code1).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--xwx8imx)",
          otherColor: "var(--xaaua2w)",
          radius: "var(--xbbre8)",
          __varGroupHash__: "xop34xu"
        };
        export const theme = {
          xop34xu: "x1odz0le xop34xu",
          $$css: true
        };"
      `);

      expect(code2).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const vars = {
          color: "var(--x103gslp)",
          otherColor: "var(--x1e7put6)",
          radius: "var(--xm3n3tg)",
          __varGroupHash__: "x1ngxneg"
        };
        export const theme = {
          x1ngxneg: "xkyujlh x1ngxneg",
          $$css: true
        };"
      `);

      expect(metadata1).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xop34xu",
              {
                "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "xop34xu-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-bdddrq",
              {
                "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xop34xu-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1odz0le",
              {
                "ltr": ".x1odz0le, .x1odz0le:root{--xwx8imx:green;--xaaua2w:antiquewhite;--xbbre8:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "x1odz0le-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xaaua2w:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "x1odz0le-bdddrq",
              {
                "ltr": "@media print{.x1odz0le, .x1odz0le:root{--xwx8imx:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "x1odz0le-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.x1odz0le, .x1odz0le:root{--xwx8imx:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);

      expect(metadata2).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1ngxneg",
              {
                "ltr": ":root, .x1ngxneg{--x103gslp:blue;--x1e7put6:grey;--xm3n3tg:10;}",
                "rtl": null,
              },
              0.1,
            ],
            [
              "x1ngxneg-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){:root, .x1ngxneg{--x103gslp:lightblue;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1ngxneg-bdddrq",
              {
                "ltr": "@media print{:root, .x1ngxneg{--x103gslp:white;}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "x1ngxneg-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){:root, .x1ngxneg{--x1e7put6:rgba(0, 0, 0, 0.8);}}",
                "rtl": null,
              },
              0.2,
            ],
            [
              "xkyujlh",
              {
                "ltr": ".xkyujlh, .xkyujlh:root{--x103gslp:green;--x1e7put6:antiquewhite;--xm3n3tg:6px;}",
                "rtl": null,
              },
              0.5,
            ],
            [
              "xkyujlh-1lveb7",
              {
                "ltr": "@media (prefers-color-scheme: dark){.xkyujlh, .xkyujlh:root{--x1e7put6:floralwhite;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "xkyujlh-1e6ryz3",
              {
                "ltr": "@supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){.xkyujlh, .xkyujlh:root{--x1e7put6:oklab(0.7 -0.3 -0.4);}}}",
                "rtl": null,
              },
              0.7,
            ],
            [
              "xkyujlh-bdddrq",
              {
                "ltr": "@media print{.xkyujlh, .xkyujlh:root{--x103gslp:transparent;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
            [
              "xkyujlh-28esbo",
              {
                "ltr": "@media screen and (prefers-color-scheme: dark){.xkyujlh, .xkyujlh:root{--x103gslp:lightgreen;}}",
                "rtl": null,
              },
              0.6000000000000001,
            ],
          ],
        }
      `);
    });

    test('themes are indifferent to order of keys', () => {
      const { code: code1, metadata: metadata1 } = transformWithFixture(`
        export const theme = stylex.createTheme(vars, ${themeObject});
      `);
      const { code: code2, metadata: metadata2 } = transformWithFixture(`
        export const theme = stylex.createTheme(vars, {
          radius: '6px',
          otherColor: {
            default: 'antiquewhite',
            '@media (prefers-color-scheme: dark)': {
              default: 'floralwhite',
              '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
            }
          },
          color: {
            default: 'green',
            '@media (prefers-color-scheme: dark)': 'lightgreen',
            '@media print': 'transparent',
          }
        });
      `);

      expect(code1).toEqual(code2);
      expect(metadata1).toEqual(metadata2);
    });

    // TODO: Add debug data to compiled themes
    describe('options `debug:true`', () => {
      test('adds debug data', () => {
        const pluginOptions = {
          debug: true,
          filename: '/html/js/components/Foo.react.js',
        };
        const { code, metadata } = transformWithFixture(
          `
          export const theme = stylex.createTheme(vars, {
            color: 'orange'
          });
        `,
          null,
          pluginOptions,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            radius: "var(--xbbre8)",
            __varGroupHash__: "xop34xu"
          };
          export const theme = {
            xop34xu: "xowvtgn xop34xu",
            $$css: true
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-28esbo",
                {
                  "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-bdddrq",
                {
                  "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xowvtgn",
                {
                  "ltr": ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  "rtl": null,
                },
                0.5,
              ],
            ],
          }
        `);
      });

      test('adds debug data for npm packages', () => {
        const pluginOptions = {
          debug: true,
          filename: '/js/node_modules/npm-package/dist/components/Foo.react.js',
        };
        const { code, metadata } = transformWithFixture(
          `
          export const theme = stylex.createTheme(vars, {
            color: 'orange'
          });
        `,
          null,
          pluginOptions,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            radius: "var(--xbbre8)",
            __varGroupHash__: "xop34xu"
          };
          export const theme = {
            xop34xu: "xowvtgn xop34xu",
            $$css: true
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-28esbo",
                {
                  "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-bdddrq",
                {
                  "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xowvtgn",
                {
                  "ltr": ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  "rtl": null,
                },
                0.5,
              ],
            ],
          }
        `);
      });

      test('adds debug data (haste)', () => {
        const pluginOptions = {
          debug: true,
          filename: '/html/js/components/Foo.react.js',
          unstable_moduleResolution: { type: 'haste' },
        };
        const { code, metadata } = transformWithFixture(
          `
          export const theme = stylex.createTheme(vars, {
            color: 'orange'
          });
        `,
          null,
          pluginOptions,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            radius: "var(--xbbre8)",
            __varGroupHash__: "xop34xu"
          };
          export const theme = {
            xop34xu: "xowvtgn xop34xu",
            $$css: true
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-28esbo",
                {
                  "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-bdddrq",
                {
                  "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xowvtgn",
                {
                  "ltr": ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  "rtl": null,
                },
                0.5,
              ],
            ],
          }
        `);
      });

      test('adds debug data for npm packages (haste)', () => {
        const pluginOptions = {
          debug: true,
          filename: '/node_modules/npm-package/dist/components/Foo.react.js',
          unstable_moduleResolution: { type: 'haste' },
        };
        const { code, metadata } = transformWithFixture(
          `
          export const theme = stylex.createTheme(vars, {
            color: 'orange'
          });
        `,
          null,
          pluginOptions,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            radius: "var(--xbbre8)",
            __varGroupHash__: "xop34xu"
          };
          export const theme = {
            xop34xu: "xowvtgn xop34xu",
            $$css: true
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-28esbo",
                {
                  "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-bdddrq",
                {
                  "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xowvtgn",
                {
                  "ltr": ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  "rtl": null,
                },
                0.5,
              ],
            ],
          }
        `);
      });
    });

    describe('options `dev:true`', () => {
      test('adds dev data', () => {
        const pluginOptions = {
          dev: true,
          filename: '/html/js/components/Foo.react.js',
        };
        const { code, metadata } = transformWithFixture(
          `
          export const theme = stylex.createTheme(vars, {
            color: 'orange'
          });
        `,
          null,
          pluginOptions,
        );

        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            radius: "var(--xbbre8)",
            __varGroupHash__: "xop34xu"
          };
          export const theme = {
            Foo__theme: "Foo__theme",
            xop34xu: "xowvtgn xop34xu",
            $$css: true
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-28esbo",
                {
                  "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-bdddrq",
                {
                  "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xowvtgn",
                {
                  "ltr": ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  "rtl": null,
                },
                0.5,
              ],
            ],
          }
        `);
      });
    });

    describe('options `runtimeInjection:true`', () => {
      test('adds style injection', () => {
        const pluginOptions = {
          filename: '/html/js/components/Foo.react.js',
          runtimeInjection: true,
        };
        const { code, metadata } = transformWithFixture(
          `
          export const theme = stylex.createTheme(vars, {
            color: 'orange'
          });
        `,
          null,
          pluginOptions,
        );

        expect(code).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import * as stylex from '@stylexjs/stylex';
          export const vars = {
            color: "var(--xwx8imx)",
            otherColor: "var(--xaaua2w)",
            radius: "var(--xbbre8)",
            __varGroupHash__: "xop34xu"
          };
          _inject2({
            ltr: ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
            priority: 0.5
          });
          export const theme = {
            xop34xu: "xowvtgn xop34xu",
            $$css: true
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xop34xu",
                {
                  "ltr": ":root, .xop34xu{--xwx8imx:blue;--xaaua2w:grey;--xbbre8:10;}",
                  "rtl": null,
                },
                0.1,
              ],
              [
                "xop34xu-28esbo",
                {
                  "ltr": "@media screen and (prefers-color-scheme: dark){:root, .xop34xu{--xwx8imx:lightblue;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-bdddrq",
                {
                  "ltr": "@media print{:root, .xop34xu{--xwx8imx:white;}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xop34xu-1lveb7",
                {
                  "ltr": "@media (prefers-color-scheme: dark){:root, .xop34xu{--xaaua2w:rgba(0, 0, 0, 0.8);}}",
                  "rtl": null,
                },
                0.2,
              ],
              [
                "xowvtgn",
                {
                  "ltr": ".xowvtgn, .xowvtgn:root{--xwx8imx:orange;}",
                  "rtl": null,
                },
                0.5,
              ],
            ],
          }
        `);
      });
    });
  });
});
