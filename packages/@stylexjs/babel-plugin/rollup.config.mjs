/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import { nodeResolve } from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';
import { babel } from '@rollup/plugin-babel';
import path from 'path';
import babelTypes from '@babel/types';

const extensions = ['.js', '.jsx', '.cjs', '.mjs'];

// get ESM __dirname
const __dirname = new URL('.', import.meta.url).pathname;

const rootDir = path.resolve(__dirname, '../..');

const babelTypesExports = Object.keys(babelTypes).filter((name) =>
  babelTypes.isValidIdentifier(name),
);

const external = [
  '@babel/traverse',
  '@babel/types',
  '@babel/core',
  '@babel/helper-module-imports',
  'node:crypto',
  'node:fs',
  'node:module',
  'node:path',
  'node:url',
  'assert',
  'crypto',
  'fs',
  'module',
  'path',
  'postcss-value-parser',
  'url',
];

const config = {
  input: './src/index.js',
  output: {
    file: './lib/index.js',
    format: 'cjs',
  },
  external: process.env['HASTE']
    ? external
    : [...external, '@dual-bundle/import-meta-resolve', '@stylexjs/stylex'],
  plugins: [
    babel({ babelHelpers: 'bundled', extensions, include: ['./src/**/*'] }),
    nodeResolve({
      preferBuiltins: false,
      extensions,
      allowExportsFolderMapping: true,
      rootDir,
    }),
    commonjs(),
    json(),
  ],
};

const browserConfig = {
  input: './src/index.js',
  output: {
    file: './lib/index.browser.js',
    format: 'esm',
  },
  external: ['@babel/standalone', '@stylexjs/stylex', 'postcss-value-parser'],
  onwarn(warning, warn) {
    if (warning.code === 'MISSING_EXPORT') {
      throw new Error(warning.message);
    }
    warn(warning);
  },
  plugins: [
    {
      name: 'stub-modules',
      resolveId(source, importer) {
        switch (source) {
          case 'node:path':
            return this.resolve('path-browserify', importer);
          case 'node:fs':
          case 'node:url':
          case 'assert':
          case '@dual-bundle/import-meta-resolve':
          case '@babel/core':
          case '@babel/traverse':
          case '@babel/types':
            return source;
          default:
            return null;
        }
      },
      load(id) {
        switch (id) {
          case 'node:fs':
          case 'node:url':
            return 'export default {};';
          case 'assert':
            return 'export default function assert(condition, message) { if (!condition) throw new Error(message || "Assertion failed"); }';
          case '@dual-bundle/import-meta-resolve':
            return 'export function moduleResolve() {}';
          case '@babel/core':
            return "import { packages } from '@babel/standalone'; export const parseSync = packages.parser.parse;";
          case '@babel/traverse':
            return "import { packages } from '@babel/standalone'; export default packages.traverse.default;";
          case '@babel/types':
            return `import { packages } from '@babel/standalone';
export const { ${babelTypesExports.join(', ')} } = packages.types;`;
          default:
            return null;
        }
      },
    },
    babel({ babelHelpers: 'bundled', extensions, include: ['./src/**/*'] }),
    nodeResolve({
      preferBuiltins: false,
      extensions,
      allowExportsFolderMapping: true,
      rootDir,
      browser: true,
    }),
    commonjs(),
    json(),
  ],
};

export default [config, browserConfig];
