/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('path');
const postcss = require('postcss');
const createPlugin = require('../src/plugin');

describe('@stylexjs/postcss-plugin', () => {
  const fixturesDir = path.resolve(__dirname, '__fixtures__');
  const autoDiscoveryFixturesDir = path.resolve(
    __dirname,
    '__auto_discovery_fixtures__',
  );

  function createAutoDiscoveryFixture() {
    const tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'stylex-postcss-auto-discovery-'),
    );

    fs.cpSync(autoDiscoveryFixturesDir, tempDir, { recursive: true });

    const stylexBabelPluginDir = path.join(
      tempDir,
      'node_modules',
      '@stylexjs',
      'babel-plugin',
    );
    fs.mkdirSync(stylexBabelPluginDir, { recursive: true });
    fs.writeFileSync(
      path.join(stylexBabelPluginDir, 'package.json'),
      JSON.stringify(
        {
          name: '@stylexjs/babel-plugin',
          version: '0.0.0-test',
          main: 'index.js',
        },
        null,
        2,
      ),
      'utf8',
    );
    fs.writeFileSync(
      path.join(stylexBabelPluginDir, 'index.js'),
      `module.exports = require(${JSON.stringify(
        path.resolve(__dirname, '..', '..', 'babel-plugin', 'lib', 'index.js'),
      )});\n`,
      'utf8',
    );

    const stylexLibDir = path.join(
      tempDir,
      'node_modules',
      'stylex-custom-lib',
    );
    fs.mkdirSync(stylexLibDir, { recursive: true });
    fs.writeFileSync(
      path.join(stylexLibDir, 'package.json'),
      JSON.stringify(
        {
          name: 'stylex-custom-lib',
          version: '1.0.0',
          main: 'index.js',
          dependencies: {
            'react-strict-dom': '^0.0.0',
          },
        },
        null,
        2,
      ),
      'utf8',
    );
    fs.writeFileSync(
      path.join(stylexLibDir, 'index.js'),
      [
        "import { css } from 'react-strict-dom';",
        '',
        'export const styles = css.create({',
        '  lib: {',
        "    backgroundColor: 'orange',",
        '  },',
        '});',
        '',
      ].join('\n'),
      'utf8',
    );

    const nonStylexLibDir = path.join(
      tempDir,
      'node_modules',
      'non-stylex-lib',
    );
    fs.mkdirSync(nonStylexLibDir, { recursive: true });
    fs.writeFileSync(
      path.join(nonStylexLibDir, 'package.json'),
      JSON.stringify(
        {
          name: 'non-stylex-lib',
          version: '1.0.0',
          main: 'index.js',
        },
        null,
        2,
      ),
      'utf8',
    );
    fs.writeFileSync(
      path.join(nonStylexLibDir, 'index.js'),
      'export const v = 1;\n',
    );

    const cachedNodeModulesDir = path.join(tempDir, 'node_modules', '.cache');
    fs.mkdirSync(cachedNodeModulesDir, { recursive: true });
    fs.writeFileSync(
      path.join(cachedNodeModulesDir, 'hidden-stylex.js'),
      [
        "import { css } from 'react-strict-dom';",
        '',
        'export const styles = css.create({',
        '  hidden: {',
        "    backgroundColor: 'hotpink',",
        '  },',
        '});',
        '',
      ].join('\n'),
      'utf8',
    );

    return tempDir;
  }

  async function runStylexPostcss(options = {}, inputCSS = '@stylex;') {
    // Create a new instance for each test as the plugin is stateful
    const stylexPostcssPlugin = createPlugin();

    const plugin = stylexPostcssPlugin({
      cwd: fixturesDir,
      include: ['**/*.js'],
      babelConfig: {
        configFile: path.join(fixturesDir, '.babelrc.js'),
      },
      ...options,
    });

    const processor = postcss([plugin]);
    const result = await processor.process(inputCSS, {
      from: path.join(fixturesDir, 'input.css'),
    });

    return result;
  }

  async function runAutoDiscoveryPostcss(options = {}, inputCSS = '@stylex;') {
    const fixtureDir = createAutoDiscoveryFixture();
    const stylexPostcssPlugin = createPlugin();
    const resolvedOptions =
      typeof options === 'function' ? options(fixtureDir) : options;
    const { useResolvedBabelConfig = false, ...pluginOptions } =
      resolvedOptions;

    const basePluginOptions = {
      cwd: fixtureDir,
    };

    if (!useResolvedBabelConfig) {
      const babelConfig = require(path.join(fixtureDir, 'babel.config.js'));
      basePluginOptions.babelConfig = {
        babelrc: false,
        plugins: babelConfig.plugins,
      };
    }

    const plugin = stylexPostcssPlugin({
      ...basePluginOptions,
      ...pluginOptions,
    });

    const processor = postcss([plugin]);
    try {
      const result = await processor.process(inputCSS, {
        from: path.join(fixtureDir, 'input.css'),
      });
      return {
        css: result.css,
        messages: result.messages,
      };
    } finally {
      fs.rmSync(fixtureDir, { recursive: true, force: true });
    }
  }

  test('extracts CSS from StyleX files', async () => {
    const result = await runStylexPostcss();

    expect(result.css).toMatchInlineSnapshot(`
      ".x1u857p9{background-color:green}
      .xrkmrrc{background-color:red}"
    `);

    // Check that messages contain dependency information
    expect(result.messages.length).toBeGreaterThan(0);
    expect(result.messages.some((m) => m.type === 'dir-dependency')).toBe(true);
  });

  test('handles empty CSS input without @stylex rule', async () => {
    const result = await runStylexPostcss({}, '/* No stylex rule here */');

    expect(result.css).toMatchInlineSnapshot('"/* No stylex rule here */"');
    expect(result.messages.length).toBe(0);
  });

  test('supports CSS layers', async () => {
    const result = await runStylexPostcss({ useCSSLayers: true });

    expect(result.css).toContain('@layer');
    expect(result.css).toMatchInlineSnapshot(`
      "
      @layer priority1;
      @layer priority1{
      .x1u857p9{background-color:green}
      .xrkmrrc{background-color:red}
      }"
    `);
  });

  test('handles exclude patterns', async () => {
    const result = await runStylexPostcss({
      exclude: ['**/styles-second.js'],
    });

    // Should not contain styles-second.js styles
    expect(result.css).not.toContain('green');

    expect(result.css).toMatchInlineSnapshot(
      '".xrkmrrc{background-color:red}"',
    );
  });

  test('respects string syntax for importSources', async () => {
    // Default importSources should not process any files
    const defaultResult = await runStylexPostcss({
      include: ['**/import-sources-*.js'],
    });

    expect(defaultResult.css).toBe('');

    // Custom importSources should process only import-sources-string.js
    const customResult = await runStylexPostcss({
      include: ['**/import-sources-*.js'],
      importSources: ['custom'],
      babelConfig: {
        babelrc: false,
        plugins: [
          [
            '@stylexjs/babel-plugin',
            {
              dev: false,
              runtimeInjection: false,
              importSources: ['custom'],
            },
          ],
        ],
      },
    });

    expect(customResult.css).toMatchInlineSnapshot(
      '".x1t391ir{background-color:blue}"',
    );
  });

  test('supports object syntax for importSources', async () => {
    const result = await runStylexPostcss({
      include: ['**/import-sources-object.js'],
      importSources: [{ as: 'css', from: 'react-strict-dom' }],
      babelConfig: {
        babelrc: false,
        plugins: [
          [
            '@stylexjs/babel-plugin',
            {
              dev: false,
              runtimeInjection: false,
              importSources: [{ as: 'css', from: 'react-strict-dom' }],
            },
          ],
        ],
      },
    });

    expect(result.css).toMatchInlineSnapshot(
      '".x1cu41gw{background-color:yellow}"',
    );
  });

  test('skips files that do not match include/exclude patterns', async () => {
    const result = await runStylexPostcss({
      include: ['**/styles-second.js'],
    });

    // Should contain styles-second.js styles but not styles.js
    expect(result.css).not.toContain('red');

    expect(result.css).toMatchInlineSnapshot(
      '".x1u857p9{background-color:green}"',
    );
  });

  test('dedupes files matched by relative and absolute include patterns', async () => {
    const targetFile = path.join(fixturesDir, 'styles.js');
    const readFileSpy = jest.spyOn(fs, 'readFileSync');

    try {
      const result = await runStylexPostcss({
        include: ['styles.js', targetFile],
        babelConfig: {
          babelrc: false,
          plugins: [
            [
              '@stylexjs/babel-plugin',
              {
                dev: false,
                runtimeInjection: false,
              },
            ],
          ],
        },
      });

      expect(result.css).toContain('background-color:red');

      const readsForTargetFile = readFileSpy.mock.calls.filter(([file]) => {
        return path.normalize(String(file)) === path.normalize(targetFile);
      });

      expect(readsForTargetFile).toHaveLength(1);
    } finally {
      readFileSpy.mockRestore();
    }
  });

  test('auto-discovers include globs when include is omitted', async () => {
    const result = await runStylexPostcss({
      include: undefined,
    });

    expect(result.css).toContain('background-color:green');
    expect(result.css).toContain('background-color:red');
  });

  test('auto-discovers StyleX dependency directories and importSources from Babel config', async () => {
    const result = await runAutoDiscoveryPostcss();

    expect(result.css).toContain('color:red');
    expect(result.css).toContain('color:purple');
    expect(result.css).toContain('background-color:orange');

    expect(
      result.messages.some(
        (message) =>
          message.type === 'dir-dependency' &&
          message.dir.includes('stylex-custom-lib'),
      ),
    ).toBe(true);
  });

  test('does not auto-discover dependency directories when include is explicitly provided', async () => {
    const result = await runAutoDiscoveryPostcss({
      include: ['src/local-stylex.js'],
    });

    expect(result.css).toContain('color:red');
    expect(result.css).not.toContain('background-color:orange');
    expect(
      result.messages.some(
        (message) =>
          message.type === 'dir-dependency' &&
          message.dir.includes('stylex-custom-lib'),
      ),
    ).toBe(false);
  });

  test('infers importSources from resolved babel config when plugins are not passed inline', async () => {
    const previousCwd = process.cwd();
    process.chdir(os.tmpdir());

    try {
      const result = await runAutoDiscoveryPostcss({
        useResolvedBabelConfig: true,
      });

      expect(result.css).toContain('color:red');
      expect(result.css).toContain('color:purple');
      expect(result.css).toContain('background-color:orange');
    } finally {
      process.chdir(previousCwd);
    }
  });

  test('prefers explicit importSources over inferred babel importSources', async () => {
    const result = await runAutoDiscoveryPostcss({
      useResolvedBabelConfig: true,
      importSources: ['@stylexjs/stylex'],
    });

    expect(result.css).toContain('color:red');
    expect(result.css).not.toContain('color:purple');
    expect(result.css).not.toContain('background-color:orange');
  });

  test('infers importSources when StyleX Babel plugin is passed as a function reference', async () => {
    const stylexBabelPlugin = require(
      path.resolve(__dirname, '..', '..', 'babel-plugin', 'lib', 'index.js'),
    );

    const result = await runStylexPostcss({
      include: ['**/import-sources-string.js'],
      babelConfig: {
        babelrc: false,
        plugins: [
          [
            stylexBabelPlugin,
            {
              dev: false,
              runtimeInjection: false,
              importSources: ['custom'],
            },
          ],
        ],
      },
    });

    expect(result.css).toContain('background-color:blue');
  });

  test('falls back to default importSources when babel plugin options omit importSources', async () => {
    const stylexBabelPlugin = require(
      path.resolve(__dirname, '..', '..', 'babel-plugin', 'lib', 'index.js'),
    );

    const result = await runStylexPostcss({
      include: ['**/styles.js', '**/import-sources-string.js'],
      babelConfig: {
        babelrc: false,
        plugins: [
          [
            stylexBabelPlugin,
            {
              dev: false,
              runtimeInjection: false,
            },
          ],
        ],
      },
    });

    expect(result.css).toContain('background-color:red');
    expect(result.css).not.toContain('background-color:blue');
  });

  test('processes absolute include paths in node_modules even when node_modules is excluded', async () => {
    const result = await runAutoDiscoveryPostcss((fixtureDir) => ({
      include: [
        path.join(fixtureDir, 'node_modules/stylex-custom-lib/index.js'),
      ],
      exclude: ['**/node_modules/**'],
      babelConfig: {
        babelrc: false,
        plugins: [
          [
            '@stylexjs/babel-plugin',
            {
              dev: false,
              runtimeInjection: false,
              importSources: [{ from: 'react-strict-dom', as: 'css' }],
            },
          ],
        ],
      },
    }));

    expect(result.css).toContain('background-color:orange');
  });

  test('keeps specific node_modules excludes when absolute includes point to node_modules', async () => {
    const result = await runAutoDiscoveryPostcss((fixtureDir) => ({
      include: [path.join(fixtureDir, 'node_modules/**/*.js')],
      exclude: ['**/node_modules/**', '**/node_modules/.cache/**'],
      babelConfig: {
        babelrc: false,
        plugins: [
          [
            '@stylexjs/babel-plugin',
            {
              dev: false,
              runtimeInjection: false,
              importSources: [{ from: 'react-strict-dom', as: 'css' }],
            },
          ],
        ],
      },
    }));

    expect(result.css).toContain('background-color:orange');
    expect(result.css).not.toContain('background-color:hotpink');
  });

  test('logs discovery details in debug mode', async () => {
    const previousDebugValue = process.env.STYLEX_POSTCSS_DEBUG;
    process.env.STYLEX_POSTCSS_DEBUG = '1';
    const infoSpy = jest.spyOn(console, 'info').mockImplementation(() => {});

    try {
      await runAutoDiscoveryPostcss();

      const log = infoSpy.mock.calls.find(([arg]) =>
        String(arg).includes('Auto-discovery details'),
      );

      expect(log).toBeDefined();
      expect(String(log?.[0])).toContain('"importSourcesSource"');
      expect(String(log?.[0])).toContain('"include"');
    } finally {
      if (previousDebugValue == null) {
        delete process.env.STYLEX_POSTCSS_DEBUG;
      } else {
        process.env.STYLEX_POSTCSS_DEBUG = previousDebugValue;
      }
      infoSpy.mockRestore();
    }
  });

  test('concurrent builds do not skip files mid transform', async () => {
    const stylesPath = path.join(fixturesDir, 'concurrent-styles.js');
    const stylexPostcssPlugin = createPlugin();
    const plugin = stylexPostcssPlugin({
      cwd: fixturesDir,
      include: ['concurrent-styles.js'],
      babelConfig: {
        configFile: path.join(fixturesDir, '.babelrc.js'),
      },
    });
    const processor = postcss([plugin]);
    const input = {
      from: path.join(fixturesDir, 'input.css'),
    };

    fs.writeFileSync(
      stylesPath,
      [
        "import * as stylex from '@stylexjs/stylex';",
        '',
        'export const styles = stylex.create({',
        '  container: {',
        "    backgroundColor: 'red',",
        '  },',
        '});',
        '',
      ].join('\n'),
      'utf8',
    );

    try {
      await processor.process('@stylex;', input);

      fs.writeFileSync(
        stylesPath,
        [
          "import * as stylex from '@stylexjs/stylex';",
          '',
          'export const styles = stylex.create({',
          '  container: {',
          "    backgroundColor: 'blue',",
          '  },',
          '});',
          '',
        ].join('\n'),
        'utf8',
      );
      const nextMtime = fs.statSync(stylesPath).mtimeMs + 1000;
      fs.utimesSync(stylesPath, new Date(nextMtime), new Date(nextMtime));

      const [first, second] = await Promise.all([
        processor.process('@stylex;', input),
        processor.process('@stylex;', input),
      ]);

      expect(first.css).toContain('background-color:blue');
      expect(second.css).toContain('background-color:blue');
      expect(first.css).not.toContain('background-color:red');
      expect(second.css).not.toContain('background-color:red');
    } finally {
      fs.rmSync(stylesPath, { force: true });
    }
  });
});
