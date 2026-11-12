/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import js from '@eslint/js';
import globals from 'globals';
import stylexPlugin from '@stylexjs/eslint-plugin';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';
import { defineConfig, globalIgnores } from 'eslint/config';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const stylexRules = {
  '@stylexjs/valid-styles': 'error',
  '@stylexjs/no-unused': 'error',
  '@stylexjs/no-legacy-contextual-styles': 'error',
  '@stylexjs/sort-keys': ['error', { order: 'recess' }],
};

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs['recommended-latest'],
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
      parserOptions: {
        tsconfigRootDir: __dirname,
      },
    },
    plugins: {
      '@stylexjs': stylexPlugin,
    },
    rules: stylexRules,
  },
]);
