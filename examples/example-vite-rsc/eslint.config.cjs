/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
const stylexPlugin = require('@stylexjs/eslint-plugin');
const tsParser = require('@typescript-eslint/parser');

const stylexRules = {
  '@stylexjs/valid-styles': 'error',
  '@stylexjs/no-unused': 'error',
  '@stylexjs/no-legacy-contextual-styles': 'error',
  '@stylexjs/sort-keys': ['error', { order: 'recess' }],
};

module.exports = [
  {
    ignores: ['dist/**', 'node_modules/**'],
  },
  {
    files: ['**/*.{js,jsx,mjs,cjs}'],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    plugins: {
      '@stylexjs': stylexPlugin,
    },
    rules: stylexRules,
  },
  {
    files: ['**/*.{ts,tsx,mts,cts}'],
    languageOptions: {
      ecmaVersion: 'latest',
      parser: tsParser,
      sourceType: 'module',
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    plugins: {
      '@stylexjs': stylexPlugin,
    },
    rules: stylexRules,
  },
];
