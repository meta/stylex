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
    plugins: [[stylexPlugin, { ...opts }]],
  });

  return { code, metadata };
}

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] stylex.create()', () => {
    describe('static styles', () => {
      test('unused style object', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              backgroundColor: 'red',
              color: 'blue',
            }
          });
        `);
        expect(code).toMatchInlineSnapshot(
          '"import * as stylex from \'@stylexjs/stylex\';"',
        );
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrkmrrc",
                {
                  "ltr": ".xrkmrrc{background-color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "xju2f9n",
                {
                  "ltr": ".xju2f9n{color:blue}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('style object', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              backgroundColor: 'red',
              color: 'blue',
            }
          });
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              kWkggS: "xrkmrrc",
              kMwMTN: "xju2f9n",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrkmrrc",
                {
                  "ltr": ".xrkmrrc{background-color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "xju2f9n",
                {
                  "ltr": ".xju2f9n{color:blue}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env resolves compile-time constants', () => {
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              color: stylex.env.brandPrimary,
            }
          });
        `,
          { env: { brandPrimary: '#123456' } },
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              kMwMTN: "x1tfn4g9",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x1tfn4g9",
                {
                  "ltr": ".x1tfn4g9{color:#123456}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env named import resolves compile-time constants', () => {
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          import { env } from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              color: env.brandPrimary,
            }
          });
        `,
          { env: { brandPrimary: '#654321' } },
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          import { env } from '@stylexjs/stylex';
          export const styles = {
            root: {
              kMwMTN: "xa6cz37",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xa6cz37",
                {
                  "ltr": ".xa6cz37{color:#654321}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env destructured import resolves compile-time constants', () => {
        const { code, metadata } = transform(
          `
          import {create, env} from '@stylexjs/stylex';
          export const styles = create({
            root: {
              color: env.brandPrimary,
            }
          });
        `,
          { env: { brandPrimary: '#123456' } },
        );
        expect(code).toMatchInlineSnapshot(`
          "import { create, env } from '@stylexjs/stylex';
          export const styles = {
            root: {
              kMwMTN: "x1tfn4g9",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x1tfn4g9",
                {
                  "ltr": ".x1tfn4g9{color:#123456}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env function call resolves at compile time', () => {
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              color: stylex.env.colorMix('red', 'blue', 50),
            }
          });
        `,
          {
            env: {
              colorMix: (c1, c2, pct) =>
                `color-mix(in srgb, ${c1} ${pct}%, ${c2})`,
            },
          },
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              kMwMTN: "x10zuzju",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x10zuzju",
                {
                  "ltr": ".x10zuzju{color:color-mix(in srgb,red 50%,blue)}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env named import function call resolves at compile time', () => {
        const { code, metadata } = transform(
          `
          import { create, env } from '@stylexjs/stylex';
          export const styles = create({
            root: {
              color: env.colorMix('red', 'blue', 50),
            }
          });
        `,
          {
            env: {
              colorMix: (c1, c2, pct) =>
                `color-mix(in srgb, ${c1} ${pct}%, ${c2})`,
            },
          },
        );
        expect(code).toMatchInlineSnapshot(`
          "import { create, env } from '@stylexjs/stylex';
          export const styles = {
            root: {
              kMwMTN: "x10zuzju",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x10zuzju",
                {
                  "ltr": ".x10zuzju{color:color-mix(in srgb,red 50%,blue)}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env function using template literals', () => {
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              boxShadow: stylex.env.shadow('black', 0.35),
            }
          });
        `,
          {
            env: {
              shadow: (color, opacity) =>
                `0 4px 4px 2px color-mix(in srgb, ${color} ${opacity * 100}%, transparent)`,
            },
          },
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              kGVxlE: "xft59df",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xft59df",
                {
                  "ltr": ".xft59df{box-shadow:0 4px 4px 2px color-mix(in srgb,black 35%,transparent)}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('stylex.env function with multiple properties', () => {
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              color: stylex.env.opacity('red', 0.5),
              backgroundColor: stylex.env.opacity('blue', 0.8),
            }
          });
        `,
          {
            env: {
              opacity: (color, pct) =>
                `color-mix(in srgb, ${color} ${pct * 100}%, transparent)`,
            },
          },
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              kMwMTN: "xa1gjp6",
              kWkggS: "xuy6j5x",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xa1gjp6",
                {
                  "ltr": ".xa1gjp6{color:color-mix(in srgb,red 50%,transparent)}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "xuy6j5x",
                {
                  "ltr": ".xuy6j5x{background-color:color-mix(in srgb,blue 80%,transparent)}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('nested referenced style object', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          function fooBar() {
            const styles = stylex.create({
              root: {
                backgroundColor: 'red',
                color: 'blue',
              }
            });
            console.log(styles);
          }
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          const _styles = {
            root: {
              kWkggS: "xrkmrrc",
              kMwMTN: "xju2f9n",
              $$css: true
            }
          };
          function fooBar() {
            const styles = _styles;
            console.log(styles);
          }"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrkmrrc",
                {
                  "ltr": ".xrkmrrc{background-color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "xju2f9n",
                {
                  "ltr": ".xju2f9n{color:blue}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('multiple nested referenced style object', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          function fooBar() {
            const styles = stylex.create({
              root: {
                backgroundColor: 'red',
                color: 'blue',
              }
            });
            const styles2 = stylex.create({
              root: {
                backgroundColor: 'blue',
                color: 'green',
              }
            });
            console.log(styles);
            console.log(styles2);
          }
          const otherFunction = () => {
            const styles3 = stylex.create({
              root: {
                backgroundColor: 'green',
                color: 'red',
              }
            });
            console.log(styles3);
          }
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          const _styles = {
            root: {
              kWkggS: "xrkmrrc",
              kMwMTN: "xju2f9n",
              $$css: true
            }
          };
          const _styles2 = {
            root: {
              kWkggS: "x1t391ir",
              kMwMTN: "x1prwzq3",
              $$css: true
            }
          };
          function fooBar() {
            const styles = _styles;
            const styles2 = _styles2;
            console.log(styles);
            console.log(styles2);
          }
          const _styles3 = {
            root: {
              kWkggS: "x1u857p9",
              kMwMTN: "x1e2nbdu",
              $$css: true
            }
          };
          const otherFunction = () => {
            const styles3 = _styles3;
            console.log(styles3);
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrkmrrc",
                {
                  "ltr": ".xrkmrrc{background-color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "xju2f9n",
                {
                  "ltr": ".xju2f9n{color:blue}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1t391ir",
                {
                  "ltr": ".x1t391ir{background-color:blue}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1prwzq3",
                {
                  "ltr": ".x1prwzq3{color:green}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1u857p9",
                {
                  "ltr": ".x1u857p9{background-color:green}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1e2nbdu",
                {
                  "ltr": ".x1e2nbdu{color:red}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('style object (multiple)', () => {
        // Check multiple objects and different key types
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              backgroundColor: 'red',
            },
            other: {
              color: 'blue',
            },
            'bar-baz': {
              color: 'green',
            },
            1: {
              color: 'blue',
            },
            [2]: {
              color: 'purple',
            },
          });
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            "1": {
              kMwMTN: "xju2f9n",
              $$css: true
            },
            "2": {
              kMwMTN: "x125ip1n",
              $$css: true
            },
            root: {
              kWkggS: "xrkmrrc",
              $$css: true
            },
            other: {
              kMwMTN: "xju2f9n",
              $$css: true
            },
            "bar-baz": {
              kMwMTN: "x1prwzq3",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xju2f9n",
                {
                  "ltr": ".xju2f9n{color:blue}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x125ip1n",
                {
                  "ltr": ".x125ip1n{color:purple}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "xrkmrrc",
                {
                  "ltr": ".xrkmrrc{background-color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1prwzq3",
                {
                  "ltr": ".x1prwzq3{color:green}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('style object with custom properties', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              '--background-color': 'red',
              '--otherColor': 'green',
              '--foo': 10
            }
          });
        `);
        // Must not modify casing of custom properties
        // Must not add units to unitless values
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              "--background-color": "xgau0yw",
              "--otherColor": "x1p9b6ba",
              "--foo": "x40g909",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xgau0yw",
                {
                  "ltr": ".xgau0yw{--background-color:red}",
                  "rtl": null,
                },
                1,
              ],
              [
                "x1p9b6ba",
                {
                  "ltr": ".x1p9b6ba{--otherColor:green}",
                  "rtl": null,
                },
                1,
              ],
              [
                "x40g909",
                {
                  "ltr": ".x40g909{--foo:10}",
                  "rtl": null,
                },
                1,
              ],
            ],
          }
        `);
      });

      // TODO: eventually these multi-value shortforms should not be allowed
      // Requires Meta migration to be completed.
      test('style object with shortform properties', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          const borderRadius = 2;
          export const styles = stylex.create({
            error: {
              borderColor: 'red blue',
              borderStyle: 'dashed solid',
              borderWidth: '0 0 2px 0',
              margin: 'calc((100% - 50px) * 0.5) 20px 0',
              padding: 'calc((100% - 50px) * 0.5) var(--rightpadding, 20px)',
            },
            short: {
              borderBottomWidth: '5px',
              borderBottomStyle: 'solid',
              borderBottomColor: 'red',
              borderColor: 'var(--divider)',
              borderRadius: borderRadius * 2,
              borderStyle: 'solid',
              borderWidth: 1,
              marginTop: 'calc((100% - 50px) * 0.5)',
              marginRight: 20,
              marginBottom: 0,
              paddingTop: 0,
            },
          });
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          const borderRadius = 2;
          export const styles = {
            error: {
              kVAM5u: "xs4buau",
              ksu8eU: "xn06r42",
              kMzoRj: "xn43iik",
              kogj98: "xe4njm9",
              kmVPX3: "x1lmef92",
              $$css: true
            },
            short: {
              kt9PQ7: "xa309fb",
              kfdmCh: "x1q0q8m5",
              kL6WhQ: "xud65wk",
              kVAM5u: "x1lh7sze",
              kaIpWk: "x12oqio5",
              ksu8eU: "x1y0btm7",
              kMzoRj: "xmkeg23",
              keoZOQ: "xxsse2n",
              km5ZXQ: "x1wh8b8d",
              k1K539: "xat24cr",
              kLKAdn: "xexx8yu",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xs4buau",
                {
                  "ltr": ".xs4buau{border-color:red blue}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xn06r42",
                {
                  "ltr": ".xn06r42{border-style:dashed solid}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xn43iik",
                {
                  "ltr": ".xn43iik{border-width:0 0 2px 0}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xe4njm9",
                {
                  "ltr": ".xe4njm9{margin:calc((100% - 50px) * .5) 20px 0}",
                  "rtl": null,
                },
                1000,
              ],
              [
                "x1lmef92",
                {
                  "ltr": ".x1lmef92{padding:calc((100% - 50px) * .5) var(--rightpadding,20px)}",
                  "rtl": null,
                },
                1000,
              ],
              [
                "xa309fb",
                {
                  "ltr": ".xa309fb{border-bottom-width:5px}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "x1q0q8m5",
                {
                  "ltr": ".x1q0q8m5{border-bottom-style:solid}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "xud65wk",
                {
                  "ltr": ".xud65wk{border-bottom-color:red}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "x1lh7sze",
                {
                  "ltr": ".x1lh7sze{border-color:var(--divider)}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "x12oqio5",
                {
                  "ltr": ".x12oqio5{border-radius:4px}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "x1y0btm7",
                {
                  "ltr": ".x1y0btm7{border-style:solid}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xmkeg23",
                {
                  "ltr": ".xmkeg23{border-width:1px}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xxsse2n",
                {
                  "ltr": ".xxsse2n{margin-top:calc((100% - 50px) * .5)}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "x1wh8b8d",
                {
                  "ltr": ".x1wh8b8d{margin-right:20px}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "xat24cr",
                {
                  "ltr": ".xat24cr{margin-bottom:0}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "xexx8yu",
                {
                  "ltr": ".xexx8yu{padding-top:0}",
                  "rtl": null,
                },
                4000,
              ],
            ],
          }
        `);
      });

      test('style object with shortform properties (styleResolution: "property-specificity")', () => {
        const options = {
          runtimeInjection: false,
          styleResolution: 'property-specificity',
        };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const borderRadius = 2;
          export const styles = stylex.create({
            error: {
              borderColor: 'red blue',
              borderStyle: 'dashed solid',
              borderWidth: '0 0 2px 0',
              margin: 'calc((100% - 50px) * 0.5) 20px 0',
              padding: 'calc((100% - 50px) * 0.5) var(--rightpadding, 20px)',
            },
            short: {
              borderBottomWidth: '5px',
              borderBottomStyle: 'solid',
              borderBottomColor: 'red',
              borderColor: 'var(--divider)',
              borderRadius: borderRadius * 2,
              borderStyle: 'solid',
              borderWidth: 1,
              marginTop: 'calc((100% - 50px) * 0.5)',
              marginRight: 20,
              marginBottom: 0,
              paddingTop: 0,
            },
          });
        `,
          options,
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          const borderRadius = 2;
          export const styles = {
            error: {
              kVAM5u: "xs4buau",
              ksu8eU: "xn06r42",
              kMzoRj: "xn43iik",
              kogj98: "xe4njm9",
              kmVPX3: "x1lmef92",
              $$css: true
            },
            short: {
              kt9PQ7: "xa309fb",
              kfdmCh: "x1q0q8m5",
              kL6WhQ: "xud65wk",
              kVAM5u: "x1lh7sze",
              kaIpWk: "x12oqio5",
              ksu8eU: "x1y0btm7",
              kMzoRj: "xmkeg23",
              keoZOQ: "xxsse2n",
              km5ZXQ: "x1wh8b8d",
              k1K539: "xat24cr",
              kLKAdn: "xexx8yu",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xs4buau",
                {
                  "ltr": ".xs4buau{border-color:red blue}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xn06r42",
                {
                  "ltr": ".xn06r42{border-style:dashed solid}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xn43iik",
                {
                  "ltr": ".xn43iik{border-width:0 0 2px 0}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xe4njm9",
                {
                  "ltr": ".xe4njm9{margin:calc((100% - 50px) * .5) 20px 0}",
                  "rtl": null,
                },
                1000,
              ],
              [
                "x1lmef92",
                {
                  "ltr": ".x1lmef92{padding:calc((100% - 50px) * .5) var(--rightpadding,20px)}",
                  "rtl": null,
                },
                1000,
              ],
              [
                "xa309fb",
                {
                  "ltr": ".xa309fb{border-bottom-width:5px}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "x1q0q8m5",
                {
                  "ltr": ".x1q0q8m5{border-bottom-style:solid}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "xud65wk",
                {
                  "ltr": ".xud65wk{border-bottom-color:red}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "x1lh7sze",
                {
                  "ltr": ".x1lh7sze{border-color:var(--divider)}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "x12oqio5",
                {
                  "ltr": ".x12oqio5{border-radius:4px}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "x1y0btm7",
                {
                  "ltr": ".x1y0btm7{border-style:solid}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xmkeg23",
                {
                  "ltr": ".xmkeg23{border-width:1px}",
                  "rtl": null,
                },
                2000,
              ],
              [
                "xxsse2n",
                {
                  "ltr": ".xxsse2n{margin-top:calc((100% - 50px) * .5)}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "x1wh8b8d",
                {
                  "ltr": ".x1wh8b8d{margin-right:20px}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "xat24cr",
                {
                  "ltr": ".xat24cr{margin-bottom:0}",
                  "rtl": null,
                },
                4000,
              ],
              [
                "xexx8yu",
                {
                  "ltr": ".xexx8yu{padding-top:0}",
                  "rtl": null,
                },
                4000,
              ],
            ],
          }
        `);
      });

      test('style object requiring vendor prefixes', () => {
        // TODO: add more vendor-prefixed properties and values
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: {
              userSelect: 'none',
            },
          });
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: {
              kfSwDN: "x87ps6o",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x87ps6o",
                {
                  "ltr": ".x87ps6o{user-select:none}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      describe('simple values', () => {
        test('set custom property', () => {
          const options = {
            filename: 'MyComponent.js',
            unstable_moduleResolution: { type: 'haste' },
          };
          const { code, metadata } = transform(
            `
            import * as stylex from '@stylexjs/stylex';
            import {vars} from 'vars.stylex.js';

            export const styles = stylex.create({
              root: {
                [vars.foo]: 500,
              },
            });
          `,
            options,
          );
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            import { vars } from 'vars.stylex.js';
            export const styles = {
              root: {
                "--x1mxfvjx": "x1pojaxg",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1pojaxg",
                  {
                    "ltr": ".x1pojaxg{--x1mxfvjx:500}",
                    "rtl": null,
                  },
                  1,
                ],
              ],
            }
          `);
        });

        test('set "transitionProperty"', () => {
          const camelCased = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                transitionProperty: 'marginTop',
              },
            });
          `);
          expect(camelCased.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                k1ekBW: "x1cfch2b",
                $$css: true
              }
            };"
          `);
          expect(camelCased.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1cfch2b",
                  {
                    "ltr": ".x1cfch2b{transition-property:margin-top}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);

          const kebabCased = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                transitionProperty: 'margin-top',
              },
            });
          `);
          expect(kebabCased.code).toEqual(camelCased.code);
          expect(kebabCased.metadata).toEqual(kebabCased.metadata);

          const customProperty = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                transitionProperty: '--foo',
              },
            });
          `);
          expect(customProperty.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                k1ekBW: "x17389it",
                $$css: true
              }
            };"
          `);
          expect(customProperty.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x17389it",
                  {
                    "ltr": ".x17389it{transition-property:--foo}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);

          const multiProperty = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              one: {
                transitionProperty: 'opacity, insetInlineStart',
              },
              two: {
                transitionProperty: 'opacity, inset-inline-start',
              },
            });
          `);
          expect(multiProperty.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              one: {
                k1ekBW: "xh6nlrc",
                $$css: true
              },
              two: {
                k1ekBW: "xh6nlrc",
                $$css: true
              }
            };"
          `);
          expect(multiProperty.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xh6nlrc",
                  {
                    "ltr": ".xh6nlrc{transition-property:opacity,inset-inline-start}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('set "willChange"', () => {
          const camelCased = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                willChange: 'insetInlineStart',
              },
            });
          `);
          expect(camelCased.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                k6sLGO: "x1n5prqt",
                $$css: true
              }
            };"
          `);
          expect(camelCased.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1n5prqt",
                  {
                    "ltr": ".x1n5prqt{will-change:inset-inline-start}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);

          const kebabCased = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                willChange: 'inset-inline-start',
              },
            });
          `);
          expect(kebabCased.code).toEqual(camelCased.code);
          expect(kebabCased.metadata).toEqual(kebabCased.metadata);

          const customProperty = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                willChange: '--foo',
              },
            });
          `);
          expect(customProperty.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                k6sLGO: "x1lxaxzv",
                $$css: true
              }
            };"
          `);
          expect(customProperty.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1lxaxzv",
                  {
                    "ltr": ".x1lxaxzv{will-change:--foo}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);

          const multiProperty = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              one: {
                willChange: 'opacity, insetInlineStart',
              },
              two: {
                willChange: 'opacity, inset-inline-start',
              }
            });
          `);
          expect(multiProperty.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              one: {
                k6sLGO: "x30a982",
                $$css: true
              },
              two: {
                k6sLGO: "x30a982",
                $$css: true
              }
            };"
          `);
          expect(multiProperty.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x30a982",
                  {
                    "ltr": ".x30a982{will-change:opacity,inset-inline-start}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);

          const keyword = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                willChange: 'scroll-position'
              }
            });
          `);
          expect(keyword.code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                k6sLGO: "x1q5hf6d",
                $$css: true
              }
            };"
          `);
          expect(keyword.metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1q5hf6d",
                  {
                    "ltr": ".x1q5hf6d{will-change:scroll-position}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('use "attr()" function', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                content: 'attr(some-attribute)',
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kah6P1: "xd71okc",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xd71okc",
                  {
                    "ltr": ".xd71okc{content:attr(some-attribute)}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('use array (fallbacks)', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                position: ['sticky', 'fixed']
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kVAEAm: "x1ruww2u",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1ruww2u",
                  {
                    "ltr": ".x1ruww2u{position:sticky;position:fixed}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('use CSS variable', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: 'var(--background-color)',
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "xn9heto",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xn9heto",
                  {
                    "ltr": ".xn9heto{background-color:var(--background-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('use string containing CSS variables', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                boxShadow: '0px 2px 4px var(--shadow-1)',
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kGVxlE: "xxnfx33",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xxnfx33",
                  {
                    "ltr": ".xxnfx33{box-shadow:0 2px 4px var(--shadow-1)}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });
      });

      describe('function value: stylex.firstThatWorks()', () => {
        // Check various combinations of fallbacks
        test('args: value, value', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                position: stylex.firstThatWorks('sticky', 'fixed'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kVAEAm: "x15oojuh",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x15oojuh",
                  {
                    "ltr": ".x15oojuh{position:fixed;position:sticky}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('args: value, var', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('red', 'var(--color)'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x1nv2f59",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1nv2f59",
                  {
                    "ltr": ".x1nv2f59{color:var(--color);color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('args: var, value', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('var(--color)', 'red'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x8nmrrw",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x8nmrrw",
                  {
                    "ltr": ".x8nmrrw{color:var(--color,red)}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('args: var, var', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('var(--color)', 'var(--otherColor)'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x1775bb3",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1775bb3",
                  {
                    "ltr": ".x1775bb3{color:var(--color,var(--otherColor))}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('args: var, var, var', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('var(--color)', 'var(--secondColor)', 'var(--thirdColor)'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "xsrkhny",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xsrkhny",
                  {
                    "ltr": ".xsrkhny{color:var(--color,var(--secondColor,var(--thirdColor)))}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('args: func, var, value', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('color-mix(in srgb, currentColor 20%, transparent)', 'var(--color)', 'red'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x8vgp76",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x8vgp76",
                  {
                    "ltr": ".x8vgp76{color:var(--color,red);color:color-mix(in srgb,currentColor 20%,transparent)}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('args: func, var, value, value', () => {
          // Ignore simple fallbacks after the first one
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: stylex.firstThatWorks('color-mix(in srgb, currentColor 20%, transparent)', 'var(--color)', 'red', 'green'),
              }
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x8vgp76",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x8vgp76",
                  {
                    "ltr": ".x8vgp76{color:var(--color,red);color:color-mix(in srgb,currentColor 20%,transparent)}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });
      });

      describe.skip('function value: stylex.types.*()', () => {
        // TODO: port tests from "stylex-types-test.js" in "shared"
      });

      describe('object values: pseudo-classes', () => {
        test('invalid pseudo-class', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: {
                  ':invalidpseudo': 'blue'
                },
              },
            });
          `);
          // TODO: this should either fail or guarantee an insertion
          // order relative to valid pseudo-classes
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x1qo2jjy",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1qo2jjy",
                  {
                    "ltr": ".x1qo2jjy:invalidpseudo{color:blue}",
                    "rtl": null,
                  },
                  3040,
                ],
              ],
            }
          `);
        });

        test('valid pseudo-class', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: {
                  ':hover': 'red',
                },
                color: {
                  ':hover': 'blue',
                }
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "x1gykpug",
                kMwMTN: "x17z2mba",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1gykpug",
                  {
                    "ltr": ".x1gykpug:hover{background-color:red}",
                    "rtl": null,
                  },
                  3130,
                ],
                [
                  "x17z2mba",
                  {
                    "ltr": ".x17z2mba:hover{color:blue}",
                    "rtl": null,
                  },
                  3130,
                ],
              ],
            }
          `);
        });

        test('pseudo-class generated order', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: {
                  ':hover': 'blue',
                  ':active':'red',
                  ':focus': 'yellow',
                  ':nth-child(2n)': 'purple',
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x17z2mba x96fq8s x1wvtd7d x126ychx",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x17z2mba",
                  {
                    "ltr": ".x17z2mba:hover{color:blue}",
                    "rtl": null,
                  },
                  3130,
                ],
                [
                  "x96fq8s",
                  {
                    "ltr": ".x96fq8s:active{color:red}",
                    "rtl": null,
                  },
                  3170,
                ],
                [
                  "x1wvtd7d",
                  {
                    "ltr": ".x1wvtd7d:focus{color:yellow}",
                    "rtl": null,
                  },
                  3150,
                ],
                [
                  "x126ychx",
                  {
                    "ltr": ".x126ychx:nth-child(2n){color:purple}",
                    "rtl": null,
                  },
                  3060,
                ],
              ],
            }
          `);
        });

        test('pseudo-class generated order (nested, same value)', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: {
                  ':hover': {
                    ':active':'red',
                  },
                  ':active': {
                    ':hover':'red',
                  },
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "xa2ikkt",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xa2ikkt",
                  {
                    "ltr": ".xa2ikkt:active:hover{color:red}",
                    "rtl": null,
                  },
                  3300,
                ],
              ],
            }
          `);
        });

        test('pseudo-class generated order (nested, different value)', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: {
                  ':hover': {
                    ':active':'red',
                  },
                  ':active': {
                    ':hover':'green',
                  },
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "xa2ikkt x13pwkn",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xa2ikkt",
                  {
                    "ltr": ".xa2ikkt:active:hover{color:red}",
                    "rtl": null,
                  },
                  3300,
                ],
                [
                  "x13pwkn",
                  {
                    "ltr": ".x13pwkn:active:hover{color:green}",
                    "rtl": null,
                  },
                  3300,
                ],
              ],
            }
          `);
        });

        test('attribute selector with pseudo-class (nested, same value)', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                color: {
                  ':hover': {
                    '[data-state="open"]': 'red',
                  },
                  '[data-state="open"]': {
                    ':hover': 'red',
                  },
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kMwMTN: "x113j3rq",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x113j3rq",
                  {
                    "ltr": ".x113j3rq:hover[data-state="open"]{color:red}",
                    "rtl": null,
                  },
                  6130,
                ],
              ],
            }
          `);
        });

        test('pseudo-class with array fallbacks', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                position: {
                  ':hover': ['sticky', 'fixed'],
                }
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kVAEAm: "x1nxcus0",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1nxcus0",
                  {
                    "ltr": ".x1nxcus0:hover{position:sticky;position:fixed}",
                    "rtl": null,
                  },
                  3130,
                ],
              ],
            }
          `);
        });
      });

      describe('object values: pseudo-elements', () => {
        test('"::before" and "::after"', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: {
                '::before': {
                  color: 'red'
                },
                '::after': {
                  color: 'blue'
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: {
                kxBb7d: "x16oeupf",
                kB1Fuz: "xdaarc3",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x16oeupf",
                  {
                    "ltr": ".x16oeupf::before{color:red}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "xdaarc3",
                  {
                    "ltr": ".xdaarc3::after{color:blue}",
                    "rtl": null,
                  },
                  8000,
                ],
              ],
            }
          `);
        });

        test('"::placeholder"', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: {
                '::placeholder': {
                  color: 'gray',
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: {
                k8Qsv1: "x6yu8oj",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x6yu8oj",
                  {
                    "ltr": ".x6yu8oj::placeholder{color:gray}",
                    "rtl": null,
                  },
                  8000,
                ],
              ],
            }
          `);
        });

        test('"::thumb"', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: {
                '::thumb': {
                  width: 16,
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: {
                k8pbKx: "x1en94km",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1en94km",
                  {
                    "ltr": ".x1en94km::-webkit-slider-thumb, .x1en94km::-moz-range-thumb, .x1en94km::-ms-thumb{width:16px}",
                    "rtl": null,
                  },
                  9000,
                ],
              ],
            }
          `);
        });

        test('"::before" containing pseudo-classes', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: {
                '::before': {
                  color: {
                    default: 'red',
                    ':hover': 'blue',
                  }
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: {
                kxBb7d: "x16oeupf xeb2lg0",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x16oeupf",
                  {
                    "ltr": ".x16oeupf::before{color:red}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "xeb2lg0",
                  {
                    "ltr": ".xeb2lg0:hover::before{color:blue}",
                    "rtl": null,
                  },
                  8130,
                ],
              ],
            }
          `);
        });

        test('legacy compound ":hover::after" selector as single key', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: {
                ':hover::after': {
                  color: 'red',
                },
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: {
                kF1atM: "x1gfyp89",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1gfyp89",
                  {
                    "ltr": ".x1gfyp89:hover::after{color:red}",
                    "rtl": null,
                  },
                  8130,
                ],
              ],
            }
          `);
        });

        test('compound ":hover::after" selector as single key', () => {
          const { metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: {
                color: {
                  default: 'red',
                  ':hover::after': 'blue',
                },
              },
            });
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
                  "x6wc952",
                  {
                    "ltr": ".x6wc952:hover::after{color:blue}",
                    "rtl": null,
                  },
                  8130,
                ],
              ],
            }
          `);
        });

        test('"::after" with multiple pseudo-class conditions', () => {
          const { metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              button: {
                '::after': {
                  content: '""',
                  boxShadow: {
                    default: '0 0 0 1px gray',
                    ':hover': '0 0 0 1px blue',
                    ':active': '0 0 0 1px darkblue',
                  },
                },
              },
            });
          `);

          // Pseudo-classes (:hover, :active) must come before the
          // pseudo-element (::after) in the selector for valid CSS.
          const rules = metadata.stylex.map(([_className, { ltr }]) => ltr);
          expect(rules).toMatchInlineSnapshot(`
            [
              ".x1s928wv::after{content:""}",
              ".x5fy8b7::after{box-shadow:0 0 0 1px gray}",
              ".xgazanr:hover::after{box-shadow:0 0 0 1px blue}",
              ".x136huz6:active::after{box-shadow:0 0 0 1px darkblue}",
            ]
          `);
        });
      });

      describe('object values: queries', () => {
        test('media queries without last query wins', () => {
          const { code, metadata } = transform(
            `
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: {
                  default: 'red',
                  '@media (max-width: 900px)': 'blue',
                  '@media (max-width: 500px)': 'purple',
                  '@media (max-width: 400px)': 'green',
                }
              },
            });
          `,
            { enableMediaQueryOrder: true },
          );
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "xrkmrrc xdm03ys xb3e2qq x856a2w",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xdm03ys",
                  {
                    "ltr": "@media (min-width: 500.01px) and (max-width: 900px){.xdm03ys.xdm03ys{background-color:blue}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "xb3e2qq",
                  {
                    "ltr": "@media (min-width: 400.01px) and (max-width: 500px){.xb3e2qq.xb3e2qq{background-color:purple}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x856a2w",
                  {
                    "ltr": "@media (max-width: 400px){.x856a2w.x856a2w{background-color:green}}",
                    "rtl": null,
                  },
                  3200,
                ],
              ],
            }
          `);
        });

        test('media queries without last query wins', () => {
          const { code, metadata } = transform(
            `
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: {
                  default: 'red',
                  '@media screen and (max-width: 900px)': 'blue',
                  '@media screen and (max-width: 500px)': 'purple',
                  '@media screen and (max-width: 400px)': 'green',
                }
              },
            });
          `,
            { enableMediaQueryOrder: true },
          );
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "xrkmrrc xa5oah1 x1seukri xduqtph",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xa5oah1",
                  {
                    "ltr": "@media screen and (max-width: 900px) and (not (max-width: 500px)) and (not (max-width: 400px)){.xa5oah1.xa5oah1{background-color:blue}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x1seukri",
                  {
                    "ltr": "@media screen and (max-width: 500px) and (not (max-width: 400px)){.x1seukri.x1seukri{background-color:purple}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "xduqtph",
                  {
                    "ltr": "@media screen and (max-width: 400px){.xduqtph.xduqtph{background-color:green}}",
                    "rtl": null,
                  },
                  3200,
                ],
              ],
            }
          `);
        });

        test('media queries without last query wins', () => {
          const { code, metadata } = transform(
            `
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: {
                  default: 'red',
                  '@media (max-width: 900px)': 'blue',
                  '@media (max-width: 500px)': 'purple',
                  '@media (max-width: 400px)': 'green',
                }
              },
            });
          `,
            { enableMediaQueryOrder: false },
          );
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "xrkmrrc xn8cmr1 x1lr89ez x856a2w",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xn8cmr1",
                  {
                    "ltr": "@media (max-width: 900px){.xn8cmr1.xn8cmr1{background-color:blue}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x1lr89ez",
                  {
                    "ltr": "@media (max-width: 500px){.x1lr89ez.x1lr89ez{background-color:purple}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x856a2w",
                  {
                    "ltr": "@media (max-width: 400px){.x856a2w.x856a2w{background-color:green}}",
                    "rtl": null,
                  },
                  3200,
                ],
              ],
            }
          `);
        });

        test('media queries', () => {
          const { code, metadata } = transform(
            `
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: {
                  default: 'red',
                  '@media (min-width: 1000px)': 'blue',
                  '@media (min-width: 2000px)': 'purple',
                }
              },
            });
          `,
          );
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "xrkmrrc xr725a3 x1ssfqz5",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xr725a3",
                  {
                    "ltr": "@media (min-width: 1000px) and (max-width: 1999.98px){.xr725a3.xr725a3{background-color:blue}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x1ssfqz5",
                  {
                    "ltr": "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
                    "rtl": null,
                  },
                  3200,
                ],
              ],
            }
          `);
        });

        test('supports queries', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                backgroundColor: {
                  default:'red',
                  '@supports (hover: hover)': 'blue',
                  '@supports not (hover: hover)': 'purple',
                }
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kWkggS: "xrkmrrc x6m3b6q x6um648",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x6m3b6q",
                  {
                    "ltr": "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
                    "rtl": null,
                  },
                  3030,
                ],
                [
                  "x6um648",
                  {
                    "ltr": "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
                    "rtl": null,
                  },
                  3030,
                ],
              ],
            }
          `);
        });

        test('media query with pseudo-classes', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: {
                fontSize: {
                  default: '1rem',
                  '@media (min-width: 800px)': {
                    default: '2rem',
                    ':hover': '2.2rem'
                  }
                }
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: {
                kGuDYH: "x1jchvi3 x1w3nbkt xicay7j",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1jchvi3",
                  {
                    "ltr": ".x1jchvi3{font-size:1rem}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x1w3nbkt",
                  {
                    "ltr": "@media (min-width: 800px){.x1w3nbkt.x1w3nbkt{font-size:2rem}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "xicay7j",
                  {
                    "ltr": "@media (min-width: 800px){.xicay7j.xicay7j:hover{font-size:2.2rem}}",
                    "rtl": null,
                  },
                  3330,
                ],
              ],
            }
          `);
        });

        test('media query with array fallbacks', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              default: {
                position: {
                  default: 'fixed',
                  '@media (min-width: 768px)': ['sticky', 'fixed'],
                }
              },
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              default: {
                kVAEAm: "xixxii4 x1vazst0",
                $$css: true
              }
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xixxii4",
                  {
                    "ltr": ".xixxii4{position:fixed}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x1vazst0",
                  {
                    "ltr": "@media (min-width: 768px){.x1vazst0.x1vazst0{position:sticky;position:fixed}}",
                    "rtl": null,
                  },
                  3200,
                ],
              ],
            }
          `);
        });
      });
    });

    describe('dynamic styles', () => {
      describe('safe conditional checks', () => {
        test('template literal expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: \`\${color}\`,
                color: \`\${color}px\`,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {
                "--x-backgroundColor": \`\${color}\` != null ? \`\${color}\` : undefined,
                "--x-color": \`\${color}px\` != null ? \`\${color}px\` : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('binary expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (width, height) => ({
                width: width + 100,
                height: height * 2,
                margin: width - 50,
                padding: height / 2,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kzqmXN: "x5lhr3w",
              kZKoxP: "x16ye13r",
              kogj98: "xb9ncqk",
              kmVPX3: "x1fozly0",
              "$$css": true
            };
            export const styles = {
              root: (width, height) => [_temp, {
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100),
                "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height * 2),
                "--x-margin": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width - 50),
                "--x-padding": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height / 2)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "x16ye13r",
                  {
                    "ltr": ".x16ye13r{height:var(--x-height)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xb9ncqk",
                  {
                    "ltr": ".xb9ncqk{margin:var(--x-margin)}",
                    "rtl": null,
                  },
                  1000,
                ],
                [
                  "x1fozly0",
                  {
                    "ltr": ".x1fozly0{padding:var(--x-padding)}",
                    "rtl": null,
                  },
                  1000,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-height",
                  {
                    "ltr": "@property --x-height { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-margin",
                  {
                    "ltr": "@property --x-margin { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
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

        test('unary expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (value) => ({
                opacity: -value,
                transform: +value,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kSiTet: "xb4nw82",
              k3aq6I: "xsqj5wx",
              "$$css": true
            };
            export const styles = {
              root: value => [_temp, {
                "--x-opacity": -value != null ? -value : undefined,
                "--x-transform": +value != null ? +value : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xb4nw82",
                  {
                    "ltr": ".xb4nw82{opacity:var(--x-opacity)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xsqj5wx",
                  {
                    "ltr": ".xsqj5wx{transform:var(--x-transform)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-opacity",
                  {
                    "ltr": "@property --x-opacity { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-transform",
                  {
                    "ltr": "@property --x-transform { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('logical expressions with safe left side', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: color || 'red',
                color: color || 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {
                "--x-backgroundColor": (color || 'red') != null ? color || 'red' : undefined,
                "--x-color": (color || 'black') != null ? color || 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('logical expressions with safe right side', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: 'red' || color,
                color: 'black' || color,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xrkmrrc",
              kMwMTN: "x1mqxbix",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {}]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x1mqxbix",
                  {
                    "ltr": ".x1mqxbix{color:black}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('nullish coalescing with safe left side', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: color ?? 'red',
                color: color ?? 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {
                "--x-backgroundColor": (color ?? 'red') != null ? color ?? 'red' : undefined,
                "--x-color": (color ?? 'black') != null ? color ?? 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('conditional expressions with safe branches', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color, isDark) => ({
                backgroundColor: isDark ? 'black' : 'white',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              "$$css": true
            };
            export const styles = {
              root: (color, isDark) => [_temp, {
                "--x-backgroundColor": (isDark ? 'black' : 'white') != null ? isDark ? 'black' : 'white' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('conditional expressions with safe branches', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color, isDark) => ({
                backgroundColor: isDark ? ('black') : 'white',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              "$$css": true
            };
            export const styles = {
              root: (color, isDark) => [_temp, {
                "--x-backgroundColor": (isDark ? 'black' : 'white') != null ? isDark ? 'black' : 'white' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('complex nested safe expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (width, height, color) => ({
                width: (width + 100) || 200,
                height: (height * 2) ?? 300,
                backgroundColor: \`\${color}\` || 'red',
                color: (-color) || 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kzqmXN: "x5lhr3w",
              kZKoxP: "x16ye13r",
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: (width, height, color) => [_temp, {
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100 || 200),
                "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height * 2 ?? 300),
                "--x-backgroundColor": (\`\${color}\` || 'red') != null ? \`\${color}\` || 'red' : undefined,
                "--x-color": (-color || 'black') != null ? -color || 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "x16ye13r",
                  {
                    "ltr": ".x16ye13r{height:var(--x-height)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-height",
                  {
                    "ltr": "@property --x-height { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('complex safe ternary expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (isDark, isLarge, isActive, width, height, color) => ({
                backgroundColor: isDark ? (isLarge ? 'black' : 'gray') : (isActive ? 'blue' : 'white'),
                color: isDark ? (color || 'white') : (color ?? 'black'),
                width: isLarge ? (width + 100) : (width - 50),
                height: isActive ? (height * 2) : (height / 2),
                margin: isDark ? ((width + height) || 20) : ((width - height) ?? 10),
                padding: isLarge ? ((width * height) + 50) : ((width / height) - 25),
                fontSize: isDark ? (isLarge ? (width + 20) : (width - 10)) : (isActive ? (height + 15) : (height - 5)),
                opacity: isLarge ? (isActive ? 1 : 0.8) : (isDark ? 0.9 : 0.7),
                transform: isActive ? (isLarge ? 'scale(1.2)' : 'scale(1.1)') : (isDark ? 'rotate(5deg)' : 'rotate(-5deg)'),
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              kzqmXN: "x5lhr3w",
              kZKoxP: "x16ye13r",
              kogj98: "xb9ncqk",
              kmVPX3: "x1fozly0",
              kGuDYH: "xdmh292",
              kSiTet: "xb4nw82",
              k3aq6I: "xsqj5wx",
              "$$css": true
            };
            export const styles = {
              root: (isDark, isLarge, isActive, width, height, color) => [_temp, {
                "--x-backgroundColor": (isDark ? isLarge ? 'black' : 'gray' : isActive ? 'blue' : 'white') != null ? isDark ? isLarge ? 'black' : 'gray' : isActive ? 'blue' : 'white' : undefined,
                "--x-color": (isDark ? color || 'white' : color ?? 'black') != null ? isDark ? color || 'white' : color ?? 'black' : undefined,
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isLarge ? width + 100 : width - 50),
                "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isActive ? height * 2 : height / 2),
                "--x-margin": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isDark ? width + height || 20 : width - height ?? 10),
                "--x-padding": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isLarge ? width * height + 50 : width / height - 25),
                "--x-fontSize": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(isDark ? isLarge ? width + 20 : width - 10 : isActive ? height + 15 : height - 5),
                "--x-opacity": (isLarge ? isActive ? 1 : 0.8 : isDark ? 0.9 : 0.7) != null ? isLarge ? isActive ? 1 : 0.8 : isDark ? 0.9 : 0.7 : undefined,
                "--x-transform": (isActive ? isLarge ? 'scale(1.2)' : 'scale(1.1)' : isDark ? 'rotate(5deg)' : 'rotate(-5deg)') != null ? isActive ? isLarge ? 'scale(1.2)' : 'scale(1.1)' : isDark ? 'rotate(5deg)' : 'rotate(-5deg)' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "x16ye13r",
                  {
                    "ltr": ".x16ye13r{height:var(--x-height)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xb9ncqk",
                  {
                    "ltr": ".xb9ncqk{margin:var(--x-margin)}",
                    "rtl": null,
                  },
                  1000,
                ],
                [
                  "x1fozly0",
                  {
                    "ltr": ".x1fozly0{padding:var(--x-padding)}",
                    "rtl": null,
                  },
                  1000,
                ],
                [
                  "xdmh292",
                  {
                    "ltr": ".xdmh292{font-size:var(--x-fontSize)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xb4nw82",
                  {
                    "ltr": ".xb4nw82{opacity:var(--x-opacity)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xsqj5wx",
                  {
                    "ltr": ".xsqj5wx{transform:var(--x-transform)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-height",
                  {
                    "ltr": "@property --x-height { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-margin",
                  {
                    "ltr": "@property --x-margin { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-padding",
                  {
                    "ltr": "@property --x-padding { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-fontSize",
                  {
                    "ltr": "@property --x-fontSize { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-opacity",
                  {
                    "ltr": "@property --x-opacity { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-transform",
                  {
                    "ltr": "@property --x-transform { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });
      });

      test('style function', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: (color) => ({
              backgroundColor: 'red',
              color,
            })
          });
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          const _temp = {
            kWkggS: "xrkmrrc",
            "$$css": true
          };
          export const styles = {
            root: color => [_temp, color !== undefined && {
              kMwMTN: color != null ? "x14rh7hd" : "",
              $$css: true
            }, {
              "--x-color": color != null ? color : undefined
            }]
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrkmrrc",
                {
                  "ltr": ".xrkmrrc{background-color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x14rh7hd",
                {
                  "ltr": ".x14rh7hd{color:var(--x-color)}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "--x-color",
                {
                  "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                  "rtl": null,
                },
                0,
              ],
            ],
          }
        `);
      });

      test('style function and object', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            one: (color) => ({
              color: color,
            }),
            two: {
              color: 'black',
            },
          });
        `);
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            one: color => [color !== undefined && {
              kMwMTN: color != null ? "x14rh7hd" : "",
              $$css: true
            }, {
              "--x-color": color != null ? color : undefined
            }],
            two: {
              kMwMTN: "x1mqxbix",
              $$css: true
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "x14rh7hd",
                {
                  "ltr": ".x14rh7hd{color:var(--x-color)}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1mqxbix",
                {
                  "ltr": ".x1mqxbix{color:black}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "--x-color",
                {
                  "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                  "rtl": null,
                },
                0,
              ],
            ],
          }
        `);
      });

      test('style function with custom properties', () => {
        const { code, metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            root: (bgColor, otherColor) => ({
              '--background-color': bgColor,
              '--otherColor': otherColor,
            }),
          });
        `);
        // NOTE: the generated variable name is a little weird, but valid.
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            root: (bgColor, otherColor) => [bgColor !== undefined && {
              "--background-color": bgColor != null ? "xwn82o0" : "",
              $$css: true
            }, otherColor !== undefined && {
              "--otherColor": otherColor != null ? "xp3hsad" : "",
              $$css: true
            }, {
              "--x---background-color": bgColor != null ? bgColor : undefined,
              "--x---otherColor": otherColor != null ? otherColor : undefined
            }]
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xwn82o0",
                {
                  "ltr": ".xwn82o0{--background-color:var(--x---background-color)}",
                  "rtl": null,
                },
                1,
              ],
              [
                "xp3hsad",
                {
                  "ltr": ".xp3hsad{--otherColor:var(--x---otherColor)}",
                  "rtl": null,
                },
                1,
              ],
              [
                "--x---background-color",
                {
                  "ltr": "@property --x---background-color { syntax: "*"; inherits: false;}",
                  "rtl": null,
                },
                0,
              ],
              [
                "--x---otherColor",
                {
                  "ltr": "@property --x---otherColor { syntax: "*"; inherits: false;}",
                  "rtl": null,
                },
                0,
              ],
            ],
          }
        `);
      });

      describe('simple values', () => {
        test('set number unit', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (width) => ({
                width,
              })
            });
          `);
          // Check that dynamic number values get units where appropriate
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: width => [width !== undefined && {
                kzqmXN: width != null ? "x5lhr3w" : "",
                $$css: true
              }, {
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('set mixed values', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (width) => ({
                width,
                backgroundColor: 'red',
                height: width + 100,
              })
            });
          `);
          // Check that dynamic number values get units where appropriate
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xrkmrrc",
              kZKoxP: "x16ye13r",
              "$$css": true
            };
            export const styles = {
              root: width => [_temp, width !== undefined && {
                kzqmXN: width != null ? "x5lhr3w" : "",
                $$css: true
              }, {
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width),
                "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x16ye13r",
                  {
                    "ltr": ".x16ye13r{height:var(--x-height)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-height",
                  {
                    "ltr": "@property --x-height { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('set custom property', () => {
          const options = {
            filename: 'MyComponent.js',
            unstable_moduleResolution: { type: 'haste' },
          };
          const { code, metadata } = transform(
            `
            import * as stylex from '@stylexjs/stylex';
            import {vars} from 'vars.stylex.js';

            export const styles = stylex.create({
              root: (width) => ({
                [vars.width]: width
              })
            });
          `,
            options,
          );
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            import { vars } from 'vars.stylex.js';
            export const styles = {
              root: width => [width !== undefined && {
                "--x1anmu0j": width != null ? "x5fq457" : "",
                $$css: true
              }, {
                "--x---x1anmu0j": width != null ? width : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5fq457",
                  {
                    "ltr": ".x5fq457{--x1anmu0j:var(--x---x1anmu0j)}",
                    "rtl": null,
                  },
                  1,
                ],
                [
                  "--x---x1anmu0j",
                  {
                    "ltr": "@property --x---x1anmu0j { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });
      });

      describe('safe conditional checks', () => {
        test('template literal expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: \`\${color}\`,
                color: \`\${color}px\`,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {
                "--x-backgroundColor": \`\${color}\` != null ? \`\${color}\` : undefined,
                "--x-color": \`\${color}px\` != null ? \`\${color}px\` : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('binary expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (width, height) => ({
                width: width + 100,
                height: height * 2,
                margin: width - 50,
                padding: height / 2,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kzqmXN: "x5lhr3w",
              kZKoxP: "x16ye13r",
              kogj98: "xb9ncqk",
              kmVPX3: "x1fozly0",
              "$$css": true
            };
            export const styles = {
              root: (width, height) => [_temp, {
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100),
                "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height * 2),
                "--x-margin": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width - 50),
                "--x-padding": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height / 2)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "x16ye13r",
                  {
                    "ltr": ".x16ye13r{height:var(--x-height)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xb9ncqk",
                  {
                    "ltr": ".xb9ncqk{margin:var(--x-margin)}",
                    "rtl": null,
                  },
                  1000,
                ],
                [
                  "x1fozly0",
                  {
                    "ltr": ".x1fozly0{padding:var(--x-padding)}",
                    "rtl": null,
                  },
                  1000,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-height",
                  {
                    "ltr": "@property --x-height { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-margin",
                  {
                    "ltr": "@property --x-margin { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
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

        test('unary expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (value) => ({
                opacity: -value,
                transform: +value,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kSiTet: "xb4nw82",
              k3aq6I: "xsqj5wx",
              "$$css": true
            };
            export const styles = {
              root: value => [_temp, {
                "--x-opacity": -value != null ? -value : undefined,
                "--x-transform": +value != null ? +value : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xb4nw82",
                  {
                    "ltr": ".xb4nw82{opacity:var(--x-opacity)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xsqj5wx",
                  {
                    "ltr": ".xsqj5wx{transform:var(--x-transform)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-opacity",
                  {
                    "ltr": "@property --x-opacity { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-transform",
                  {
                    "ltr": "@property --x-transform { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('logical expressions with safe left side', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: color || 'red',
                color: color || 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {
                "--x-backgroundColor": (color || 'red') != null ? color || 'red' : undefined,
                "--x-color": (color || 'black') != null ? color || 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('logical expressions with safe right side', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: 'red' || color,
                color: 'black' || color,
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xrkmrrc",
              kMwMTN: "x1mqxbix",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {}]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xrkmrrc",
                  {
                    "ltr": ".xrkmrrc{background-color:red}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x1mqxbix",
                  {
                    "ltr": ".x1mqxbix{color:black}",
                    "rtl": null,
                  },
                  3000,
                ],
              ],
            }
          `);
        });

        test('nullish coalescing with safe left side', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: color ?? 'red',
                color: color ?? 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: color => [_temp, {
                "--x-backgroundColor": (color ?? 'red') != null ? color ?? 'red' : undefined,
                "--x-color": (color ?? 'black') != null ? color ?? 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('conditional expressions with safe branches', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color, isDark) => ({
                backgroundColor: isDark ? 'black' : 'white',
                color: isDark ? color : 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kWkggS: "xl8spv7",
              "$$css": true
            };
            export const styles = {
              root: (color, isDark) => [_temp, (isDark ? color : 'black') !== undefined && {
                kMwMTN: (isDark ? color : 'black') != null ? "x14rh7hd" : "",
                $$css: true
              }, {
                "--x-backgroundColor": (isDark ? 'black' : 'white') != null ? isDark ? 'black' : 'white' : undefined,
                "--x-color": (isDark ? color : 'black') != null ? isDark ? color : 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('complex nested safe expressions', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (width, height, color) => ({
                width: (width + 100) || 200,
                height: (height * 2) ?? 300,
                backgroundColor: \`\${color}\` || 'red',
                color: (-color) || 'black',
              })
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kzqmXN: "x5lhr3w",
              kZKoxP: "x16ye13r",
              kWkggS: "xl8spv7",
              kMwMTN: "x14rh7hd",
              "$$css": true
            };
            export const styles = {
              root: (width, height, color) => [_temp, {
                "--x-width": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width + 100 || 200),
                "--x-height": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(height * 2 ?? 300),
                "--x-backgroundColor": (\`\${color}\` || 'red') != null ? \`\${color}\` || 'red' : undefined,
                "--x-color": (-color || 'black') != null ? -color || 'black' : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x5lhr3w",
                  {
                    "ltr": ".x5lhr3w{width:var(--x-width)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "x16ye13r",
                  {
                    "ltr": ".x16ye13r{height:var(--x-height)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xl8spv7",
                  {
                    "ltr": ".xl8spv7{background-color:var(--x-backgroundColor)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x14rh7hd",
                  {
                    "ltr": ".x14rh7hd{color:var(--x-color)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "--x-width",
                  {
                    "ltr": "@property --x-width { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-height",
                  {
                    "ltr": "@property --x-height { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-backgroundColor",
                  {
                    "ltr": "@property --x-backgroundColor { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-color",
                  {
                    "ltr": "@property --x-color { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });
      });

      describe('object values: pseudo-classes', () => {
        test('valid pseudo-class', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (color) => ({
                backgroundColor: {
                  ':hover': color,
                },
                color: {
                  ':hover': color,
                }
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: color => [color !== undefined && {
                kWkggS: color != null ? "x1j2k28p" : "",
                $$css: true
              }, color !== undefined && {
                kMwMTN: color != null ? "x1qvlgnj" : "",
                $$css: true
              }, {
                "--x-1e2mv7m": color != null ? color : undefined,
                "--x-1113oo7": color != null ? color : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1j2k28p",
                  {
                    "ltr": ".x1j2k28p:hover{background-color:var(--x-1e2mv7m)}",
                    "rtl": null,
                  },
                  3130,
                ],
                [
                  "x1qvlgnj",
                  {
                    "ltr": ".x1qvlgnj:hover{color:var(--x-1113oo7)}",
                    "rtl": null,
                  },
                  3130,
                ],
                [
                  "--x-1e2mv7m",
                  {
                    "ltr": "@property --x-1e2mv7m { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-1113oo7",
                  {
                    "ltr": "@property --x-1113oo7 { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('pseudo-class generated order', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (hover, active, focus) => ({
                color: {
                  ':hover': hover,
                  ':active': active,
                  ':focus': focus,
                  ':nth-child(2n)': 'purple',
                },
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: (hover, active, focus) => [{
                kMwMTN: (hover != null ? "x1qvlgnj " : "") + (active != null ? "xx746rz " : "") + (focus != null ? "x152n5rj " : "") + "x126ychx",
                $$css: true
              }, {
                "--x-1113oo7": hover != null ? hover : undefined,
                "--x-hxnnmm": active != null ? active : undefined,
                "--x-8tbbve": focus != null ? focus : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1qvlgnj",
                  {
                    "ltr": ".x1qvlgnj:hover{color:var(--x-1113oo7)}",
                    "rtl": null,
                  },
                  3130,
                ],
                [
                  "xx746rz",
                  {
                    "ltr": ".xx746rz:active{color:var(--x-hxnnmm)}",
                    "rtl": null,
                  },
                  3170,
                ],
                [
                  "x152n5rj",
                  {
                    "ltr": ".x152n5rj:focus{color:var(--x-8tbbve)}",
                    "rtl": null,
                  },
                  3150,
                ],
                [
                  "x126ychx",
                  {
                    "ltr": ".x126ychx:nth-child(2n){color:purple}",
                    "rtl": null,
                  },
                  3060,
                ],
                [
                  "--x-1113oo7",
                  {
                    "ltr": "@property --x-1113oo7 { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-hxnnmm",
                  {
                    "ltr": "@property --x-hxnnmm { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-8tbbve",
                  {
                    "ltr": "@property --x-8tbbve { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });
      });

      describe('object values: pseudo-elements', () => {
        test('"::before" and "::after"', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: (a, b) => ({
                '::before': {
                  color: a
                },
                '::after': {
                  color: b
                },
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: (a, b) => [a !== undefined && {
                kxBb7d: a != null ? "xaigonn" : "",
                $$css: true
              }, b !== undefined && {
                kB1Fuz: b != null ? "x1p1099i" : "",
                $$css: true
              }, {
                "--x-1g451k2": a != null ? a : undefined,
                "--x-19erzii": b != null ? b : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xaigonn",
                  {
                    "ltr": ".xaigonn::before{color:var(--x-1g451k2)}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "x1p1099i",
                  {
                    "ltr": ".x1p1099i::after{color:var(--x-19erzii)}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "--x-1g451k2",
                  {
                    "ltr": "@property --x-1g451k2 { syntax: "*"; inherits: true;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-19erzii",
                  {
                    "ltr": "@property --x-19erzii { syntax: "*"; inherits: true;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('"::placeholder"', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: (color) => ({
                '::placeholder': {
                  color,
                },
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: color => [color !== undefined && {
                k8Qsv1: color != null ? "x1mzl164" : "",
                $$css: true
              }, {
                "--x-163tekb": color != null ? color : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1mzl164",
                  {
                    "ltr": ".x1mzl164::placeholder{color:var(--x-163tekb)}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "--x-163tekb",
                  {
                    "ltr": "@property --x-163tekb { syntax: "*"; inherits: true;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('"::thumb"', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: (width) => ({
                '::thumb': {
                  width,
                },
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              foo: width => [width !== undefined && {
                k8pbKx: width != null ? "x18fgbt0" : "",
                $$css: true
              }, {
                "--x-msahdu": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(width)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x18fgbt0",
                  {
                    "ltr": ".x18fgbt0::-webkit-slider-thumb, .x18fgbt0::-moz-range-thumb, .x18fgbt0::-ms-thumb{width:var(--x-msahdu)}",
                    "rtl": null,
                  },
                  9000,
                ],
                [
                  "--x-msahdu",
                  {
                    "ltr": "@property --x-msahdu { syntax: "*"; inherits: true;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('"::before" containing pseudo-classes', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              foo: (color) => ({
                '::before': {
                  color: {
                    default: 'red',
                    ':hover': color,
                  }
                },
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            const _temp = {
              kxBb7d: "x16oeupf " + "xndy4z1",
              "$$css": true
            };
            export const styles = {
              foo: color => [_temp, {
                "--x-6bge3v": color != null ? color : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x16oeupf",
                  {
                    "ltr": ".x16oeupf::before{color:red}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "xndy4z1",
                  {
                    "ltr": ".xndy4z1:hover::before{color:var(--x-6bge3v)}",
                    "rtl": null,
                  },
                  8130,
                ],
                [
                  "--x-6bge3v",
                  {
                    "ltr": "@property --x-6bge3v { syntax: "*"; inherits: true;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('dynamic style in "::after" generates valid @property with inherits', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              repro: (color) => ({
                '::after': {
                  color,
                },
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              repro: color => [color !== undefined && {
                kB1Fuz: color != null ? "x1p1099i" : "",
                $$css: true
              }, {
                "--x-19erzii": color != null ? color : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x1p1099i",
                  {
                    "ltr": ".x1p1099i::after{color:var(--x-19erzii)}",
                    "rtl": null,
                  },
                  8000,
                ],
                [
                  "--x-19erzii",
                  {
                    "ltr": "@property --x-19erzii { syntax: "*"; inherits: true;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });
      });

      describe('object values: queries', () => {
        test('media queries', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (a, b, c) => ({
                width: {
                  default: 'color-mix(' + color + ', blue)',
                  '@media (min-width: 1000px)': b,
                  '@media (min-width: 2000px)': c,
                }
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: (a, b, c) => [{
                kzqmXN: "x11ymkkh " + "xwr9iet " + (c != null ? "x1bai16n" : ""),
                $$css: true
              }, {
                "--x-1xmrurk": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)('color-mix(' + color + ', blue)'),
                "--x-wm47pl": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b),
                "--x-1obb2yn": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x11ymkkh",
                  {
                    "ltr": ".x11ymkkh{width:var(--x-1xmrurk)}",
                    "rtl": null,
                  },
                  4000,
                ],
                [
                  "xwr9iet",
                  {
                    "ltr": "@media (min-width: 1000px) and (max-width: 1999.98px){.xwr9iet.xwr9iet{width:var(--x-wm47pl)}}",
                    "rtl": null,
                  },
                  4200,
                ],
                [
                  "x1bai16n",
                  {
                    "ltr": "@media (min-width: 2000px){.x1bai16n.x1bai16n{width:var(--x-1obb2yn)}}",
                    "rtl": null,
                  },
                  4200,
                ],
                [
                  "--x-1xmrurk",
                  {
                    "ltr": "@property --x-1xmrurk { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-wm47pl",
                  {
                    "ltr": "@property --x-wm47pl { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-1obb2yn",
                  {
                    "ltr": "@property --x-1obb2yn { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('supports queries', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (a, b, c) => ({
                color: {
                  default: a,
                  '@supports (hover: hover)': b,
                  '@supports not (hover: hover)': c,
                }
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: (a, b, c) => [{
                kMwMTN: (a != null ? "x3d248p " : "") + (b != null ? "x1iuwwch " : "") + (c != null ? "x5268pl" : ""),
                $$css: true
              }, {
                "--x-4xs81a": a != null ? a : undefined,
                "--x-b262sw": b != null ? b : undefined,
                "--x-wu2acw": c != null ? c : undefined
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "x3d248p",
                  {
                    "ltr": ".x3d248p{color:var(--x-4xs81a)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x1iuwwch",
                  {
                    "ltr": "@supports (hover: hover){.x1iuwwch.x1iuwwch{color:var(--x-b262sw)}}",
                    "rtl": null,
                  },
                  3030,
                ],
                [
                  "x5268pl",
                  {
                    "ltr": "@supports not (hover: hover){.x5268pl.x5268pl{color:var(--x-wu2acw)}}",
                    "rtl": null,
                  },
                  3030,
                ],
                [
                  "--x-4xs81a",
                  {
                    "ltr": "@property --x-4xs81a { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-b262sw",
                  {
                    "ltr": "@property --x-b262sw { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-wu2acw",
                  {
                    "ltr": "@property --x-wu2acw { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('media query with pseudo-classes', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (a, b, c) => ({
                fontSize: {
                  default: a,
                  '@media (min-width: 800px)': {
                    default: b,
                    ':hover': c
                  }
                }
              }),
            });
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: (a, b, c) => [{
                kGuDYH: (a != null ? "xww4jgc " : "") + (b != null ? "xfqys7t " : "") + (c != null ? "x13w7uki" : ""),
                $$css: true
              }, {
                "--x-19zvkyr": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(a),
                "--x-1xajcet": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b),
                "--x-ke45ok": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c)
              }]
            };"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xww4jgc",
                  {
                    "ltr": ".xww4jgc{font-size:var(--x-19zvkyr)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "xfqys7t",
                  {
                    "ltr": "@media (min-width: 800px){.xfqys7t.xfqys7t{font-size:var(--x-1xajcet)}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x13w7uki",
                  {
                    "ltr": "@media (min-width: 800px){.x13w7uki.x13w7uki:hover{font-size:var(--x-ke45ok)}}",
                    "rtl": null,
                  },
                  3330,
                ],
                [
                  "--x-19zvkyr",
                  {
                    "ltr": "@property --x-19zvkyr { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-1xajcet",
                  {
                    "ltr": "@property --x-1xajcet { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-ke45ok",
                  {
                    "ltr": "@property --x-ke45ok { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });

        test('media query values with nullish coalescing', () => {
          const { code, metadata } = transform(`
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              root: (a, b, c) => ({
                fontSize: {
                  default: a ? '16px' : undefined,
                  '@media (min-width: 800px)': b ? '18px' : undefined,
                  '@media (min-width: 1280px)': c ? '20px' : undefined,
                }
              }),
            });
            stylex.props(styles.root(true, false, true));
          `);
          expect(code).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            export const styles = {
              root: (a, b, c) => [{
                kGuDYH: ((a ? '16px' : undefined) != null ? "xww4jgc " : "") + ((b ? '18px' : undefined) != null ? "x1845opi " : "") + ((c ? '20px' : undefined) != null ? "x1j86d60" : ""),
                $$css: true
              }, {
                "--x-19zvkyr": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(a ? '16px' : undefined),
                "--x-1bks2es": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(b ? '18px' : undefined),
                "--x-q0n1i6": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(c ? '20px' : undefined)
              }]
            };
            stylex.props(styles.root(true, false, true));"
          `);
          expect(metadata).toMatchInlineSnapshot(`
            {
              "stylex": [
                [
                  "xww4jgc",
                  {
                    "ltr": ".xww4jgc{font-size:var(--x-19zvkyr)}",
                    "rtl": null,
                  },
                  3000,
                ],
                [
                  "x1845opi",
                  {
                    "ltr": "@media (min-width: 800px) and (max-width: 1279.98px){.x1845opi.x1845opi{font-size:var(--x-1bks2es)}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "x1j86d60",
                  {
                    "ltr": "@media (min-width: 1280px){.x1j86d60.x1j86d60{font-size:var(--x-q0n1i6)}}",
                    "rtl": null,
                  },
                  3200,
                ],
                [
                  "--x-19zvkyr",
                  {
                    "ltr": "@property --x-19zvkyr { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-1bks2es",
                  {
                    "ltr": "@property --x-1bks2es { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
                [
                  "--x-q0n1i6",
                  {
                    "ltr": "@property --x-q0n1i6 { syntax: "*"; inherits: false;}",
                    "rtl": null,
                  },
                  0,
                ],
              ],
            }
          `);
        });
      });
    });

    describe('options `debug:true`', () => {
      test('adds debug data', () => {
        const options = {
          debug: true,
          filename: '/html/js/components/Foo.react.js',
        };
        const { code, metadata } = transform(
          `
            import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            foo: {
              color: 'red'
            },
            'bar-baz': {
              display: 'block'
            },
            1: {
              fontSize: '1em'
            }
          });
        `,
          options,
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            "1": {
              "fontSize-kGuDYH": "xrv4cvt",
              $$css: "components/Foo.react.js:10"
            },
            foo: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "components/Foo.react.js:4"
            },
            "bar-baz": {
              "display-k1xSpc": "x1lliihq",
              $$css: "components/Foo.react.js:7"
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrv4cvt",
                {
                  "ltr": ".xrv4cvt{font-size:1em}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1e2nbdu",
                {
                  "ltr": ".x1e2nbdu{color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1lliihq",
                {
                  "ltr": ".x1lliihq{display:block}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('adds debug data for npm packages', () => {
        const options = {
          debug: true,
          filename: '/js/node_modules/npm-package/dist/components/Foo.react.js',
        };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            foo: {
              color: 'red'
            },
            'bar-baz': {
              display: 'block'
            },
            1: {
              fontSize: '1em'
            }
          });
        `,
          options,
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            "1": {
              "fontSize-kGuDYH": "xrv4cvt",
              $$css: "npm-package:components/Foo.react.js:10"
            },
            foo: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "npm-package:components/Foo.react.js:4"
            },
            "bar-baz": {
              "display-k1xSpc": "x1lliihq",
              $$css: "npm-package:components/Foo.react.js:7"
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrv4cvt",
                {
                  "ltr": ".xrv4cvt{font-size:1em}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1e2nbdu",
                {
                  "ltr": ".x1e2nbdu{color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1lliihq",
                {
                  "ltr": ".x1lliihq{display:block}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('adds debug data (haste)', () => {
        const options = {
          debug: true,
          filename: '/html/js/components/Foo.react.js',
          unstable_moduleResolution: { type: 'haste' },
        };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            foo: {
              color: 'red'
            },
            'bar-baz': {
              display: 'block'
            },
            1: {
              fontSize: '1em'
            }
          });
        `,
          options,
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            "1": {
              "fontSize-kGuDYH": "xrv4cvt",
              $$css: "Foo.react.js:10"
            },
            foo: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "Foo.react.js:4"
            },
            "bar-baz": {
              "display-k1xSpc": "x1lliihq",
              $$css: "Foo.react.js:7"
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrv4cvt",
                {
                  "ltr": ".xrv4cvt{font-size:1em}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1e2nbdu",
                {
                  "ltr": ".x1e2nbdu{color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1lliihq",
                {
                  "ltr": ".x1lliihq{display:block}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });

      test('adds debug data for npm packages (haste)', () => {
        const options = {
          debug: true,
          filename: '/node_modules/npm-package/dist/components/Foo.react.js',
          unstable_moduleResolution: { type: 'haste' },
        };
        const { code, metadata } = transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            foo: {
              color: 'red'
            },
            'bar-baz': {
              display: 'block'
            },
            1: {
              fontSize: '1em'
            }
          });
        `,
          options,
        );
        expect(code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          export const styles = {
            "1": {
              "fontSize-kGuDYH": "xrv4cvt",
              $$css: "npm-package:components/Foo.react.js:10"
            },
            foo: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "npm-package:components/Foo.react.js:4"
            },
            "bar-baz": {
              "display-k1xSpc": "x1lliihq",
              $$css: "npm-package:components/Foo.react.js:7"
            }
          };"
        `);
        expect(metadata).toMatchInlineSnapshot(`
          {
            "stylex": [
              [
                "xrv4cvt",
                {
                  "ltr": ".xrv4cvt{font-size:1em}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1e2nbdu",
                {
                  "ltr": ".x1e2nbdu{color:red}",
                  "rtl": null,
                },
                3000,
              ],
              [
                "x1lliihq",
                {
                  "ltr": ".x1lliihq{display:block}",
                  "rtl": null,
                },
                3000,
              ],
            ],
          }
        `);
      });
    });

    // LEGACY (TODO: Remove)
    describe('legacy / deprecated', () => {
      const options = {
        runtimeInjection: true,
      };

      test('transforms nested pseudo-class to CSS', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                ':hover': {
                  backgroundColor: 'red',
                  color: 'blue',
                },
              },
            });
          `,
            options,
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1gykpug:hover{background-color:red}",
            priority: 3130
          });
          _inject2({
            ltr: ".x17z2mba:hover{color:blue}",
            priority: 3130
          });"
        `);
      });

      describe('pseudo-classes', () => {
        // TODO: this should either fail or guarantee an insertion order relative to valid pseudo-classes
        test('transforms invalid pseudo-class', () => {
          expect(
            transform(
              `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  ':invalpwdijad': {
                    backgroundColor: 'red',
                    color: 'blue',
                  },
                },
              });
            `,
              options,
            ).code,
          ).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x19iys6w:invalpwdijad{background-color:red}",
              priority: 3040
            });
            _inject2({
              ltr: ".x5z3o4w:invalpwdijad{color:blue}",
              priority: 3040
            });"
          `);
        });

        test('transforms valid pseudo-classes in order', () => {
          expect(
            transform(
              `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  ':hover': {
                    color: 'blue',
                  },
                  ':active': {
                    color: 'red',
                  },
                  ':focus': {
                    color: 'yellow',
                  },
                  ':nth-child(2n)': {
                    color: 'purple'
                  }
                },
              });
            `,
              options,
            ).code,
          ).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x17z2mba:hover{color:blue}",
              priority: 3130
            });
            _inject2({
              ltr: ".x96fq8s:active{color:red}",
              priority: 3170
            });
            _inject2({
              ltr: ".x1wvtd7d:focus{color:yellow}",
              priority: 3150
            });
            _inject2({
              ltr: ".x126ychx:nth-child(2n){color:purple}",
              priority: 3060
            });"
          `);
        });

        test('transforms pseudo-class with array value as fallbacks', () => {
          expect(
            transform(
              `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  ':hover': {
                    position: ['sticky', 'fixed'],
                  }
                },
              });
            `,
              options,
            ).code,
          ).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            _inject2({
              ltr: ".x1nxcus0:hover{position:sticky;position:fixed}",
              priority: 3130
            });"
          `);
        });
      });

      test('transforms legacy pseudo class within a pseudo element', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                '::before': {
                  color: 'red',
                  ':hover': {
                    color: 'blue',
                  },
                },
              },
            });
          `,
            options,
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x16oeupf::before{color:red}",
            priority: 8000
          });
          _inject2({
            ltr: ".xeb2lg0:hover::before{color:blue}",
            priority: 8130
          });
          export const styles = {
            foo: {
              kxBb7d: "x16oeupf",
              kkC3X7: "xeb2lg0",
              $$css: true
            }
          };"
        `);
      });

      test('transforms pseudo elements within legacy pseudo class', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                '::before': {
                  color: 'red',
                },
                ':hover': {
                  '::before': {
                    color: 'blue',
                  },
                },
              },
            });
          `,
            options,
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x16oeupf::before{color:red}",
            priority: 8000
          });
          _inject2({
            ltr: ".xzzpreb:hover::before{color:blue}",
            priority: 8130
          });
          export const styles = {
            foo: {
              kxBb7d: "x16oeupf",
              kFlxxK: "xzzpreb",
              $$css: true
            }
          };"
        `);
      });

      test('transforms pseudo elements sandwiched within pseudo classes', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            export const styles = stylex.create({
              foo: {
                '::before': {
                  color: 'red',
                },
                ':hover': {
                  '::before': {
                    color: {
                      default: 'blue',
                      ':hover': 'green',
                      ':active': 'purple',
                    },
                  },
                },
              },
            });
          `,
            options,
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x16oeupf::before{color:red}",
            priority: 8000
          });
          _inject2({
            ltr: ".xzzpreb:hover::before{color:blue}",
            priority: 8130
          });
          _inject2({
            ltr: ".x1gobd9t:hover:hover::before{color:green}",
            priority: 8260
          });
          _inject2({
            ltr: ".xs8jp5:hover:active::before{color:purple}",
            priority: 8300
          });
          export const styles = {
            foo: {
              kxBb7d: "x16oeupf",
              kFlxxK: "xzzpreb x1gobd9t xs8jp5",
              $$css: true
            }
          };"
        `);
      });

      test('transforms media queries', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                backgroundColor: 'red',
                '@media (min-width: 1000px)': {
                  backgroundColor: 'blue',
                },
                '@media (min-width: 2000px)': {
                  backgroundColor: 'purple',
                },
              },
            });
          `,
            options,
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".xrkmrrc{background-color:red}",
            priority: 3000
          });
          _inject2({
            ltr: "@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",
            priority: 3200
          });
          _inject2({
            ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
            priority: 3200
          });"
        `);
      });

      test('transforms supports queries', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                backgroundColor: 'red',
                '@supports (hover: hover)': {
                  backgroundColor: 'blue',
                },
                '@supports not (hover: hover)': {
                  backgroundColor: 'purple',
                },
              },
            });
          `,
            options,
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".xrkmrrc{background-color:red}",
            priority: 3000
          });
          _inject2({
            ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
            priority: 3030
          });
          _inject2({
            ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
            priority: 3030
          });"
        `);
      });

      test('transforms dynamic shorthands in legacy-expand-shorthands mode', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            export const styles = stylex.create({
              default: (margin) => ({
                backgroundColor: 'red',
                margin: {
                  default: margin,
                  ':hover': margin + 4,
                },
                marginTop: margin - 4,
              })
            });
          `,
            {
              runtimeInjection: true,
              styleResolution: 'legacy-expand-shorthands',
            },
          ).code,
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          const _temp = {
            kWkggS: "xrkmrrc",
            keoZOQ: "x1gkbulp",
            "$$css": true
          };
          _inject2({
            ltr: ".xrkmrrc{background-color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".x17e2bsb{margin-inline-end:var(--x-14mfytm)}",
            priority: 3000
          });
          _inject2({
            ltr: ".xtcj1g9:hover{margin-inline-end:var(--x-yepcm9)}",
            priority: 3130
          });
          _inject2({
            ltr: ".xg6eqc8{margin-bottom:var(--x-14mfytm)}",
            priority: 4000
          });
          _inject2({
            ltr: ".xgrn1a3:hover{margin-bottom:var(--x-yepcm9)}",
            priority: 4130
          });
          _inject2({
            ltr: ".x19ja4a5{margin-inline-start:var(--x-14mfytm)}",
            priority: 3000
          });
          _inject2({
            ltr: ".x2tye95:hover{margin-inline-start:var(--x-yepcm9)}",
            priority: 3130
          });
          _inject2({
            ltr: ".x1gkbulp{margin-top:var(--x-marginTop)}",
            priority: 4000
          });
          _inject2({
            ltr: "@property --x-14mfytm { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          _inject2({
            ltr: "@property --x-yepcm9 { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          _inject2({
            ltr: "@property --x-marginTop { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          export const styles = {
            default: margin => [_temp, {
              k71WvV: (margin != null ? "x17e2bsb " : "") + "xtcj1g9",
              k1K539: (margin != null ? "xg6eqc8 " : "") + "xgrn1a3",
              keTefX: (margin != null ? "x19ja4a5 " : "") + "x2tye95",
              $$css: true
            }, {
              "--x-14mfytm": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(margin),
              "--x-yepcm9": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(margin + 4),
              "--x-marginTop": (val => typeof val === "number" ? val + "px" : val != null ? val : undefined)(margin - 4)
            }]
          };"
        `);
      });
    });
  });
});
