/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const stylexBabelPlugin = require('@stylexjs/babel-plugin');
const createBuilder = require('../src/builder');

const stylexImport = 'import * as stylex from "@stylexjs/stylex";\n';
const source = (color) =>
  `${stylexImport}export const styles = stylex.create({root: {color: "${color}"}});\n`;

describe('PostCSS builder file results', () => {
  let cwd;
  let file;
  let time;

  beforeEach(() => {
    cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-postcss-rebuild-'));
    file = path.join(cwd, 'component.js');
    time = Date.now();
  });

  afterEach(() => {
    jest.restoreAllMocks();
    fs.rmSync(cwd, { recursive: true, force: true });
  });

  function put(contents) {
    fs.writeFileSync(file, contents);
    time += 1000;
    fs.utimesSync(file, time / 1000, time / 1000);
  }

  function setup({ isDev = true, ignored = false } = {}) {
    let failNext = false;
    let attempts = 0;
    const builder = createBuilder();
    const config = {
      cwd,
      include: ['*.js'],
      exclude: [],
      useCSSLayers: false,
      importSources: ['@stylexjs/stylex'],
      isDev,
      babelConfig: {
        babelrc: false,
        configFile: false,
        plugins: [
          () => ({
            visitor: {
              Program() {
                attempts++;
                if (failNext) {
                  failNext = false;
                  throw new Error('transient transform failure');
                }
              },
            },
          }),
          [stylexBabelPlugin, { dev: false, runtimeInjection: false }],
        ],
        ...(ignored ? { ignore: [file] } : {}),
      },
    };
    builder.configure(config);
    return {
      config,
      build: (shouldSkipTransformError = false) =>
        builder.build({ shouldSkipTransformError }),
      failOnce: () => {
        failNext = true;
      },
      attempts: () => attempts,
    };
  }

  test.each([
    ['last style', stylexImport + 'export const styles = {};\n'],
    ['StyleX import', 'export const styles = {};\n'],
  ])('removes prior CSS when the %s is removed', async (_name, replacement) => {
    const { build } = setup();
    put(source('red'));
    expect(await build()).toContain('color:red');
    put(replacement);
    expect(await build()).toBe('');
  });

  test.each([false, true])(
    'Babel-ignored files contribute no CSS (isDev: %s)',
    async (isDev) => {
      const { build, attempts } = setup({ isDev, ignored: true });
      put(source('red'));
      expect(await build()).toBe('');
      expect(attempts()).toBe(0);
    },
  );

  test('clears old rules when a changed file becomes Babel-ignored', async () => {
    const { config, build } = setup();
    put(source('red'));
    expect(await build()).toContain('color:red');
    config.babelConfig.ignore = [file];
    put(source('blue'));
    expect(await build()).toBe('');
  });

  test('preserves successful rules on a skipped failure and retries unchanged source', async () => {
    const { build, failOnce, attempts } = setup();
    put(source('red'));
    const before = await build();
    expect(before).toContain('color:red');
    put(source('blue'));
    failOnce();
    const warn = jest.spyOn(console, 'warn').mockImplementation(() => {});
    expect(await build(true)).toBe(before);
    expect(warn).toHaveBeenCalledTimes(1);
    expect(attempts()).toBe(2);
    const after = await build(true);
    expect(after).toContain('color:blue');
    expect(after).not.toContain('color:red');
    expect(attempts()).toBe(3);
    expect(warn).toHaveBeenCalledTimes(1);
  });

  test('propagates non-skipped failures and retries on the next build', async () => {
    const { build, failOnce, attempts } = setup();
    put(source('red'));
    await build();
    put(source('blue'));
    failOnce();
    await expect(build(false)).rejects.toThrow('transient transform failure');
    expect(await build(false)).toContain('color:blue');
    expect(attempts()).toBe(3);
  });

  test('still caches successful files and removes deleted files', async () => {
    const { build, attempts } = setup();
    put(source('red'));
    const before = await build();
    expect(before).toContain('color:red');
    expect(await build()).toBe(before);
    expect(attempts()).toBe(1);
    fs.unlinkSync(file);
    expect(await build()).toBe('');
  });
});
