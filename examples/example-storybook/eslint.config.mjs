/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */
import stylexPlugin from '@stylexjs/eslint-plugin';
import tseslint, { parser as typescriptParser } from 'typescript-eslint';
import storybookPlugin from 'eslint-plugin-storybook';
import importPlugin from 'eslint-plugin-import';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const typescriptPlugin = tseslint.config({
  files: ['**/*.ts', '**/*.tsx'],
  extends: [...tseslint.configs.recommended],
  rules: {
    '@typescript-eslint/no-unused-vars': [
      'error',
      {
        caughtErrors: 'none',
        argsIgnorePattern: '^_',
        destructuredArrayIgnorePattern: '^_',
        varsIgnorePattern: '^_.',
      },
    ],
    '@typescript-eslint/object-curly-spacing': ['warn'],
    '@typescript-eslint/consistent-type-imports': ['error'],
    'no-undef': 'off',
  },
});

export default [
  {
    ignores: ['postcss.config.*', 'storybook-static/'],
  },
  {
    plugins: {
      '@stylexjs': stylexPlugin,
      ...typescriptPlugin,
      ...storybookPlugin.configs['flat/recommended'],
      import: importPlugin,
    },

    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 12,
        sourceType: 'module',
        tsconfigRootDir: __dirname,
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    settings: {
      'import/ignore': ['node_modules'],
      react: {
        version: 'detect',
      },
    },
    rules: {
      '@stylexjs/valid-styles': 'error',
      '@stylexjs/no-unused': 'error',
      '@stylexjs/no-legacy-contextual-styles': 'error',
      '@stylexjs/sort-keys': ['error', { order: 'recess' }],
    },
  },
];
