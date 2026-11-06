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

const { unplugin } = require('../src');
const {
  unpluginFactory,
  pickCssAssetFromRollupBundle,
} = require('../src/core');
const { Features, browserslistToTargets } = require('lightningcss');
const browserslist = require('browserslist');

describe('@stylexjs/unplugin', () => {
  test('ignores files without StyleX imports', async () => {
    const plugin = unplugin.raw({});
    if (typeof plugin.buildStart === 'function') {
      plugin.buildStart();
    }
    const result = await plugin.transform('const noop = 1;', '/virtual/foo.js');
    expect(result).toBeNull();
  });

  test('still transforms built-in imports when a custom importSource is set', async () => {
    const plugin = unplugin.raw({ importSources: ['custom-stylex'] });
    if (typeof plugin.buildStart === 'function') {
      plugin.buildStart();
    }
    const source = `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({ foo: { color: 'red' } });
      export default styles;
    `;
    const result = await plugin.transform(source, '/virtual/example.js');
    expect(result).not.toBeNull();
  });

  test('writes fallback CSS asset when no CSS bundle entry exists', async () => {
    const plugin = unplugin.rollup({
      runtimeInjection: false,
      devPersistToDisk: false,
      dev: false,
    });
    if (typeof plugin.buildStart === 'function') {
      plugin.buildStart();
    }
    const source = `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({ foo: { color: 'red' } });
      export default styles;
    `;
    const result = await plugin.transform(source, '/virtual/example.js');
    expect(result).not.toBeNull();

    const tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'stylex-unplugin-test-'),
    );
    try {
      await plugin.writeBundle({ dir: tempDir }, {});
      const cssPath = path.join(tempDir, 'assets', 'stylex.css');
      expect(fs.existsSync(cssPath)).toBe(true);
      const cssContent = fs.readFileSync(cssPath, 'utf8');
      expect(cssContent).toContain('color: red;');
      expect(cssContent.trim()).toMatch(
        /^\.[a-z0-9]+ \{\n {2}color: red;\n\}$/i,
      );
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  test('writeBundle is skipped when generateBundle already injected CSS (Vite 8)', async () => {
    const plugin = unplugin.vite({
      runtimeInjection: false,
      devPersistToDisk: false,
      dev: false,
    });
    if (typeof plugin.buildStart === 'function') {
      plugin.buildStart();
    }
    const source = `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({ foo: { color: 'red' } });
      export default styles;
    `;
    await plugin.transform(source, '/virtual/example.js');

    // Simulate Vite 8: CSS asset exists in bundle during generateBundle
    const bundle = {
      'assets/index-abc123.css': {
        type: 'asset',
        fileName: 'assets/index-abc123.css',
        source: '/* existing css */',
      },
    };
    const emittedFiles = {};
    const ctx = {
      emitFile(file) {
        const id = 'ref_1';
        emittedFiles[id] = file;
        return id;
      },
      getFileName(id) {
        return emittedFiles[id]?.name
          ? 'assets/' + emittedFiles[id].name
          : null;
      },
    };

    plugin.generateBundle.handler.call(ctx, {}, bundle);

    // generateBundle should have injected CSS into the bundle
    const cssAssets = Object.values(bundle).filter(
      (a) => a.type === 'asset' && a.fileName.endsWith('.css'),
    );
    expect(cssAssets.length).toBeGreaterThan(0);
    const injectedAsset = cssAssets.find((a) =>
      typeof a.source === 'string' ? a.source.includes('color') : false,
    );
    expect(injectedAsset).toBeTruthy();

    // Now simulate writeBundle — it should be a no-op
    const tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'stylex-unplugin-vite8-'),
    );
    try {
      const assetsDir = path.join(tempDir, 'assets');
      fs.mkdirSync(assetsDir, { recursive: true });
      // Write initial CSS file that writeBundle might try to append to
      fs.writeFileSync(
        path.join(assetsDir, 'index-abc123.css'),
        '/* existing css */',
        'utf8',
      );
      await plugin.writeBundle(
        { dir: tempDir },
        {
          'assets/index-abc123.css': {
            type: 'asset',
            fileName: 'assets/index-abc123.css',
            source: '/* existing css */',
          },
        },
      );
      // The file on disk should NOT have StyleX CSS appended
      const diskContent = fs.readFileSync(
        path.join(assetsDir, 'index-abc123.css'),
        'utf8',
      );
      expect(diskContent).toBe('/* existing css */');
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  test('writeBundle still works as fallback when no CSS asset in generateBundle', async () => {
    const plugin = unplugin.vite({
      runtimeInjection: false,
      devPersistToDisk: false,
      dev: false,
    });
    if (typeof plugin.buildStart === 'function') {
      plugin.buildStart();
    }
    const source = `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({ bar: { color: 'blue' } });
      export default styles;
    `;
    await plugin.transform(source, '/virtual/example.js');

    // generateBundle with empty bundle (no CSS assets) — like SSR
    const ctx = {
      emitFile() {
        return 'ref_1';
      },
      getFileName() {
        return null;
      },
    };
    plugin.generateBundle.handler.call(ctx, {}, {});

    // writeBundle should still create fallback CSS
    const tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'stylex-unplugin-fallback-'),
    );
    try {
      await plugin.writeBundle({ dir: tempDir }, {});
      const cssPath = path.join(tempDir, 'assets', 'stylex.css');
      expect(fs.existsSync(cssPath)).toBe(true);
      const cssContent = fs.readFileSync(cssPath, 'utf8');
      expect(cssContent).toContain('color:');
    } finally {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  test('runs generateBundle after Vite emits the CSS asset', () => {
    const plugin = unplugin.vite({ dev: false });
    expect(plugin.generateBundle.order).toBe('post');
  });

  test('marks StyleX deps as non-optimized in Vite', async () => {
    const tempDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'stylex-unplugin-vite-'),
    );
    const originalCwd = process.cwd();
    try {
      const pkgJson = {
        dependencies: {
          'lib-using-stylex': '1.0.0',
          'lib-no-stylex': '1.0.0',
        },
      };
      fs.writeFileSync(
        path.join(tempDir, 'package.json'),
        JSON.stringify(pkgJson),
        'utf8',
      );
      fs.mkdirSync(path.join(tempDir, 'node_modules', 'lib-using-stylex'), {
        recursive: true,
      });
      fs.writeFileSync(
        path.join(tempDir, 'node_modules', 'lib-using-stylex', 'package.json'),
        JSON.stringify({
          name: 'lib-using-stylex',
          version: '1.0.0',
          dependencies: { '@stylexjs/stylex': '^0.0.0' },
        }),
        'utf8',
      );
      fs.mkdirSync(path.join(tempDir, 'node_modules', 'lib-no-stylex'), {
        recursive: true,
      });
      fs.writeFileSync(
        path.join(tempDir, 'node_modules', 'lib-no-stylex', 'package.json'),
        JSON.stringify({
          name: 'lib-no-stylex',
          version: '1.0.0',
        }),
        'utf8',
      );
      process.chdir(tempDir);

      const nestedDir = path.join(tempDir, 'nested', 'deeper');
      fs.mkdirSync(nestedDir, { recursive: true });
      process.chdir(nestedDir);

      const plugin = unplugin.vite({});
      const viteConfigHook = plugin.config;
      const result =
        typeof viteConfigHook === 'function'
          ? await viteConfigHook.call(
              plugin,
              {},
              { command: 'serve', mode: 'development' },
            )
          : null;
      expect(result?.optimizeDeps?.exclude).toEqual(
        expect.arrayContaining(['lib-using-stylex']),
      );
      expect(result?.optimizeDeps?.exclude || []).not.toEqual(
        expect.arrayContaining(['lib-no-stylex']),
      );
      expect(result?.ssr?.optimizeDeps?.exclude).toEqual(
        expect.arrayContaining(['lib-using-stylex']),
      );

      const pluginWithManual = unplugin.vite({
        externalPackages: ['manual-stylex-lib'],
      });
      const manualResult =
        typeof pluginWithManual.config === 'function'
          ? await pluginWithManual.config(
              {},
              { command: 'serve', mode: 'development' },
            )
          : null;
      expect(manualResult?.optimizeDeps?.exclude).toEqual(
        expect.arrayContaining(['manual-stylex-lib', 'lib-using-stylex']),
      );
    } finally {
      process.chdir(originalCwd);
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('CSS asset selection', () => {
    // File names taken from a production `vite build`, which hashes assets
    // with the default `assetFileNames` of `assets/[name]-[hash][extname]`.
    const ENTRY_CSS = 'assets/index-W1erjkBN.css';
    const ROUTE_CSS = 'assets/admin-CyH6f07N.css';
    const asset = (fileName) => ({ type: 'asset', fileName, source: '' });

    test('prefers the hashed entry stylesheet over other stylesheets', () => {
      const bundle = {
        [ROUTE_CSS]: asset(ROUTE_CSS),
        [ENTRY_CSS]: asset(ENTRY_CSS),
      };
      expect(pickCssAssetFromRollupBundle(bundle).fileName).toBe(ENTRY_CSS);
    });

    // Rollup's default `hashCharacters` is base64url, so a hash can contain
    // `-` and `_`. Both names below came out of a real `vite build`.
    test('prefers an entry stylesheet whose hash contains an underscore', () => {
      const entryCss = 'assets/index-BVm_Qe95.css';
      const bundle = {
        [ROUTE_CSS]: asset(ROUTE_CSS),
        [entryCss]: asset(entryCss),
      };
      expect(pickCssAssetFromRollupBundle(bundle).fileName).toBe(entryCss);
    });

    test('prefers an entry stylesheet whose hash contains a hyphen', () => {
      const entryCss = 'assets/index-CnU-v52M.css';
      const bundle = {
        [ROUTE_CSS]: asset(ROUTE_CSS),
        [entryCss]: asset(entryCss),
      };
      expect(pickCssAssetFromRollupBundle(bundle).fileName).toBe(entryCss);
    });

    test('prefers a hashed style.css over other stylesheets', () => {
      const styleCss = 'assets/style-BpQ2m1Zx.css';
      const bundle = {
        [ROUTE_CSS]: asset(ROUTE_CSS),
        [styleCss]: asset(styleCss),
      };
      expect(pickCssAssetFromRollupBundle(bundle).fileName).toBe(styleCss);
    });

    test('still prefers unhashed index.css', () => {
      const bundle = {
        'assets/vendor.css': asset('assets/vendor.css'),
        'assets/index.css': asset('assets/index.css'),
      };
      expect(pickCssAssetFromRollupBundle(bundle).fileName).toBe(
        'assets/index.css',
      );
    });

    test('does not treat a longer name ending in index as the entry', () => {
      const resetCss = 'assets/reset-index-BpQ2m1Zx.css';
      const bundle = {
        [ROUTE_CSS]: asset(ROUTE_CSS),
        [resetCss]: asset(resetCss),
      };
      expect(pickCssAssetFromRollupBundle(bundle).fileName).toBe(ROUTE_CSS);
    });

    test('cssInjectionTarget still overrides the preferred names', () => {
      const bundle = {
        [ENTRY_CSS]: asset(ENTRY_CSS),
        [ROUTE_CSS]: asset(ROUTE_CSS),
      };
      const chosen = pickCssAssetFromRollupBundle(bundle, (f) =>
        f.includes('admin'),
      );
      expect(chosen.fileName).toBe(ROUTE_CSS);
    });

    test('writeBundle fallback appends to the hashed entry stylesheet', async () => {
      const plugin = unplugin.rollup({
        runtimeInjection: false,
        devPersistToDisk: false,
        dev: false,
      });
      if (typeof plugin.buildStart === 'function') {
        plugin.buildStart();
      }
      // A value that survives minification, so it is easy to locate on disk.
      const marker = '987654';
      const source = `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({ foo: { zIndex: ${marker} } });
        export default styles;
      `;
      await plugin.transform(source, '/virtual/example.js');

      const tempDir = fs.mkdtempSync(
        path.join(os.tmpdir(), 'stylex-unplugin-hashed-'),
      );
      try {
        const assetsDir = path.join(tempDir, 'assets');
        fs.mkdirSync(assetsDir, { recursive: true });
        const entryFile = path.join(assetsDir, 'index-W1erjkBN.css');
        const routeFile = path.join(assetsDir, 'admin-CyH6f07N.css');
        fs.writeFileSync(entryFile, 'body{color:red}', 'utf8');
        fs.writeFileSync(routeFile, '.admin{color:green}', 'utf8');

        await plugin.writeBundle({ dir: tempDir }, {});

        expect(fs.readFileSync(entryFile, 'utf8')).toContain(marker);
        expect(fs.readFileSync(routeFile, 'utf8')).not.toContain(marker);
      } finally {
        fs.rmSync(tempDir, { recursive: true, force: true });
      }
    });
  });

  describe('browserslist & light-dark() handling', () => {
    const LIGHT_DARK_SOURCE = `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({ foo: { color: 'light-dark(#000, #fff)' } });
      export default styles;
    `;

    async function collectCssWithOptions(options) {
      const plugin = unpluginFactory(
        { runtimeInjection: false, ...options },
        { framework: 'rollup' },
      );
      plugin.buildStart();
      await plugin.transform.call({}, LIGHT_DARK_SOURCE, '/virtual/test.js');
      return plugin.__stylexCollectCss();
    }

    test('preserves light-dark() when Features.LightDark is excluded', async () => {
      const css = await collectCssWithOptions({
        lightningcssOptions: {
          exclude: Features.LightDark,
          minify: false,
        },
      });
      expect(css).toContain('light-dark(');
      expect(css).not.toContain('--lightningcss-light');
    });

    test('preserves light-dark() with modern browser targets', async () => {
      const css = await collectCssWithOptions({
        lightningcssOptions: {
          targets: browserslistToTargets(browserslist('Chrome >= 123')),
          minify: false,
        },
      });
      expect(css).toContain('light-dark(');
      expect(css).not.toContain('--lightningcss-light');
    });

    test('lowers light-dark() when targets do not support it', async () => {
      const css = await collectCssWithOptions({
        lightningcssOptions: {
          targets: browserslistToTargets(browserslist('Chrome >= 80')),
          minify: false,
        },
      });
      expect(css).toContain('--lightningcss-light');
      expect(css).not.toContain('light-dark(');
    });

    test('uses project browserslist by default (not hardcoded >= 1%)', async () => {
      const css = await collectCssWithOptions({
        lightningcssOptions: { minify: false },
      });
      // The CSS should be valid regardless of which browserslist resolves.
      // The key assertion: the output is non-empty and contains the color rule.
      expect(css).toContain('color:');
      expect(css).toContain('.x1aif7nf');
    });
  });

  // Regression: https://github.com/facebook/stylex/issues/1836
  // The Vite adapter's shared-store polling timer must never keep the process
  // alive. Vitest's Vite server has no `httpServer`, so the `close` cleanup is
  // skipped and a still-referenced interval would hang `vitest run` on exit.
  describe('Vite dev shared-store polling timer', () => {
    function makeServer(httpServer) {
      return {
        middlewares: { use: () => {} },
        ws: { send: () => {} },
        httpServer,
      };
    }

    test('is unref’d when the server has no httpServer (Vitest)', () => {
      const plugin = unplugin.vite({ dev: true });
      expect(typeof plugin.configureServer).toBe('function');

      const server = makeServer(null);
      plugin.configureServer(server);

      const interval = server.__stylexSharedPollingInterval;
      expect(interval).toBeDefined();
      // The timer must not hold the event loop open.
      expect(typeof interval.hasRef).toBe('function');
      expect(interval.hasRef()).toBe(false);
      clearInterval(interval);
    });

    test('is cleared when the httpServer closes', () => {
      jest.useFakeTimers();
      try {
        const plugin = unplugin.vite({ dev: true });
        const closeListeners = [];
        const httpServer = {
          once(event, fn) {
            if (event === 'close') closeListeners.push(fn);
          },
        };
        const sent = [];
        const server = {
          middlewares: { use: () => {} },
          ws: { send: (msg) => sent.push(msg) },
          httpServer,
        };
        plugin.configureServer(server);
        expect(server.__stylexSharedPollingInterval).toBeDefined();
        expect(closeListeners).toHaveLength(1);

        const shared = plugin.__stylexGetSharedStore();
        const updates = () =>
          sent.filter((m) => m && m.event === 'stylex:css-update');

        // A version bump is picked up on the next poll.
        shared.version += 1;
        jest.advanceTimersByTime(150);
        expect(updates()).toHaveLength(1);

        // Simulate the server closing: the timer is cleared, so later bumps
        // are never broadcast.
        closeListeners[0]();
        shared.version += 1;
        jest.advanceTimersByTime(1500);
        expect(updates()).toHaveLength(1);
      } finally {
        jest.useRealTimers();
      }
    });
  });
});
