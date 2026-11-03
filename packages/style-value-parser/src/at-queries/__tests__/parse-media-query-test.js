/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { MediaQuery } from '../media-query.js';

describe('style-value-parser/at-queries', () => {
  describe('[parse] media queries', () => {
    describe('keywords', () => {
      test('@media screen', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media screen');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "screen",
              "not": false,
              "only": false,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media screen"');
      });

      test('@media print', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media print');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "print",
              "not": false,
              "only": false,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media print"');
      });

      test('@media all', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media all');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "all",
              "not": false,
              "only": false,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media all"');
      });

      test('@media only screen', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media only screen');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "screen",
              "not": false,
              "only": true,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media only screen"');
      });

      test('@media only print and (color)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media only print and (color)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "print",
                  "not": false,
                  "only": true,
                  "type": "media-keyword",
                },
                {
                  "keyValue": "color",
                  "type": "word-rule",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media only print and (color)"',
        );
      });

      test('@media only screen and (max-width: 38em)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media only screen and (max-width: 38em)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "screen",
                  "not": false,
                  "only": true,
                  "type": "media-keyword",
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "em",
                    "value": 38,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media only screen and (max-width: 38em)"',
        );
      });

      test('@media not screen', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media not screen');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "screen",
              "not": true,
              "only": false,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media not screen"');
      });

      test('@media not all and (monochrome)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media not all and (monochrome)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "all",
              "not": true,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media not all"');
      });
    });

    describe('pair rule', () => {
      test('@media (width: 100px)', () => {
        const input = '@media (width: 100px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "px",
                "value": 100,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (width: 100px)"',
        );
      });

      test('@media (max-width: 50em)', () => {
        const input = '@media (max-width: 50em)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "max-width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "em",
                "value": 50,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (max-width: 50em)"',
        );
      });

      test('@media (orientation: landscape)', () => {
        const input = '@media (orientation: landscape)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "orientation",
              "type": "pair",
              "value": "landscape",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (orientation: landscape)"',
        );
      });

      test('@media (update: fast)', () => {
        const input = '@media (update: fast)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "update",
              "type": "pair",
              "value": "fast",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (update: fast)"',
        );
      });

      test('@media (overflow-block: scroll)', () => {
        const input = '@media (overflow-block: scroll)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "overflow-block",
              "type": "pair",
              "value": "scroll",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (overflow-block: scroll)"',
        );
      });

      test('@media (display-mode: fullscreen)', () => {
        const input = '@media (display-mode: fullscreen)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "display-mode",
              "type": "pair",
              "value": "fullscreen",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (display-mode: fullscreen)"',
        );
      });

      test('@media (scripting: enabled)', () => {
        const input = '@media (scripting: enabled)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "scripting",
              "type": "pair",
              "value": "enabled",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (scripting: enabled)"',
        );
      });

      test('@media (hover: hover)', () => {
        const input = '@media (hover: hover)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "hover",
              "type": "pair",
              "value": "hover",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (hover: hover)"',
        );
      });

      test('@media (any-hover: none)', () => {
        const input = '@media (any-hover: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "any-hover",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (any-hover: none)"',
        );
      });

      test('@media (pointer: coarse)', () => {
        const input = '@media (pointer: coarse)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "pointer",
              "type": "pair",
              "value": "coarse",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (pointer: coarse)"',
        );
      });

      test('@media (any-pointer: fine)', () => {
        const input = '@media (any-pointer: fine)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "any-pointer",
              "type": "pair",
              "value": "fine",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (any-pointer: fine)"',
        );
      });

      test('@media (light-level: dim)', () => {
        const input = '@media (light-level: dim)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "light-level",
              "type": "pair",
              "value": "dim",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (light-level: dim)"',
        );
      });

      test('@media (inverted-colors: inverted)', () => {
        const input = '@media (inverted-colors: inverted)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "inverted-colors",
              "type": "pair",
              "value": "inverted",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (inverted-colors: inverted)"',
        );
      });

      test('@media (prefers-reduced-motion: reduce)', () => {
        const input = '@media (prefers-reduced-motion: reduce)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-reduced-motion",
              "type": "pair",
              "value": "reduce",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-reduced-motion: reduce)"',
        );
      });

      test('@media (prefers-contrast: more)', () => {
        const input = '@media (prefers-contrast: more)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-contrast",
              "type": "pair",
              "value": "more",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-contrast: more)"',
        );
      });

      test('@media (forced-colors: active)', () => {
        const input = '@media (forced-colors: active)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "forced-colors",
              "type": "pair",
              "value": "active",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (forced-colors: active)"',
        );
      });

      test('@media (prefers-reduced-transparency: reduce)', () => {
        const input = '@media (prefers-reduced-transparency: reduce)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-reduced-transparency",
              "type": "pair",
              "value": "reduce",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-reduced-transparency: reduce)"',
        );
      });

      test('@media (min-width: calc(300px + 5em))', () => {
        const input = '@media (min-width: calc(300px + 5em))';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-width",
              "type": "pair",
              "value": "calc(300px + 5em)",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: calc(300px + 5em))"',
        );
      });

      test('@media (max-height: calc(100vh - 50px))', () => {
        const input = '@media (max-height: calc(100vh - 50px))';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "max-height",
              "type": "pair",
              "value": "calc(100vh - 50px)",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (max-height: calc(100vh - 50px))"',
        );
      });

      test('@media (aspect-ratio: 16 / 9)', () => {
        const input = '@media (aspect-ratio: 16 / 9)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "aspect-ratio",
              "type": "pair",
              "value": [
                16,
                "/",
                9,
              ],
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (aspect-ratio: 16 / 9)"',
        );
      });

      test('@media (device-aspect-ratio: 16 / 9)', () => {
        const input = '@media (device-aspect-ratio: 16 / 9)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "device-aspect-ratio",
              "type": "pair",
              "value": [
                16,
                "/",
                9,
              ],
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (device-aspect-ratio: 16 / 9)"',
        );
      });

      test('@media (min-resolution: 150dpi)', () => {
        const input = '@media (min-resolution: 150dpi)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-resolution",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "dpi",
                "value": 150,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-resolution: 150dpi)"',
        );
      });

      test('@media (max-resolution: 600dppx)', () => {
        const input = '@media (max-resolution: 600dppx)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "max-resolution",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "dppx",
                "value": 600,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (max-resolution: 600dppx)"',
        );
      });

      test('@media (color-gamut: srgb)', () => {
        const input = '@media (color-gamut: srgb)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "color-gamut",
              "type": "pair",
              "value": "srgb",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (color-gamut: srgb)"',
        );
      });

      test('@media (display-mode: standalone)', () => {
        const input = '@media (display-mode: standalone)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "display-mode",
              "type": "pair",
              "value": "standalone",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (display-mode: standalone)"',
        );
      });

      test('@media (prefers-color-scheme: dark)', () => {
        const input = '@media (prefers-color-scheme: dark)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-color-scheme",
              "type": "pair",
              "value": "dark",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-color-scheme: dark)"',
        );
      });

      test('@media (scripting: none)', () => {
        const input = '@media (scripting: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "scripting",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (scripting: none)"',
        );
      });

      test('@media (update: slow)', () => {
        const input = '@media (update: slow)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "update",
              "type": "pair",
              "value": "slow",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (update: slow)"',
        );
      });

      test('@media (overflow-inline: none)', () => {
        const input = '@media (overflow-inline: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "overflow-inline",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (overflow-inline: none)"',
        );
      });

      test('@media (display-mode: minimal-ui)', () => {
        const input = '@media (display-mode: minimal-ui)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "display-mode",
              "type": "pair",
              "value": "minimal-ui",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (display-mode: minimal-ui)"',
        );
      });

      test('@media (hover: none)', () => {
        const input = '@media (hover: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "hover",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (hover: none)"',
        );
      });

      test('@media (any-hover: hover)', () => {
        const input = '@media (any-hover: hover)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "any-hover",
              "type": "pair",
              "value": "hover",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (any-hover: hover)"',
        );
      });

      test('@media (pointer: none)', () => {
        const input = '@media (pointer: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "pointer",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (pointer: none)"',
        );
      });

      test('@media (any-pointer: none)', () => {
        const input = '@media (any-pointer: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "any-pointer",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (any-pointer: none)"',
        );
      });

      test('@media (light-level: washed)', () => {
        const input = '@media (light-level: washed)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "light-level",
              "type": "pair",
              "value": "washed",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (light-level: washed)"',
        );
      });

      test('@media (inverted-colors: none)', () => {
        const input = '@media (inverted-colors: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "inverted-colors",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (inverted-colors: none)"',
        );
      });

      test('@media (prefers-reduced-motion: no-preference)', () => {
        const input = '@media (prefers-reduced-motion: no-preference)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-reduced-motion",
              "type": "pair",
              "value": "no-preference",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-reduced-motion: no-preference)"',
        );
      });

      test('@media (prefers-contrast: no-preference)', () => {
        const input = '@media (prefers-contrast: no-preference)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-contrast",
              "type": "pair",
              "value": "no-preference",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-contrast: no-preference)"',
        );
      });

      test('@media (forced-colors: none)', () => {
        const input = '@media (forced-colors: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "forced-colors",
              "type": "pair",
              "value": "none",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (forced-colors: none)"',
        );
      });

      test('@media (prefers-reduced-transparency: no-preference)', () => {
        const input = '@media (prefers-reduced-transparency: no-preference)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "prefers-reduced-transparency",
              "type": "pair",
              "value": "no-preference",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-reduced-transparency: no-preference)"',
        );
      });
    });

    describe('word-rule', () => {
      test('@media (color)', () => {
        const input = '@media (color)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "keyValue": "color",
              "type": "word-rule",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media (color)"');
      });

      test('@media (color-index)', () => {
        const input = '@media (color-index)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "keyValue": "color-index",
              "type": "word-rule",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (color-index)"',
        );
      });

      test('@media (monochrome)', () => {
        const input = '@media (monochrome)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "keyValue": "monochrome",
              "type": "word-rule",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (monochrome)"',
        );
      });

      test('@media (grid)', () => {
        const input = '@media (grid)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "keyValue": "grid",
              "type": "word-rule",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media (grid)"');
      });
    });

    describe('and combinator', () => {
      test('@media not all and (monochrome)', () => {
        const input = '@media not all and (monochrome)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "all",
              "not": true,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media not all"');
      });

      test('@media screen and (min-width: 400px)', () => {
        const input = '@media screen and (min-width: 400px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "screen",
                  "not": false,
                  "only": false,
                  "type": "media-keyword",
                },
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media screen and (min-width: 400px)"',
        );
      });

      test('@media (min-height: 600px) and (orientation: landscape)', () => {
        const input = '@media (min-height: 600px) and (orientation: landscape)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-height",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 600,
                  },
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-height: 600px) and (orientation: landscape)"',
        );
      });

      test('@media screen and (device-aspect-ratio: 16/9)', () => {
        const input = '@media screen and (device-aspect-ratio: 16/9)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "screen",
                  "not": false,
                  "only": false,
                  "type": "media-keyword",
                },
                {
                  "key": "device-aspect-ratio",
                  "type": "pair",
                  "value": [
                    16,
                    "/",
                    9,
                  ],
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media screen and (device-aspect-ratio: 16 / 9)"',
        );
      });

      test('@media (min-aspect-ratio: 3/2) and (max-aspect-ratio: 16/9)', () => {
        const input =
          '@media (min-aspect-ratio: 3/2) and (max-aspect-ratio: 16/9)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-aspect-ratio",
                  "type": "pair",
                  "value": [
                    3,
                    "/",
                    2,
                  ],
                },
                {
                  "key": "max-aspect-ratio",
                  "type": "pair",
                  "value": [
                    16,
                    "/",
                    9,
                  ],
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-aspect-ratio: 3 / 2) and (max-aspect-ratio: 16 / 9)"',
        );
      });

      test('@media (min-resolution: 300dpi) and (max-resolution: 600dpi)', () => {
        const input =
          '@media (min-resolution: 300dpi) and (max-resolution: 600dpi)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-resolution",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "dpi",
                    "value": 300,
                  },
                },
                {
                  "key": "max-resolution",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "dpi",
                    "value": 600,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-resolution: 300dpi) and (max-resolution: 600dpi)"',
        );
      });

      test('@media (min-width: 768px) and (max-width: 991px)', () => {
        const input = '@media (min-width: 768px) and (max-width: 991px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 768,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 991,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 768px) and (max-width: 991px)"',
        );
      });

      test('@media (min-width: 1200px) and (orientation: landscape)', () => {
        const input = '@media (min-width: 1200px) and (orientation: landscape)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 1200,
                  },
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 1200px) and (orientation: landscape)"',
        );
      });

      test('@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)', () => {
        const input =
          '@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 992,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 1199,
                  },
                },
                {
                  "key": "pointer",
                  "type": "pair",
                  "value": "fine",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 992px) and (max-width: 1199px) and (pointer: fine)"',
        );
      });

      test('@media (min-width: 576px) and (max-width: 767px) and (hover: none)', () => {
        const input =
          '@media (min-width: 576px) and (max-width: 767px) and (hover: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 576,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 767,
                  },
                },
                {
                  "key": "hover",
                  "type": "pair",
                  "value": "none",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 576px) and (max-width: 767px) and (hover: none)"',
        );
      });

      test('@media (orientation: landscape) and (pointer: fine)', () => {
        const input = '@media (orientation: landscape) and (pointer: fine)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
                {
                  "key": "pointer",
                  "type": "pair",
                  "value": "fine",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (orientation: landscape) and (pointer: fine)"',
        );
      });

      test('@media (prefers-reduced-motion: reduce) and (update: slow)', () => {
        const input =
          '@media (prefers-reduced-motion: reduce) and (update: slow)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "prefers-reduced-motion",
                  "type": "pair",
                  "value": "reduce",
                },
                {
                  "key": "update",
                  "type": "pair",
                  "value": "slow",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (prefers-reduced-motion: reduce) and (update: slow)"',
        );
      });

      test('@media (orientation: landscape) and (update: fast)', () => {
        const input = '@media (orientation: landscape) and (update: fast)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
                {
                  "key": "update",
                  "type": "pair",
                  "value": "fast",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (orientation: landscape) and (update: fast)"',
        );
      });
    });

    describe('or combinator', () => {
      test('@media (orientation: portrait), (orientation: landscape)', () => {
        const input =
          '@media (orientation: portrait), (orientation: landscape)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "portrait",
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (orientation: portrait), (orientation: landscape)"',
        );
      });

      test('@media (min-width: 500px) or (max-width: 600px)', () => {
        const input = '@media (min-width: 500px) or (max-width: 600px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 500,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 600,
                  },
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 500px), (max-width: 600px)"',
        );
      });

      test('@media (width: 500px), (height: 400px)', () => {
        const input = '@media (width: 500px), (height: 400px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 500,
                  },
                },
                {
                  "key": "height",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (width: 500px), (height: 400px)"',
        );
      });

      test('@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)', () => {
        const input =
          '@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 576,
                  },
                },
                {
                  "rules": [
                    {
                      "key": "orientation",
                      "type": "pair",
                      "value": "portrait",
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "signCharacter": undefined,
                        "type": "integer",
                        "unit": "px",
                        "value": 767,
                      },
                    },
                  ],
                  "type": "and",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 576px), (orientation: portrait) and (max-width: 767px)"',
        );
      });

      test('@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)', () => {
        const input =
          '@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 768,
                      },
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 991,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 768px) and (max-width: 991px), (orientation: landscape)"',
        );
      });

      test('@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)', () => {
        const input =
          '@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 992,
                      },
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 1199,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "pointer",
                      "type": "pair",
                      "value": "fine",
                    },
                    {
                      "key": "hover",
                      "type": "pair",
                      "value": "hover",
                    },
                  ],
                  "type": "and",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover)"',
        );
      });

      test('@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)', () => {
        const input =
          '@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 576,
                      },
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 767,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "hover",
                      "type": "pair",
                      "value": "none",
                    },
                    {
                      "key": "any-pointer",
                      "type": "pair",
                      "value": "coarse",
                    },
                  ],
                  "type": "and",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse)"',
        );
      });

      test('@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)', () => {
        const input =
          '@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 576,
                  },
                },
                {
                  "rules": [
                    {
                      "key": "orientation",
                      "type": "pair",
                      "value": "portrait",
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "signCharacter": undefined,
                        "type": "integer",
                        "unit": "px",
                        "value": 767,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "key": "prefers-color-scheme",
                  "type": "pair",
                  "value": "dark",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 576px), (orientation: portrait) and (max-width: 767px), (prefers-color-scheme: dark)"',
        );
      });

      test('@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)', () => {
        const input =
          '@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 768,
                      },
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 991,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "orientation",
                      "type": "pair",
                      "value": "landscape",
                    },
                    {
                      "key": "update",
                      "type": "pair",
                      "value": "fast",
                    },
                  ],
                  "type": "and",
                },
                {
                  "key": "prefers-reduced-motion",
                  "type": "pair",
                  "value": "reduce",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 768px) and (max-width: 991px), (orientation: landscape) and (update: fast), (prefers-reduced-motion: reduce)"',
        );
      });

      test('@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)', () => {
        const input =
          '@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 992,
                      },
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 1199,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "pointer",
                      "type": "pair",
                      "value": "fine",
                    },
                    {
                      "key": "hover",
                      "type": "pair",
                      "value": "hover",
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "any-pointer",
                      "type": "pair",
                      "value": "coarse",
                    },
                    {
                      "key": "any-hover",
                      "type": "pair",
                      "value": "none",
                    },
                  ],
                  "type": "and",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 992px) and (max-width: 1199px), (pointer: fine) and (hover: hover), (any-pointer: coarse) and (any-hover: none)"',
        );
      });

      test('@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)', () => {
        const input =
          '@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 576,
                      },
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "type": "integer",
                        "unit": "px",
                        "value": 767,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "hover",
                      "type": "pair",
                      "value": "none",
                    },
                    {
                      "key": "any-pointer",
                      "type": "pair",
                      "value": "coarse",
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "prefers-reduced-transparency",
                      "type": "pair",
                      "value": "reduce",
                    },
                    {
                      "key": "forced-colors",
                      "type": "pair",
                      "value": "active",
                    },
                  ],
                  "type": "and",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 576px) and (max-width: 767px), (hover: none) and (any-pointer: coarse), (prefers-reduced-transparency: reduce) and (forced-colors: active)"',
        );
      });

      test('@media (color) and (min-width: 400px), screen and (max-width: 700px)', () => {
        const input =
          '@media (color) and (min-width: 400px), screen and (max-width: 700px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "rules": [
                    {
                      "keyValue": "color",
                      "type": "word-rule",
                    },
                    {
                      "key": "min-width",
                      "type": "pair",
                      "value": {
                        "signCharacter": undefined,
                        "type": "integer",
                        "unit": "px",
                        "value": 400,
                      },
                    },
                  ],
                  "type": "and",
                },
                {
                  "rules": [
                    {
                      "key": "screen",
                      "not": false,
                      "only": false,
                      "type": "media-keyword",
                    },
                    {
                      "key": "max-width",
                      "type": "pair",
                      "value": {
                        "signCharacter": undefined,
                        "type": "integer",
                        "unit": "px",
                        "value": 700,
                      },
                    },
                  ],
                  "type": "and",
                },
              ],
              "type": "or",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (color) and (min-width: 400px), screen and (max-width: 700px)"',
        );
      });
    });

    describe('not combinator', () => {
      test('@media not (not (not (min-width: 400px)))', () => {
        const input = '@media not (not (not (min-width: 400px)))';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rule": {
                "key": "min-width",
                "type": "pair",
                "value": {
                  "signCharacter": undefined,
                  "type": "integer",
                  "unit": "px",
                  "value": 400,
                },
              },
              "type": "not",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (not (min-width: 400px))"',
        );
      });

      test('@media not ((min-width: 500px) and (max-width: 600px) and (max-width: 400px))', () => {
        const input =
          '@media not ((min-width: 500px) and (max-width: 600px) and (max-width: 400px))';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "all",
              "not": false,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media all"');
      });

      test('@media not all and (monochrome) and (min-width: 600px)', () => {
        const input = '@media not all and (monochrome) and (min-width: 600px)';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "all",
              "not": true,
              "type": "media-keyword",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot('"@media not all"');
      });

      test('@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))', () => {
        const input =
          '@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))';
        const parsed = MediaQuery.parser.parseToEnd(input);
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 1024.01,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 1440,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 1024.01px) and (max-width: 1440px)"',
        );
      });
    });
  });
  describe('[normalize] media queries', () => {
    describe('flatten and combinator logic', () => {
      test('flattens nested and rules', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (min-width: 400px) and ((max-width: 700px) and (orientation: landscape))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape)"',
        );
      });

      test('flattens complex nested and rules', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media ((min-width: 400px) and ((max-width: 700px) and (orientation: landscape)))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape)"',
        );
      });

      test('flattens deeply nested and chains', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (((min-width: 400px) and (max-width: 700px)) and ((orientation: landscape) and (hover: hover)))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
                {
                  "key": "orientation",
                  "type": "pair",
                  "value": "landscape",
                },
                {
                  "key": "hover",
                  "type": "pair",
                  "value": "hover",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px) and (max-width: 700px) and (orientation: landscape) and (hover: hover)"',
        );
      });

      test('handles top-level and and nested and', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media screen and ((min-width: 500px) and ((max-width: 800px) and (color)))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "screen",
                  "not": false,
                  "only": false,
                  "type": "media-keyword",
                },
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 500,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "signCharacter": undefined,
                    "type": "integer",
                    "unit": "px",
                    "value": 800,
                  },
                },
                {
                  "keyValue": "color",
                  "type": "word-rule",
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media screen and (min-width: 500px) and (max-width: 800px) and (color)"',
        );
      });
    });

    describe('simplify not combinator logic', () => {
      test('removes duplicate nots', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media not (not (min-width: 400px))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "px",
                "value": 400,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px)"',
        );
      });

      test('removes triple negation', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media not (not (not (hover: hover)))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rule": {
                "key": "hover",
                "type": "pair",
                "value": "hover",
              },
              "type": "not",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (not (hover: hover))"',
        );
      });

      test('normalizes not with compound expression', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media not ((min-width: 600px) and (max-width: 900px))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rule": {
                "rules": [
                  {
                    "key": "min-width",
                    "type": "pair",
                    "value": {
                      "type": "integer",
                      "unit": "px",
                      "value": 600,
                    },
                  },
                  {
                    "key": "max-width",
                    "type": "pair",
                    "value": {
                      "type": "integer",
                      "unit": "px",
                      "value": 900,
                    },
                  },
                ],
                "type": "and",
              },
              "type": "not",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (not ((min-width: 600px) and (max-width: 900px)))"',
        );
      });

      test('removes even number of nots', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media not (not (not (not (update: fast))))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "update",
              "type": "pair",
              "value": "fast",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (update: fast)"',
        );
      });

      test('preserves single not over group', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media not ((pointer: fine) and (hover: hover))',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rule": {
                "rules": [
                  {
                    "key": "pointer",
                    "type": "pair",
                    "value": "fine",
                  },
                  {
                    "key": "hover",
                    "type": "pair",
                    "value": "hover",
                  },
                ],
                "type": "and",
              },
              "type": "not",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (not ((pointer: fine) and (hover: hover)))"',
        );
      });
    });

    describe('inequality rule tests', () => {
      test('@media (width > 400px)', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media (width > 400px)');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "px",
                "value": 400.01,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400.01px)"',
        );
      });

      test('@media (width >= 400px)', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media (width >= 400px)');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "px",
                "value": 400,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px)"',
        );
      });

      test('@media (400px < width)', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media (400px < width)');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "px",
                "value": 400.01,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400.01px)"',
        );
      });

      test('@media (400px <= width)', () => {
        const parsed = MediaQuery.parser.parseToEnd('@media (400px <= width)');
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "key": "min-width",
              "type": "pair",
              "value": {
                "signCharacter": undefined,
                "type": "integer",
                "unit": "px",
                "value": 400,
              },
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px)"',
        );
      });

      test('@media (1000px <= width <= 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (400px <= width <= 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px) and (max-width: 700px)"',
        );
      });

      test('@media (400px < width <= 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (400px < width <= 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 400.01,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400.01px) and (max-width: 700px)"',
        );
      });

      test('@media (400px <= width <= 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (400px <= width <= 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 400,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 400px) and (max-width: 700px)"',
        );
      });

      test('@media (1000px >= width >= 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (1000px >= width >= 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 1000,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 700px) and (max-width: 1000px)"',
        );
      });

      test('@media (1000px > width >= 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (1000px > width >= 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 999.99,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 700px) and (max-width: 999.99px)"',
        );
      });

      test('@media (1000px >= width > 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (1000px >= width > 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700.01,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 1000,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 700.01px) and (max-width: 1000px)"',
        );
      });

      test('@media (1000px > width > 700px)', () => {
        const parsed = MediaQuery.parser.parseToEnd(
          '@media (1000px > width > 700px)',
        );
        expect(parsed).toMatchInlineSnapshot(`
          MediaQuery {
            "queries": {
              "rules": [
                {
                  "key": "min-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 700.01,
                  },
                },
                {
                  "key": "max-width",
                  "type": "pair",
                  "value": {
                    "type": "integer",
                    "unit": "px",
                    "value": 999.99,
                  },
                },
              ],
              "type": "and",
            },
          }
        `);
        expect(parsed.toString()).toMatchInlineSnapshot(
          '"@media (min-width: 700.01px) and (max-width: 999.99px)"',
        );
      });
    });
  });
  describe('simplify range intervals', () => {
    test('@media (min-width: 100px) and not (max-width: 200px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (not (max-width: 200px))',
      );
      expect(parsed.toString()).toBe('@media (min-width: 200.01px)');
    });

    test('@media (min-width: 100px) and (min-width: 200px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (min-width: 200px)',
      );
      expect(parsed.toString()).toBe('@media (min-width: 200px)');
    });

    test('@media (max-width: 200px) and not (max-width: 300px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (not (max-width: 200px)) and (not (max-width: 300px))',
      );
      expect(parsed.toString()).toBe('@media (min-width: 300.01px)');
    });

    test('@media (min-width: 100px) and not (max-width: 50px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (not (max-width: 50px))',
      );
      expect(parsed.toString()).toBe('@media (min-width: 100px)');
    });

    test('@media (min-width: 100px) and not (max-width: 100px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (not (max-width: 100px))',
      );
      expect(parsed.toString()).toBe('@media (min-width: 100.01px)');
    });

    test('@media (min-width: 100px) and not (max-width: 99.99px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (not (max-width: 99.99px))',
      );
      expect(parsed.toString()).toBe('@media (min-width: 100px)');
    });

    test('@media (min-width: 100px) and not (max-width: 99.99px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (not (max-width: 99.99px))',
      );
      expect(parsed.toString()).toBe('@media (min-width: 100px)');
    });

    test('@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (max-width: 1440px) and (not (max-width: 1024px)) and (not (max-width: 768px)) and (not (max-width: 458px))',
      );
      expect(parsed.toString()).toBe(
        '@media (min-width: 1024.01px) and (max-width: 1440px)',
      );
    });

    test('@media (min-width: 100px) and (max-width: 500px) and (not (min-width: 600px)) => always-false', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (max-width: 500px) and (not (min-width: 600px))',
      );
      expect(parsed.toString()).toBe(
        '@media (min-width: 100px) and (max-width: 500px)',
      );
    });

    test('@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px)) => (min-width: 200.01px) and (max-width: 500px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px))',
      );
      expect(parsed.toString()).toBe(
        '@media (min-width: 200.01px) and (max-width: 500px)',
      );
    });

    test('@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px)) and (not (min-width: 400px)) => (min-width: 200.01px) and (max-width: 399.99px)', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (max-width: 500px) and (not (max-width: 200px)) and (not (min-width: 400px))',
      );
      expect(parsed.toString()).toBe(
        '@media (min-width: 200.01px) and (max-width: 399.98px)',
      );
    });

    test('@media (min-width: 100px) and (orientation: landscape) => should not simplify', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: 100px) and (orientation: landscape)',
      );
      expect(parsed.toString()).toBe(
        '@media (min-width: 100px) and (orientation: landscape)',
      );
    });

    test('@media (min-width: calc(100px + 2em)) and (max-width: 500px) => should not simplify', () => {
      const parsed = MediaQuery.parser.parseToEnd(
        '@media (min-width: calc(100px + 2em)) and (max-width: 500px)',
      );
      expect(parsed.toString()).toBe(
        '@media (min-width: calc(100px + 2em)) and (max-width: 500px)',
      );
    });
  });

  describe('additional media query features', () => {
    test('@media (-webkit-min-device-pixel-ratio: 2)', () => {
      const input = '@media (-webkit-min-device-pixel-ratio: 2)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "-webkit-min-device-pixel-ratio",
            "type": "pair",
            "value": 2,
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (-webkit-min-device-pixel-ratio: 2)"',
      );
    });

    test('@media (-webkit-max-device-pixel-ratio: 1.5)', () => {
      const input = '@media (-webkit-max-device-pixel-ratio: 1.5)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "-webkit-max-device-pixel-ratio",
            "type": "pair",
            "value": 1.5,
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (-webkit-max-device-pixel-ratio: 1.5)"',
      );
    });

    test('@media (color-index: 256)', () => {
      const input = '@media (color-index: 256)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "color-index",
            "type": "pair",
            "value": 256,
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (color-index: 256)"',
      );
    });

    test('@media (min-color-index: 256)', () => {
      const input = '@media (min-color-index: 256)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "min-color-index",
            "type": "pair",
            "value": 256,
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (min-color-index: 256)"',
      );
    });

    test('@media (max-color-index: 65536)', () => {
      const input = '@media (max-color-index: 65536)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "max-color-index",
            "type": "pair",
            "value": 65536,
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (max-color-index: 65536)"',
      );
    });

    test('@media (resolution: 300dpi)', () => {
      const input = '@media (resolution: 300dpi)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "resolution",
            "type": "pair",
            "value": {
              "signCharacter": undefined,
              "type": "integer",
              "unit": "dpi",
              "value": 300,
            },
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (resolution: 300dpi)"',
      );
    });

    test('@media (resolution: 2dppx)', () => {
      const input = '@media (resolution: 2dppx)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "resolution",
            "type": "pair",
            "value": {
              "signCharacter": undefined,
              "type": "integer",
              "unit": "dppx",
              "value": 2,
            },
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (resolution: 2dppx)"',
      );
    });

    test('@media (resolution: 1.5dpcm)', () => {
      const input = '@media (resolution: 1.5dpcm)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "resolution",
            "type": "pair",
            "value": {
              "signCharacter": undefined,
              "type": "number",
              "unit": "dpcm",
              "value": 1.5,
            },
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (resolution: 1.5dpcm)"',
      );
    });

    test('@media (color-gamut: p3)', () => {
      const input = '@media (color-gamut: p3)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "color-gamut",
            "type": "pair",
            "value": "p3",
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (color-gamut: p3)"',
      );
    });

    test('@media (color-gamut: rec2020)', () => {
      const input = '@media (color-gamut: rec2020)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "color-gamut",
            "type": "pair",
            "value": "rec2020",
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (color-gamut: rec2020)"',
      );
    });

    test('@media (video-color-gamut: srgb)', () => {
      const input = '@media (video-color-gamut: srgb)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "video-color-gamut",
            "type": "pair",
            "value": "srgb",
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (video-color-gamut: srgb)"',
      );
    });

    test('@media (video-color-gamut: p3)', () => {
      const input = '@media (video-color-gamut: p3)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "video-color-gamut",
            "type": "pair",
            "value": "p3",
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (video-color-gamut: p3)"',
      );
    });

    test('@media (video-color-gamut: rec2020)', () => {
      const input = '@media (video-color-gamut: rec2020)';
      const parsed = MediaQuery.parser.parseToEnd(input);
      expect(parsed).toMatchInlineSnapshot(`
        MediaQuery {
          "queries": {
            "key": "video-color-gamut",
            "type": "pair",
            "value": "rec2020",
          },
        }
      `);
      expect(parsed.toString()).toMatchInlineSnapshot(
        '"@media (video-color-gamut: rec2020)"',
      );
    });
  });
});
