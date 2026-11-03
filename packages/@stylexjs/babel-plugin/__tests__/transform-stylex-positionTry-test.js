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
  describe('[transform] stylex.positionTry', () => {
    test('positionTry object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.positionTry({
          positionAnchor: '--anchor',
          top: '0',
          left: '0',
          width: '100px',
          height: '100px'
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const name = "--x1v351di";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "--x1v351di",
              {
                "ltr": "@position-try --x1v351di {height:100px;left:0;position-anchor:--anchor;top:0;width:100px;}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('local constants used in positionTry object', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const SIZE = '100px';
        export const name = stylex.positionTry({
          positionAnchor: '--anchor',
          top: '0',
          left: '0',
          width: SIZE,
          height: SIZE
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const SIZE = '100px';
        export const name = "--x1v351di";"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "--x1v351di",
              {
                "ltr": "@position-try --x1v351di {height:100px;left:0;position-anchor:--anchor;top:0;width:100px;}",
                "rtl": null,
              },
              0,
            ],
          ],
        }
      `);
    });

    test('positionTry value used within create', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        const SIZE = '100px';
        const name = stylex.positionTry({
          top: '0',
          left: '0',
          width: SIZE,
          height: SIZE
        });
        export const styles = stylex.create({
          root: {
            positionTryFallbacks: name,
          }
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const SIZE = '100px';
        const name = "--x1lerlyv";
        export const styles = {
          root: {
            k9M3vk: "x188f6ho",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "--x1lerlyv",
              {
                "ltr": "@position-try --x1lerlyv {height:100px;left:0;top:0;width:100px;}",
                "rtl": null,
              },
              0,
            ],
            [
              "x188f6ho",
              {
                "ltr": ".x188f6ho{position-try-fallbacks:--x1lerlyv}",
                "rtl": null,
              },
              3000,
            ],
          ],
        }
      `);
    });

    test('positionTry object used inline', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({
          root: {
            positionTryFallbacks: stylex.positionTry({
              positionAnchor: '--anchor',
              top: '0',
              left: '0',
              width: '100px',
              height: '100px'
            }),
          },
        });
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const styles = {
          root: {
            k9M3vk: "x1kvy05d",
            $$css: true
          }
        };"
      `);

      expect(metadata).toMatchInlineSnapshot(`
        {
          "stylex": [
            [
              "--x1v351di",
              {
                "ltr": "@position-try --x1v351di {height:100px;left:0;position-anchor:--anchor;top:0;width:100px;}",
                "rtl": null,
              },
              0,
            ],
            [
              "x1kvy05d",
              {
                "ltr": ".x1kvy05d{position-try-fallbacks:--x1v351di}",
                "rtl": null,
              },
              3000,
            ],
          ],
        }
      `);
    });

    test('processStylexRules does not inject ancestor selector into @position-try', () => {
      const { metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.positionTry({
          positionAnchor: '--anchor',
          top: '0',
          left: '0',
          width: '100px',
          height: '100px'
        });
      `);

      const css = stylexPlugin.default.processStylexRules(metadata.stylex, {
        enableLTRRTLComments: false,
      });

      expect(css).not.toContain("html[dir='rtl'] height");
      expect(css).not.toContain("html[dir='rtl'] width");
      expect(css).toContain('@position-try --x1v351di {height:');
    });

    test('processStylexRules does not inject specificity into @position-try', () => {
      const { metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const name = stylex.positionTry({
          positionAnchor: '--anchor',
          top: '0',
          left: '0',
          width: '100px',
          height: '100px'
        });
      `);

      const css = stylexPlugin.default.processStylexRules(metadata.stylex, {
        useLayers: false,
        legacyDisableLayers: false,
      });

      expect(css).not.toContain('@position-try --x1v351di :not(#\\#)');
      expect(css).toContain('@position-try --x1v351di {height:');
    });
  });
});
