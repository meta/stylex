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
    plugins: [[stylexPlugin, { ...opts }]],
  });

  return { code, metadata };
}

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] CSS keyframes', () => {
    test('keyframes object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.keyframes({
          from: {
            color: 'red',
          },
          to: {
            color: 'blue',
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const name = "x2up61p-B";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x2up61p-B",
              {
                "ltr": "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('local variables used in keyframes object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const COLOR = 'red';
        export const name = stylex.keyframes({
          from: {
            color: COLOR,
          },
          to: {
            color: 'blue',
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const COLOR = 'red';
        export const name = "x2up61p-B";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x2up61p-B",
              {
                "ltr": "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('template literals used in keyframes object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const COLOR = 'red';
        const name = stylex.keyframes({
          from: {
            color: COLOR,
          },
          to: {
            color: 'blue',
          }
        });
        export const styles = stylex.create({
          root: {
            animationName: \`\${name}\`,
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const COLOR = 'red';
        const name = "x2up61p-B";
        export const styles = {
          root: {
            kKVMdj: "xx2qnu0",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x2up61p-B",
              {
                "ltr": "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}",
                "rtl": null,
              },
              0,
            ],
            [
              "xx2qnu0",
              {
                "ltr": ".xx2qnu0{animation-name:x2up61p-B}",
                "rtl": null,
              },
              3000,
            ],
          ],
        }
      `);
    });

    test('keyframes object used inline', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({
          root: {
            animationName: stylex.keyframes({
              from: {
                color: 'red',
              },
              to: {
                color: 'blue',
              },
            }),
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const styles = {
          root: {
            kKVMdj: "xx2qnu0",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x2up61p-B",
              {
                "ltr": "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}",
                "rtl": null,
              },
              0,
            ],
            [
              "xx2qnu0",
              {
                "ltr": ".xx2qnu0{animation-name:x2up61p-B}",
                "rtl": null,
              },
              3000,
            ],
          ],
        }
      `);
    });

    test('keyframes object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.keyframes({
          from: {
            color: 'red',
          },
          to: {
            color: 'blue',
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const name = "x2up61p-B";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x2up61p-B",
              {
                "ltr": "@keyframes x2up61p-B{from{color:red;}to{color:blue;}}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('unitless numbers in keyframes object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.keyframes({
          from: {
            fontWeight: 400,
            lineHeight: 1.2,
            zIndex: 1,
          },
          to: {
            fontWeight: 700,
            lineHeight: 1.5,
            zIndex: 2,
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const name = "xrv73u4-B";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xrv73u4-B",
              {
                "ltr": "@keyframes xrv73u4-B{from{font-weight:400;line-height:1.2;z-index:1;}to{font-weight:700;line-height:1.5;z-index:2;}}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('[legacy] keyframes object RTL polyfills', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.keyframes({
          from: {
            insetBlockStart: 0,
          },
          to: {
            insetBlockStart: 100,
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const name = "x1o0a6zm-B";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1o0a6zm-B",
              {
                "ltr": "@keyframes x1o0a6zm-B{from{top:0;}to{top:100px;}}",
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
