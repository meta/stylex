/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import js from '@eslint/js';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import stylexPlugin from '@stylexjs/eslint-plugin';
import svelte from 'eslint-plugin-svelte';
import tseslint from 'typescript-eslint';

const stylexRules = {
  '@stylexjs/valid-styles': 'error',
  '@stylexjs/no-unused': 'error',
  '@stylexjs/no-legacy-contextual-styles': 'error',
  '@stylexjs/sort-keys': ['error', { order: 'recess' }],
};

export default defineConfig([
  {
    ignores: ['.svelte-kit/**', 'build/**', 'node_modules/**'],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    files: ['**/*.{js,ts,svelte}'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      parserOptions: {
        parser: tseslint.parser,
      },
    },
    plugins: {
      '@stylexjs': stylexPlugin,
    },
    rules: stylexRules,
  },
]);
