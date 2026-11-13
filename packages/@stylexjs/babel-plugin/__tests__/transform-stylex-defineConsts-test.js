/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

'use strict';

jest.autoMockOff();

import fs from 'fs';
import path from 'path';
import { transformSync } from '@babel/core';
import stylexPlugin from '../src/index';

const defaultOpts = {
  unstable_moduleResolution: { rootDir: '/stylex/packages/', type: 'commonJS' },
};

function transform(source, opts = {}) {
  const { code, metadata } = transformSync(source, {
    filename: opts.filename || '/stylex/packages/TestTheme.stylex.js',
    parserOpts: {
      flow: 'all',
    },
    babelrc: false,
    plugins: [
      [
        stylexPlugin,
        {
          ...defaultOpts,
          ...opts,
        },
      ],
    ],
  });
  return { code, metadata };
}

function transformWithInlineConsts(source, opts = {}) {
  const { code, metadata } = transformSync(source, {
    filename: path.join(__dirname, '__fixtures__/main.stylex.js'),
    parserOpts: { sourceType: 'module' },
    babelrc: false,
    plugins: [
      [
        stylexPlugin,
        {
          ...opts,
          unstable_moduleResolution: {
            rootDir: path.join(__dirname, '__fixtures__'),
            type: 'commonJS',
          },
        },
      ],
    ],
  });

  return { code, metadata };
}

// Compiles one file of __fixtures__ (or `source` under that file name) and
// returns its rules.
function transformFixtureRules(filename, source) {
  const fixtureDir = path.join(__dirname, '__fixtures__');
  const file = path.join(fixtureDir, filename);
  return transformSync(source ?? fs.readFileSync(file, 'utf8'), {
    filename: file,
    parserOpts: { sourceType: 'module' },
    babelrc: false,
    plugins: [
      [
        stylexPlugin,
        {
          unstable_moduleResolution: { rootDir: fixtureDir, type: 'commonJS' },
        },
      ],
    ],
  }).metadata.stylex;
}

