/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

jest.autoMockOff();

import { transformSync } from '@babel/core';
import { messages } from '../src/shared';
import stylexPlugin from '../src/index';

function transform(source: string, opts: any = {}) {
  return transformSync(source, {
    filename: opts.filename || 'TestTheme.stylex.js',
    parserOpts: {
      flow: 'all',
    },
    plugins: [
      [
        stylexPlugin,
        {
          unstable_moduleResolution: { type: 'commonJS' },
          ...opts,
        },
      ],
    ],
  });
}

describe('@stylexjs/babel-plugin', () => {
  /**
   * stylex.createTheme
   */

  describe('[validation] stylex.createTheme()', () => {
    test('must be bound to a variable', () => {
      expect(() => {
        transform(`
          import stylex from 'stylex';
          stylex.createTheme({__varGroupHash__: 'x568ih9'}, {});
        `);
      }).toThrow(messages.unboundCallValue('createTheme'));
    });

    test('it must have two arguments', () => {
      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme();
        `);
      }).toThrow(messages.illegalArgumentLength('createTheme', 2));

      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme({});
        `);
      }).toThrow(messages.illegalArgumentLength('createTheme', 2));

      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(genStyles(), {});
        `);
      }).toThrow(messages.nonStaticValue('createTheme'));

      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme({}, {});
        `);
      }).toThrow(
        'Can only override variables theme created with defineVars().',
      );

      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme({__varGroupHash__: 'x568ih9'}, genStyles());
        `);
      }).toThrow(messages.nonStaticValue('createTheme'));

      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme({__varGroupHash__: 'x568ih9'}, {});
        `);
      }).not.toThrow();
    });

    /* Properties */

    test('variable keys must be a static value', () => {
      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(
            {__varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)'},
            {[labelColor]: 'red',});
        `);
      }).toThrow(messages.nonStaticValue('createTheme'));
    });

    /* Values */

    test('invalid media query syntax', () => {
      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(
            {__varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)'},
            {labelColor: {default: 'red', '@media not ((not (min-width: 400px))': 'blue'}}
          );
        `);
      }).toThrow(messages.INVALID_MEDIA_QUERY_SYNTAX);
    });

    test('values must be static number or string in stylex.createTheme()', () => {
      // number
      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(
            {__varGroupHash__: 'x568ih9', cornerRadius: 'var(--cornerRadiusHash)'},
            {cornerRadius: 5,}
          );
        `);
      }).not.toThrow();

      // string
      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(
            {__varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)'},
            {labelColor: 'red',}
          );
        `);
      }).not.toThrow();

      // not static
      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(
            {__varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)'},
            {labelColor: labelColor,}
          );
        `);
      }).toThrow(messages.nonStaticValue('createTheme'));

      expect(() => {
        transform(`
          import stylex from 'stylex';
          const variables = stylex.createTheme(
            {__varGroupHash__: 'x568ih9', labelColor: 'var(--labelColorHash)'},
            {labelColor: labelColor(),}
          );
        `);
      }).toThrow(messages.nonStaticValue('createTheme'));
    });
  });
});
