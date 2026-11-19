/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.disableAutomock();

const { RuleTester: ESLintTester } = require('eslint');
const rule = require('../src/stylex-no-raw-media-queries');

const eslintTester = new ESLintTester({
  parser: require.resolve('hermes-eslint'),
  parserOptions: {
    ecmaVersion: 6,
    sourceType: 'module',
  },
});

const errorMessage =
  'Raw media query strings are not allowed. Define this query with `stylex.defineConsts` and use the shared constant instead. See https://stylexjs.com/docs/api/javascript/defineConsts/';

eslintTester.run('stylex-no-raw-media-queries', rule.default, {
  valid: [
    {
      // media query conditions referencing shared constants
      code: `
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './breakpoints.stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              [breakpoints.small]: '50%',
            },
          },
        });
      `,
    },
    {
      // template literal keys containing expressions reference constants
      code: `
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './breakpoints.stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              [\`\${breakpoints.small}\`]: '50%',
            },
          },
        });
      `,
    },
    {
      // dynamic styles referencing shared constants
      code: `
        import * as stylex from '@stylexjs/stylex';
        import { breakpoints } from './breakpoints.stylex';
        const styles = stylex.create({
          root: (width) => ({
            width: {
              default: width,
              [breakpoints.small]: '50%',
            },
          }),
        });
      `,
    },
    {
      // pseudo classes and default conditions are unaffected
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            color: {
              default: 'black',
              ':hover': 'blue',
            },
          },
        });
      `,
    },
    {
      // raw media queries as *values* (not keys) are fine — this is how
      // constants are defined in the first place
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const breakpoints = stylex.defineConsts({
          small: '@media (max-width: 600px)',
        });
      `,
    },
    {
      // create calls from untracked imports are ignored
      code: `
        import * as stylex from 'other-library';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              '@media (min-width: 600px)': '50%',
            },
          },
        });
      `,
    },
    {
      // queries listed in allowedMediaQueries are permitted
      options: [{ allowedMediaQueries: ['@media print'] }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            display: {
              default: 'flex',
              '@media print': 'none',
            },
          },
        });
      `,
    },
  ],
  invalid: [
    {
      // raw media query condition in the modern syntax
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              '@media (min-width: 600px)': '50%',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // raw media query in the legacy contextual syntax
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            '@media (min-width: 600px)': {
              width: '50%',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // nested raw media queries are each reported
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              '@media (min-width: 600px)': {
                default: '50%',
                '@media screen': '40%',
              },
            },
          },
        });
      `,
      errors: [{ message: errorMessage }, { message: errorMessage }],
    },
    {
      // raw media queries inside dynamic styles are reported
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: (width) => ({
            width: {
              default: width,
              '@media (min-width: 600px)': '50%',
            },
          }),
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // template literal keys without expressions are still raw strings
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              [\`@media (min-width: 600px)\`]: '50%',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // named create imports are tracked too
      code: `
        import { create } from '@stylexjs/stylex';
        const styles = create({
          root: {
            width: {
              default: '100%',
              '@media (min-width: 600px)': '50%',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // at-rule matching is case-insensitive
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              '@MEDIA (min-width: 600px)': '50%',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // custom import sources are respected
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        const styles = stylex.create({
          root: {
            width: {
              default: '100%',
              '@media (min-width: 600px)': '50%',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
    {
      // allowedMediaQueries only permits exact matches
      options: [{ allowedMediaQueries: ['@media print'] }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          root: {
            display: {
              default: 'flex',
              '@media (min-width: 600px)': 'none',
            },
          },
        });
      `,
      errors: [{ message: errorMessage }],
    },
  ],
});
