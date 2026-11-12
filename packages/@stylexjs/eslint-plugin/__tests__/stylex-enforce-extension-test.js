/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { RuleTester } = require('eslint');
const rule = require('../src/stylex-enforce-extension');

const ruleTester = new RuleTester({
  parserOptions: { ecmaVersion: 2020, sourceType: 'module' },
});

const invalidFilenameWithRestrictedExports = (suggestedExtension) =>
  `Files that export variables from \`stylex.defineVars()\`, \`stylex.defineConsts()\`, or \`stylex.defineMarker()\` must end with a \`${suggestedExtension}\` extension.`;
const invalidFilenameWithoutRestrictedExports = (suggestedExtension) =>
  `Only variables from \`stylex.defineVars()\`, \`stylex.defineConsts()\`, or \`stylex.defineMarker()\` can be exported from a file with a \`${suggestedExtension}\` extension.`;
const invalidExportFromThemeFiles =
  'Files that export variables from `stylex.defineVars()`, `stylex.defineConsts()`, or `stylex.defineMarker()` must not export anything else.';
const invalidConstsFilenameWithRestrictedExports = (suggestedExtension) =>
  `Files that export variables from \`stylex.defineConsts()\` must end with a \`${suggestedExtension}\` extension.`;
const invalidConstsFilenameWithoutRestrictedExports = (suggestedExtension) =>
  `Only variables from \`stylex.defineConsts()\` can be exported from a file with a \`${suggestedExtension}\` extension.`;
const invalidExportFromConstsFiles =
  'Files that export variables from `stylex.defineConsts()` must not export anything else.';
const invalidDefaultExport = (type) =>
  `Default exports are not allowed for variables from \`stylex.${type}()\`. Use named exports instead.`;

