/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();

const stylexPlugin = require('../src/index');
const { transformSync } = require('@babel/core');

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
  describe('[transform] stylex.viewTransitionClass', () => {
    test('basic object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const cls = stylex.viewTransitionClass({
          group: {
            transitionProperty: 'none',
          },
          imagePair: {
            borderRadius: 16,
          },
          old: {
            animationDuration: '0.5s',
          },
          new: {
            animationTimingFunction: 'ease-out',
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const cls = "xchu1hv";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xchu1hv",
              {
                "ltr": "::view-transition-group(*.xchu1hv){transition-property:none;}::view-transition-image-pair(*.xchu1hv){border-radius:16px;}::view-transition-old(*.xchu1hv){animation-duration:.5s;}::view-transition-new(*.xchu1hv){animation-timing-function:ease-out;}",
                "rtl": null,
              },
              1,
            ],
          ],
        }
      `);
    });

    test('local variables used in view transition class', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const animationDuration = '1s';
        const cls = stylex.viewTransitionClass({
          old: { animationDuration },
          new: { animationDuration },
          group: { animationDuration },
          imagePair: { animationDuration },
        });
      `);
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const animationDuration = '1s';
        const cls = "xtngzpi";"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "xtngzpi",
              {
                "ltr": "::view-transition-old(*.xtngzpi){animation-duration:1s;}::view-transition-new(*.xtngzpi){animation-duration:1s;}::view-transition-group(*.xtngzpi){animation-duration:1s;}::view-transition-image-pair(*.xtngzpi){animation-duration:1s;}",
                "rtl": null,
              },
              1,
            ],
          ],
        }
      `);
    });

    test('using keyframes', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const fadeIn = stylex.keyframes({
          from: {opacity: 0},
          to: {opacity: 1},
        });
        const fadeOut = stylex.keyframes({
          from: {opacity: 1},
          to: {opacity: 0},
        });
        const cls = stylex.viewTransitionClass({
          old: {
            animationName: fadeOut,
            animationDuration: '1s',
          },
          new: {
            animationName: fadeIn,
            animationDuration: '1s',
          },
        });
      `);
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const fadeIn = "x18re5ia-B";
        const fadeOut = "x1jn504y-B";
        const cls = "xfh0f9i";"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x18re5ia-B",
              {
                "ltr": "@keyframes x18re5ia-B{from{opacity:0;}to{opacity:1;}}",
                "rtl": null,
              },
              0,
            ],
            [
              "x1jn504y-B",
              {
                "ltr": "@keyframes x1jn504y-B{from{opacity:1;}to{opacity:0;}}",
                "rtl": null,
              },
              0,
            ],
            [
              "xfh0f9i",
              {
                "ltr": "::view-transition-old(*.xfh0f9i){animation-name:x1jn504y-B;animation-duration:1s;}::view-transition-new(*.xfh0f9i){animation-name:x18re5ia-B;animation-duration:1s;}",
                "rtl": null,
              },
              1,
            ],
          ],
        }
      `);
    });

    test('using inline keyframes', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const cls = stylex.viewTransitionClass({
          old: {
            animationName: stylex.keyframes({
              from: {opacity: 1},
              to: {opacity: 0},
            }),
            animationDuration: '1s',
          },
          new: {
            animationName: stylex.keyframes({
              from: {opacity: 0},
              to: {opacity: 1},
            }),
            animationDuration: '1s',
          },
        });
      `);
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const cls = "xfh0f9i";"
      `);
      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1jn504y-B",
              {
                "ltr": "@keyframes x1jn504y-B{from{opacity:1;}to{opacity:0;}}",
                "rtl": null,
              },
              0,
            ],
            [
              "x18re5ia-B",
              {
                "ltr": "@keyframes x18re5ia-B{from{opacity:0;}to{opacity:1;}}",
                "rtl": null,
              },
              0,
            ],
            [
              "xfh0f9i",
              {
                "ltr": "::view-transition-old(*.xfh0f9i){animation-name:x1jn504y-B;animation-duration:1s;}::view-transition-new(*.xfh0f9i){animation-name:x18re5ia-B;animation-duration:1s;}",
                "rtl": null,
              },
              1,
            ],
          ],
        }
      `);
    });

    test('unitless numbers', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const cls = stylex.viewTransitionClass({
          group: {
            animationIterationCount: 2,
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const cls = "x1cr1834";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "x1cr1834",
              {
                "ltr": "::view-transition-group(*.x1cr1834){animation-iteration-count:2;}",
                "rtl": null,
              },
              1,
            ],
          ],
        }
      `);
    });

    test.skip('using contextual styles', () => {
      const { code, metadata } = transform(`
        import * as stylex from 'stylex';
        const cls = stylex.viewTransitionClass({
          group: {
            animationDuration: {
              default: '1s',
              '@media (min-width: 800px)': '2s'
            }
          },
        });
      `);
      expect(code).toMatchInlineSnapshot();
      expect(metadata).toMatchInlineSnapshot();
    });
  });
});