function transformWithAliasedInlineConsts(source, opts = {}) {
  const fixtureDir = path.join(__dirname, '__fixtures__');

  const { code, metadata } = transformSync(source, {
    filename: path.join(fixtureDir, 'main.stylex.js'),
    parserOpts: { sourceType: 'module' },
    babelrc: false,
    plugins: [
      [
        stylexPlugin,
        {
          ...opts,
          aliases: {
            '~fixture/*': [path.join(fixtureDir, '*')],
            ...opts.aliases,
          },
          unstable_moduleResolution: opts.unstable_moduleResolution ?? {
            rootDir: fixtureDir,
            type: 'commonJS',
          },
        },
      ],
    ],
  });

  return { code, metadata };
}

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] stylex.defineConsts()', () => {
    test('constants are unique', () => {
      const { code, metadata } = transform(`
        import stylex from 'stylex';
        export const breakpoints = stylex.defineConsts({ padding: '10px' });
      `);

      const { code: codeDuplicate, metadata: metadataDuplicate } = transform(`
        import stylex from 'stylex';
        export const breakpoints = stylex.defineConsts({ padding: '10px' });
      `);

      // Assert the generated constants are consistent for the same inputs
      expect(code).toEqual(codeDuplicate);
      expect(metadata).toEqual(metadataDuplicate);

      const { code: codeOther, metadata: metadataOther } = transform(`
        import stylex from 'stylex';
        export const breakpoints = stylex.defineConsts({ margin: '10px' });
      `);

      // Assert the generated constants are different for different inputs
      expect(code).not.toEqual(codeOther);
      expect(metadata).not.toEqual(metadataOther);
    });

    test('constants object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          sm: '(min-width: 768px)',
          md: '(min-width: 1024px)',
          lg: '(min-width: 1280px)',
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const breakpoints = {
          sm: "(min-width: 768px)",
          md: "(min-width: 1024px)",
          lg: "(min-width: 1280px)"
        };"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1izlsax",
            {
              "constKey": "x1izlsax",
              "constVal": "(min-width: 768px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "xe5hjsi",
            {
              "constKey": "xe5hjsi",
              "constVal": "(min-width: 1024px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "xmbwnbr",
            {
              "constKey": "xmbwnbr",
              "constVal": "(min-width: 1280px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });

    test('constants object (haste)', () => {
      const options = {
        unstable_moduleResolution: { type: 'haste' },
      };

      const { code, metadata } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          sm: '(min-width: 768px)',
          md: '(min-width: 1024px)',
          lg: '(min-width: 1280px)',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const breakpoints = {
          sm: "(min-width: 768px)",
          md: "(min-width: 1024px)",
          lg: "(min-width: 1280px)"
        };"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1izlsax",
            {
              "constKey": "x1izlsax",
              "constVal": "(min-width: 768px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "xe5hjsi",
            {
              "constKey": "xe5hjsi",
              "constVal": "(min-width: 1024px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "xmbwnbr",
            {
              "constKey": "xmbwnbr",
              "constVal": "(min-width: 1280px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });

    test('constant names: special characters', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const sizes = stylex.defineConsts({
          'font-size*large': '18px',
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const sizes = {
          "font-size*large": "18px"
        };"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x4spo47",
            {
              "constKey": "x4spo47",
              "constVal": "18px",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });

    test('constant names: number', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const levels = stylex.defineConsts({
          1: 'one'
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const levels = {
          "1": "one"
        };"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "xr91grk",
            {
              "constKey": "xr91grk",
              "constVal": "one",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });

    test('constant names: -- prefix preserves user-authored name', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const sizes = stylex.defineConsts({
          '--small': '8px',
          '--large': '24px',
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const sizes = {
          "--small": "8px",
          "--large": "24px"
        };"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "small",
            {
              "constKey": "small",
              "constVal": "8px",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "large",
            {
              "constKey": "large",
              "constVal": "24px",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });
  });

  describe('[transform] stylex.defineConsts() in stylex.create() ', () => {
    test('adds placeholder for constant value from constants.stylex', () => {
      const { code, metadata } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { colors } from './constants.stylex';

        export const styles = stylex.create({
          root: {
            backgroundColor: colors.background,
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        import { colors } from './constants.stylex';
        export const styles = {
          root: {
            kWkggS: "xw8d3ix",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xw8d3ix",
              {
                "ltr": ".xw8d3ix{background-color:var(--x180gk19)}",
                "rtl": null,
              },
              3000,
            ],
          ],
        }
      `);
    });

    test('adds media query placeholder from constants.stylex', () => {
      const { code, metadata } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './constants.stylex';

        export const styles = stylex.create({
          root: {
            color: {
              default: 'red',
              [breakpoints.small]: 'blue',
            },
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './constants.stylex';
        export const styles = {
          root: {
            kMwMTN: "x1e2nbdu xbs0o1n",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1e2nbdu",
              {
                "ltr": ".x1e2nbdu{color:red}",
                "rtl": null,
              },
              3000,
            ],
            [
              "xbs0o1n",
              {
                "ltr": "var(--x1r2wpmh){.xbs0o1n.xbs0o1n{color:blue}}",
                "rtl": null,
              },
              6000,
            ],
          ],
        }
      `);
    });

    test('an imported at-rule constant sorts like the at-rule it names', () => {
      // The constant reaches stylex.create() as its `var(--hash)` placeholder;
      // once processStylexRules knows its value, the rule must sort as if the
      // media query were written inline: the coarse reset before the
      // attribute rule.
      const constants = transformFixtureRules('constants.stylex.js');
      const rules = (condition) => {
        const created = transformFixtureRules(
          'main.stylex.js',
          `
            import * as stylex from '@stylexjs/stylex';
            import { breakpoints } from './constants.stylex';
            export const styles = stylex.create({
              overlay: {
                backgroundImage: {
                  default: null,
                  ':active': {
                    default: 'linear-gradient(red, red)',
                    ${condition}: 'none',
                  },
                  '[data-press="on"]': 'linear-gradient(blue, blue)',
                },
              },
            });
          `,
        );
        return stylexPlugin
          .processStylexRules([...constants, ...created], {
            useLayers: false,
            legacyDisableLayers: true,
          })
          .split('\n')
          .filter((line) => line.includes('background-image'))
          .map((line) => line.replace(/\.x[a-z0-9]+/g, '.x'));
      };

      const constant = rules('[breakpoints.small]');
      expect(constant).toEqual(rules("'@media (max-width: 600px)'"));
      expect(constant).toMatchInlineSnapshot(`
        [
          ".x:active{background-image:linear-gradient(red,red)}",
          "@media (max-width: 600px){.x.x:active{background-image:none}}",
          ".x[data-press="on"]{background-image:linear-gradient(blue,blue)}",
        ]
      `);
    });

    test('variable and theme rules under an imported at-rule constant keep their priorities', () => {
      // defineVars and createTheme count at-rules in their own fractional
      // priorities, which never include the placeholder's share.
      const constants = transformFixtureRules('constants.stylex.js');
      const vars = transformFixtureRules('themeVars.stylex.js');
      const theme = transformFixtureRules(
        'main.stylex.js',
        `
          import * as stylex from '@stylexjs/stylex';
          import { breakpoints } from './constants.stylex';
          import { themeVars } from './themeVars.stylex';
          export const theme = stylex.createTheme(themeVars, {
            accent: { default: 'green', [breakpoints.small]: 'yellow' },
          });
        `,
      );
      const rules = [...constants, ...vars, ...theme];
      expect(stylexPlugin.processStylexRules(rules, { useLayers: true }))
        .toMatchInlineSnapshot(`
        "
        @layer priority1;
        @layer priority1{
        :root, .x1mmh4rn{--x1jwwzce:blue;}
        @media (max-width: 600px){:root, .x1mmh4rn{--x1jwwzce:red;}}
        .x14qsmo4.x14qsmo4, .x14qsmo4.x14qsmo4:root{--x1jwwzce:green;}
        @media (max-width: 600px){.x14qsmo4.x14qsmo4, .x14qsmo4.x14qsmo4:root{--x1jwwzce:yellow;}}
        }"
      `);
      expect(
        stylexPlugin.processStylexRules(rules, {
          useLayers: false,
          legacyDisableLayers: true,
        }),
      ).toMatchInlineSnapshot(`
        ":root, .x1mmh4rn{--x1jwwzce:blue;}
        @media (max-width: 600px){:root, .x1mmh4rn{--x1jwwzce:red;}}
        .x14qsmo4.x14qsmo4, .x14qsmo4.x14qsmo4:root{--x1jwwzce:green;}
        @media (max-width: 600px){.x14qsmo4.x14qsmo4, .x14qsmo4.x14qsmo4:root{--x1jwwzce:yellow;}}"
      `);
    });

    test('resolves /ROOT/ placeholder alias paths for defineConsts imports', () => {
      const fixtureDir = path.join(__dirname, '__fixtures__');
      const projectRoot = path.resolve(__dirname, '../../..');
      const relativeFixturePath = path
        .relative(projectRoot, fixtureDir)
        .replace(/\\/g, '/');
      const rootFixtureGlob = `/ROOT/${relativeFixturePath}/*`;

      const { code, metadata } = transformWithAliasedInlineConsts(
        `
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from '~root-fixture/constants.stylex';

        export const styles = stylex.create({
          root: {
            color: {
              default: 'red',
              [breakpoints.small]: 'blue',
            },
          },
        });
      `,
        {
          aliases: {
            '~root-fixture/*': [rootFixtureGlob],
          },
          unstable_moduleResolution: {
            rootDir: projectRoot,
            type: 'commonJS',
          },
        },
      );

      expect(code).toContain('xbs0o1n');
      expect(metadata.stylex).toEqual(
        expect.arrayContaining([
          expect.arrayContaining([
            'xbs0o1n',
            expect.objectContaining({
              ltr: expect.stringContaining('color:blue'),
            }),
          ]),
        ]),
      );
    });

    test('resolves aliased absolute paths for defineConsts imports', () => {
      const { code, metadata } = transformWithAliasedInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from '~fixture/constants.stylex';

        export const styles = stylex.create({
          root: {
            color: {
              default: 'red',
              [breakpoints.small]: 'blue',
            },
          },
        });
      `);

      expect(code).toContain('xbs0o1n');
      expect(metadata.stylex).toEqual(
        expect.arrayContaining([
          expect.arrayContaining([
            'xbs0o1n',
            expect.objectContaining({
              ltr: expect.stringContaining('color:blue'),
            }),
          ]),
        ]),
      );
    });

    test.skip('works with firstThatWorks', () => {
      const { code } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { colors } from './constants.stylex';

        export const styles = stylex.create({
          nodeEnd: (animationDuration) => ({
            foo: {
              color: stylex.firstThatWorks(colors.background, 'transparent'),
            },
          }),
        });
      `);

      expect(code).toMatchInlineSnapshot(`
            `);
    });

    test('works with dynamic styles constants', () => {
      const { code, metadata } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { colors } from './constants.stylex';

        export const styles = stylex.create({
          node: (padding) => ({
            padding: padding,
            color: colors.background,
          }),
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        import { colors } from './constants.stylex';
        const _temp = {
          kMwMTN: "xy1iwrb",
          "$$css": true
        };
        export const styles = {
          node: padding => [_temp, padding !== undefined && {
            kmVPX3: padding != null ? "x1fozly0" : "",
            $$css: true
          }, {
            "--x-padding": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(padding)
          }]
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1fozly0",
              {
                "ltr": ".x1fozly0{padding:var(--x-padding)}",
                "rtl": null,
              },
              1000,
            ],
            [
              "xy1iwrb",
              {
                "ltr": ".xy1iwrb{color:var(--x180gk19)}",
                "rtl": null,
              },
              3000,
            ],
            [
              "--x-padding",
              {
                "ltr": "@property --x-padding { syntax: "*"; inherits: false;}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('works with dynamic styles at-rules', () => {
      const { code, metadata } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './constants.stylex';

        export const styles = stylex.create({
          node: (color) => ({
            color: {
              [breakpoints.small]: 'blue',
              default: color,
            },
          }),
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './constants.stylex';
        export const styles = {
          node: color => [{
            kMwMTN: "xbs0o1n " + (color != null ? "x3d248p" : ""),
            $$css: true
          }, {
            "--x-4xs81a": color != null ? color : undefined
          }]
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xbs0o1n",
              {
                "ltr": "var(--x1r2wpmh){.xbs0o1n.xbs0o1n{color:blue}}",
                "rtl": null,
              },
              6000,
            ],
            [
              "x3d248p",
              {
                "ltr": ".x3d248p{color:var(--x-4xs81a)}",
                "rtl": null,
              },
              3000,
            ],
            [
              "--x-4xs81a",
              {
                "ltr": "@property --x-4xs81a { syntax: "*"; inherits: false;}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('adds multiple media query placeholders from constants.stylex', () => {
      const { code, metadata } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './constants.stylex';

        export const styles = stylex.create({
          root: {
            color: {
              default: 'red',
              [breakpoints.small]: 'blue',
              [breakpoints.big]: 'yellow',
            },
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './constants.stylex';
        export const styles = {
          root: {
            kMwMTN: "x1e2nbdu xbs0o1n x1ru35j7",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1e2nbdu",
              {
                "ltr": ".x1e2nbdu{color:red}",
                "rtl": null,
              },
              3000,
            ],
            [
              "xbs0o1n",
              {
                "ltr": "var(--x1r2wpmh){.xbs0o1n.xbs0o1n{color:blue}}",
                "rtl": null,
              },
              6000,
            ],
            [
              "x1ru35j7",
              {
                "ltr": "var(--xr4bctk){.x1ru35j7.x1ru35j7{color:yellow}}",
                "rtl": null,
              },
              6000,
            ],
          ],
        }
      `);
    });

    test('adds nested media query placeholders from constants.stylex', () => {
      const { code, metadata } = transformWithInlineConsts(`
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints, colors } from './constants.stylex';

        export const styles = stylex.create({
          root: {
            color: {
              default: 'black',
              [breakpoints.big]: {
                default: colors.red,
                [breakpoints.small]: colors.blue,
              },
            },
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        import { breakpoints, colors } from './constants.stylex';
        export const styles = {
          root: {
            kMwMTN: "x1mqxbix x1iobwbz xrf68et",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1mqxbix",
              {
                "ltr": ".x1mqxbix{color:black}",
                "rtl": null,
              },
              3000,
            ],
            [
              "x1iobwbz",
              {
                "ltr": "var(--xr4bctk){.x1iobwbz.x1iobwbz{color:var(--x1itgfi6)}}",
                "rtl": null,
              },
              6000,
            ],
            [
              "xrf68et",
              {
                "ltr": "var(--x1r2wpmh){var(--xr4bctk){.xrf68et.xrf68et.xrf68et{color:var(--x9g651j)}}}",
                "rtl": null,
              },
              9000,
            ],
          ],
        }
      `);
    });
  });

  describe('[transform] stylex.defineConsts() with runtimeInjection', () => {
    test('constants object with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code, metadata } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          sm: '(min-width: 768px)',
          md: '(min-width: 1024px)',
          lg: '(min-width: 1280px)',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1izlsax",
          constVal: "(min-width: 768px)"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xe5hjsi",
          constVal: "(min-width: 1024px)"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xmbwnbr",
          constVal: "(min-width: 1280px)"
        });
        export const breakpoints = {
          sm: "(min-width: 768px)",
          md: "(min-width: 1024px)",
          lg: "(min-width: 1280px)"
        };"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1izlsax",
            {
              "constKey": "x1izlsax",
              "constVal": "(min-width: 768px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "xe5hjsi",
            {
              "constKey": "xe5hjsi",
              "constVal": "(min-width: 1024px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
          [
            "xmbwnbr",
            {
              "constKey": "xmbwnbr",
              "constVal": "(min-width: 1280px)",
              "ltr": "",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });

    test('numeric constants with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const sizes = stylex.defineConsts({
          small: 8,
          medium: 16,
          large: 24,
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1mllmr4",
          constVal: 8
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1g9nw8d",
          constVal: 16
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1c5h197",
          constVal: 24
        });
        export const sizes = {
          small: 8,
          medium: 16,
          large: 24
        };"
      `);
    });

    test('string constants with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineConsts({
          primary: 'rebeccapurple',
          secondary: 'coral',
          tertiary: 'turquoise',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xbx9tme",
          constVal: "rebeccapurple"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1is3lfz",
          constVal: "coral"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1uyqs0n",
          constVal: "turquoise"
        });
        export const colors = {
          primary: "rebeccapurple",
          secondary: "coral",
          tertiary: "turquoise"
        };"
      `);
    });

    test('mixed string and numeric constants with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const theme = stylex.defineConsts({
          spacing: 16,
          color: 'blue',
          breakpoint: '(min-width: 768px)',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xtp8oqr",
          constVal: 16
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xzwxy2o",
          constVal: "blue"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1dhodo0",
          constVal: "(min-width: 768px)"
        });
        export const theme = {
          spacing: 16,
          color: "blue",
          breakpoint: "(min-width: 768px)"
        };"
      `);
    });

    test('constants with special characters with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const urls = stylex.defineConsts({
          background: 'url("bg.png")',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1abznok",
          constVal: "url(\\"bg.png\\")"
        });
        export const urls = {
          background: "url(\\"bg.png\\")"
        };"
      `);
    });

    test('constants with custom inject path with runtimeInjection', () => {
      const options = {
        runtimeInjection: '@custom/inject-path',
      };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          sm: '(min-width: 768px)',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@custom/inject-path";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1izlsax",
          constVal: "(min-width: 768px)"
        });
        export const breakpoints = {
          sm: "(min-width: 768px)"
        };"
      `);
    });

    test('haste module with runtimeInjection: true', () => {
      const options = {
        unstable_moduleResolution: { type: 'haste' },
        runtimeInjection: true,
      };

      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          sm: '(min-width: 768px)',
          md: '(min-width: 1024px)',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1izlsax",
          constVal: "(min-width: 768px)"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xe5hjsi",
          constVal: "(min-width: 1024px)"
        });
        export const breakpoints = {
          sm: "(min-width: 768px)",
          md: "(min-width: 1024px)"
        };"
      `);
    });

    test('constants with numeric keys with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const levels = stylex.defineConsts({
          0: 'zero',
          1: 'one',
          2: 'two',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1t8zjeu",
          constVal: "zero"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xr91grk",
          constVal: "one"
        });
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x5diukc",
          constVal: "two"
        });
        export const levels = {
          "0": "zero",
          "1": "one",
          "2": "two"
        };"
      `);
    });

    test('multiple defineConsts calls with runtimeInjection: true', () => {
      const options = { runtimeInjection: true };
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          sm: '(min-width: 768px)',
        });
        export const colors = stylex.defineConsts({
          primary: 'blue',
        });
      `,
        options,
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "x1izlsax",
          constVal: "(min-width: 768px)"
        });
        export const breakpoints = {
          sm: "(min-width: 768px)"
        };
        _inject2({
          ltr: "",
          priority: 0,
          constKey: "xbx9tme",
          constVal: "blue"
        });
        export const colors = {
          primary: "blue"
        };"
      `);
    });
  });
});