ruleTester.run('stylex-enforce-extension', rule.default, {
  valid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.tsx',
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.jsx',
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.tsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.custom.jsx',
      options: [{ themeFileExtension: '.custom' }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.custom.jsx',
      options: [{ themeFileExtension: '.custom.jsx' }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.custom.jsx',
      options: [{ themeFileExtension: '.custom.cjs' }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.custom.tsx',
      options: [{ themeFileExtension: '.custom' }],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.jsx',
      options: [{ themeFileExtension: '.custom' }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({ color: 'red' });
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.jsx',
    },
    {
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        export const vars = css.defineVars({});
        export const consts = css.defineConsts({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.js',
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.js',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.ts',
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.ts',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.cjs',
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.cjs',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.stylex.mjs',
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.mjs',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.custom.jsx',
      options: [{ themeFileExtension: '.custom' }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      options: [{ legacyAllowMixedExports: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      options: [{ legacyAllowMixedExports: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.tsx',
      options: [{ legacyAllowMixedExports: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.jsx',
      options: [{ enforceDefineConstsExtension: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.js',
      options: [{ enforceDefineConstsExtension: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.tsx',
      options: [{ enforceDefineConstsExtension: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.ts',
      options: [{ enforceDefineConstsExtension: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.cjs',
      options: [{ enforceDefineConstsExtension: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.mjs',
      options: [{ enforceDefineConstsExtension: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.tsx',
      options: [{ legacyAllowMixedExports: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.ts',
      options: [{ legacyAllowMixedExports: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.js',
      options: [{ legacyAllowMixedExports: true }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.custom.ts',
      options: [
        { legacyAllowMixedExports: true, themeFileExtension: '.custom' },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.custom.js',
      options: [
        { legacyAllowMixedExports: true, themeFileExtension: '.custom' },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const marker = stylex.defineMarker();
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
        export const consts = stylex.defineConsts({});
        export const marker = stylex.defineMarker();
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.unstable_defineVarsNested({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.unstable_defineConstsNested({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import { unstable_defineVarsNested } from '@stylexjs/stylex';
        export const vars = unstable_defineVarsNested({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import { unstable_defineConstsNested } from '@stylexjs/stylex';
        export const consts = unstable_defineConstsNested({});
      `,
      filename: 'testComponent.stylex.jsx',
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.unstable_defineConstsNested({});
      `,
      filename: 'myComponent.stylex.const.jsx',
      options: [{ enforceDefineConstsExtension: true }],
    },
  ],

  invalid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = someFunction();
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [
        { message: invalidFilenameWithoutRestrictedExports('.stylex.jsx') },
        { message: invalidFilenameWithoutRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'blue',
        });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.tsx',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red',
        });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'blue',
        });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.js',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({
          color: 'red',
        });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.cjs',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
      `,
      filename: 'testComponent.tsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.tsx') },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.jsx',
      errors: [
        { message: invalidFilenameWithoutRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.js',
      errors: [
        { message: invalidFilenameWithoutRestrictedExports('.stylex.js') },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.ts',
      errors: [
        { message: invalidFilenameWithoutRestrictedExports('.stylex.ts') },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.cjs',
      errors: [
        { message: invalidFilenameWithoutRestrictedExports('.stylex.cjs') },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.mjs',
      errors: [
        { message: invalidFilenameWithoutRestrictedExports('.stylex.mjs') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
      `,
      filename: 'testComponent.jsx',
      options: [{ themeFileExtension: '.custom' }],
      errors: [
        {
          message: invalidFilenameWithRestrictedExports('.custom.jsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
      `,
      filename: 'testComponent.jsx',
      options: [{ themeFileExtension: '.custom.jsx' }],
      errors: [
        {
          message: invalidFilenameWithRestrictedExports('.custom.jsx'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'testComponent.custom.jsx',
      options: [{ themeFileExtension: '.custom' }],
      errors: [
        {
          message: invalidFilenameWithoutRestrictedExports('.custom.jsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({});
      `,
      filename: 'testComponent.tsx',
      options: [{ themeFileExtension: '.custom' }],
      errors: [
        {
          message: invalidFilenameWithRestrictedExports('.custom.tsx'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'test.custom.ts',
      options: [{ themeFileExtension: '.custom' }],
      errors: [
        {
          message: invalidFilenameWithoutRestrictedExports('.custom.ts'),
        },
      ],
    },
    {
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        export const vars = stylex.defineVars({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        {
          message: invalidFilenameWithRestrictedExports('.stylex.jsx'),
        },
      ],
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        export const vars = css.defineVars({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        {
          message: invalidFilenameWithRestrictedExports('.stylex.jsx'),
        },
      ],
    },
    {
      options: [{ validImports: ['custom-stylex'] }],
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.jsx',
      errors: [
        {
          message: invalidFilenameWithoutRestrictedExports('.stylex.jsx'),
        },
      ],
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: 'export const somethingElse = {};',
      filename: 'testComponent.stylex.jsx',
      errors: [
        {
          message: invalidFilenameWithoutRestrictedExports('.stylex.jsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.defineVars({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      options: [{ legacyAllowMixedExports: false }],
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      options: [{ legacyAllowMixedExports: false }],
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.jsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithRestrictedExports('.stylex.const.jsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.tsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithRestrictedExports('.stylex.const.tsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.js',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithRestrictedExports('.stylex.const.js'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.ts',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithRestrictedExports('.stylex.const.ts'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.jsx',
      options: [
        { enforceDefineConstsExtension: true, themeFileExtension: '.custom' },
      ],
      errors: [
        {
          message:
            invalidConstsFilenameWithRestrictedExports('.custom.const.jsx'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'myComponent.stylex.const.jsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.stylex.const.jsx'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'myComponent.stylex.const.tsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.stylex.const.tsx'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'myComponent.stylex.const.js',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.stylex.const.js'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'myComponent.stylex.const.ts',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.stylex.const.ts'),
        },
      ],
    },
    {
      code: 'export const vars = stylex.defineVars({});',
      filename: 'myComponent.stylex.const.cjs',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.stylex.const.cjs'),
        },
      ],
    },
    {
      code: 'export const somethingElse = {};',
      filename: 'myComponent.stylex.const.mjs',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.stylex.const.mjs'),
        },
      ],
    },
    {
      code: 'export const vars = stylex.defineVars({});',
      filename: 'myComponent.custom.const.jsx',
      options: [
        { enforceDefineConstsExtension: true, themeFileExtension: '.custom' },
      ],
      errors: [
        {
          message:
            invalidConstsFilenameWithoutRestrictedExports('.custom.const.jsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const consts_2 = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.const.jsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.const.tsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = stylex.defineVars({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.js',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message: invalidExportFromConstsFiles,
        },
        {
          message:
            'Files that export variables from `stylex.defineVars()` or `stylex.defineMarker()` must end with a `.stylex.js` extension.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.const.ts',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.const.cjs',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.const.mjs',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.defineConsts({ color: 'red' });
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.custom.const.jsx',
      options: [
        { enforceDefineConstsExtension: true, themeFileExtension: '.custom' },
      ],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export default stylex.defineVars({ color: 'red' });
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineVars') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export default stylex.defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineConsts') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const vars = stylex.defineVars({ color: 'red' });
        export default vars;
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineVars') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const consts = stylex.defineConsts({ color: 'red' });
        export default consts;
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineConsts') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export default stylex.defineVars({ color: 'red' });
      `,
      filename: 'myComponent.stylex.const.jsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidDefaultExport('defineVars') }],
    },
    {
      code: `
        import { defineVars } from '@stylexjs/stylex';
        export default defineVars({ color: 'red' });
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineVars') }],
    },
    {
      code: `
        import { defineConsts } from '@stylexjs/stylex';
        export default defineConsts({ color: 'red' });
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineConsts') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const marker = stylex.defineMarker();
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const marker = stylex.defineMarker();
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export default stylex.defineMarker();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineMarker') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const marker = stylex.defineMarker();
        export default marker;
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidDefaultExport('defineMarker') }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.unstable_defineVarsNested({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.unstable_defineConstsNested({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import { unstable_defineVarsNested } from '@stylexjs/stylex';
        export const vars = unstable_defineVarsNested({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import { unstable_defineConstsNested } from '@stylexjs/stylex';
        export const consts = unstable_defineConstsNested({});
      `,
      filename: 'testComponent.jsx',
      errors: [
        { message: invalidFilenameWithRestrictedExports('.stylex.jsx') },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const vars = stylex.unstable_defineVarsNested({});
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.jsx',
      errors: [{ message: invalidExportFromThemeFiles }],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.unstable_defineConstsNested({});
      `,
      filename: 'myComponent.jsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [
        {
          message:
            invalidConstsFilenameWithRestrictedExports('.stylex.const.jsx'),
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        export const consts = stylex.unstable_defineConstsNested({});
        export const somethingElse = someFunction();
      `,
      filename: 'myComponent.stylex.const.jsx',
      options: [{ enforceDefineConstsExtension: true }],
      errors: [{ message: invalidExportFromConstsFiles }],
    },
  ],
});

const typeExportRuleTester = new RuleTester({
  parser: require.resolve('@typescript-eslint/parser'),
  parserOptions: { sourceType: 'module' },
});

typeExportRuleTester.run(
  'stylex-enforce-extension (type exports)',
  rule.default,
  {
    valid: [
      {
        code: `
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({
          primary: 'blue',
          secondary: 'green',
          accent: 'coral',
        });
        export type Colors = keyof typeof colors;
      `,
        filename: 'colors.stylex.ts',
      },
      {
        code: `
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineConsts({
          primary: 'blue',
          secondary: 'green',
        });
        export type Colors = keyof typeof colors;
      `,
        filename: 'colors.stylex.const.ts',
        options: [{ enforceDefineConstsExtension: true }],
      },
      {
        code: `
        import * as stylex from '@stylexjs/stylex';
        type Colors = keyof typeof colors;
        export const colors = stylex.defineVars({
          primary: 'blue',
          secondary: 'green',
        });
        export type { Colors };
      `,
        filename: 'colors.stylex.ts',
      },
      {
        code: `
        import * as stylex from '@stylexjs/stylex';
        type Colors = keyof typeof colors;
        export const colors = stylex.defineVars({
          primary: 'blue',
          secondary: 'green',
        });
        export { type Colors };
      `,
        filename: 'colors.stylex.ts',
      },
    ],

    invalid: [
      {
        code: `
        import * as stylex from '@stylexjs/stylex';
        export const colors = stylex.defineVars({ primary: 'blue' });
        export type Colors = keyof typeof colors;
        export const helper = someFunction();
      `,
        filename: 'colors.stylex.ts',
        errors: [{ message: invalidExportFromThemeFiles }],
      },
    ],
  },
);
