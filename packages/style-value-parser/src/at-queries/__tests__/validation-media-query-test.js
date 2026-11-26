/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { validateMediaQuery } from '../media-query';
import { MediaQueryErrors } from '../messages';

describe('style-value-parser/at-queries', () => {
  describe('[validation] media queries', () => {
    describe('MediaQuery parser with simplified errors', () => {
      const parse = validateMediaQuery;

      test('throws SYNTAX_ERROR for empty or incomplete conditions', () => {
        expect(() => parse('@media')).toThrow(MediaQueryErrors.SYNTAX_ERROR);
        expect(() => parse('@media ')).toThrow(MediaQueryErrors.SYNTAX_ERROR);
        expect(() => parse('@media ()')).toThrow(MediaQueryErrors.SYNTAX_ERROR);
        expect(() => parse('@media not (min-width: )')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (width:)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (min-width:)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (max-width: )')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media and')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
      });

      test('throws SYNTAX_ERROR for malformed expressions or invalid operators', () => {
        expect(() =>
          parse('@media (min-width: 700px and max-width: 767px)'),
        ).toThrow(MediaQueryErrors.SYNTAX_ERROR);
        expect(() =>
          parse('@media (min-width:445px; max-width:768px)'),
        ).toThrow(MediaQueryErrors.SYNTAX_ERROR);
        expect(() => parse('@media (width > )')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media ( > 600px)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (600px > width) or')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (width < )')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (width <=)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (>= width)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (300px < width < )')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
      });

      test('throws SYNTAX_ERROR for invalid or missing colon or value', () => {
        expect(() => parse('@media (width :)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (: 600px)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (width: #$%)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (width: [])')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
      });

      test('throws SYNTAX_ERROR for invalid var() usage', () => {
        expect(() => parse('@media (min-width: var(--test))')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() =>
          parse('@media (min-width: var(--foo) and (max-width: 700px))'),
        ).toThrow(MediaQueryErrors.SYNTAX_ERROR);
        expect(() =>
          parse('@media (min-width: var(foo) and (max-width: 700px))'),
        ).toThrow(MediaQueryErrors.SYNTAX_ERROR);
      });

      test('throws SYNTAX_ERROR for invalid symbols or tokens', () => {
        expect(() => parse('@media (width @ 600px)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
      });

      test('throws SYNTAX_ERROR for misused logical operators', () => {
        expect(() => parse('@media ((width: 600px) and)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media and (min-width: 600px)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media or (max-width: 1200px)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
        expect(() => parse('@media (color) and')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
      });

      test('throws SYNTAX_ERROR for unknown boolean media features', () => {
        expect(() => parse('@media (prefers-reduce-motion)')).toThrow(
          MediaQueryErrors.SYNTAX_ERROR,
        );
      });

      test('throws UNBALANCED_PARENS for unmatched parentheses', () => {
        expect(() => parse('@media (width: 600px')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media screen and (color')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media not (min-resolution: 300dpi')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media (orientation: portrait')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() =>
          parse('@media ((min-width: 300px) and (max-width: 1000px'),
        ).toThrow(MediaQueryErrors.UNBALANCED_PARENS);
        expect(() => parse('@media (hover: hover) and (pointer: fine')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media (width: calc(100% - 50px')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media (aspect-ratio: (16/9')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media screen and ((min-width: 640px')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
        expect(() => parse('@media ((prefers-color-scheme: dark)')).toThrow(
          MediaQueryErrors.UNBALANCED_PARENS,
        );
      });
    });
  });
});
