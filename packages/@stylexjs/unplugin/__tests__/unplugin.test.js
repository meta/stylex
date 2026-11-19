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
  perChunkAssetName,
  cssImportSpecifier,
  injectCssImport,
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

  // Regression: https://github.com/facebook/stylex/issues/2015
  // After a successful transform, replacing the module with code that has no
  // StyleX import used to skip cleanup (shouldHandle early-return). Removing
  // the last style while keeping the import already cleared CSS; dropping the
  // import must do the same.
  test('clears collected CSS when a module drops its StyleX import', async () => {
    const plugin = unpluginFactory(
      {
        dev: true,
        runtimeInjection: false,
        devPersistToDisk: false,
      },
      { framework: 'vite' },
    );
    plugin.buildStart();

    const id = '/virtual/import-removal-example.ts';
    const original = `import * as stylex from '@stylexjs/stylex';
export const styles = stylex.create({
  marker: { outlineWidth: 37, outlineOffset: 71 },
});
`;

    await plugin.transform.call({}, original, id);
    expect(plugin.__stylexCollectCss()).toEqual(
      expect.stringContaining('37px'),
    );
    expect(plugin.__stylexCollectCss()).toEqual(
      expect.stringContaining('71px'),
    );

    await plugin.transform.call(
      {},
      'import * as stylex from "@stylexjs/stylex";\nexport const styles = null;\n',
      id,
    );
    expect(plugin.__stylexCollectCss()).toBe('');

    await plugin.transform.call({}, original, id);
    expect(plugin.__stylexCollectCss()).toEqual(
      expect.stringContaining('37px'),
    );

    const result = await plugin.transform.call(
      {},
      'export const styles = null;\n',
      id,
    );
    expect(result).toBeNull();
    expect(plugin.__stylexCollectCss()).toBe('');
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

    plugin.generateBundle.call(ctx, {}, bundle);

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
    plugin.generateBundle.call(ctx, {}, {});

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

  describe('cssCodeSplit option normalization', () => {
    test('defaults to false', () => {
      expect(
        unpluginFactory({}, { framework: 'rollup' }).__stylexCssCodeSplit,
      ).toBe(false);
    });

    test('true is kept on the Rollup, Vite and Rolldown adapters', () => {
      expect(
        unpluginFactory({ cssCodeSplit: true }, { framework: 'rollup' })
          .__stylexCssCodeSplit,
      ).toBe(true);
      expect(
        unpluginFactory({ cssCodeSplit: true }, { framework: 'vite' })
          .__stylexCssCodeSplit,
      ).toBe(true);
      expect(
        unpluginFactory({ cssCodeSplit: true }, { framework: 'rolldown' })
          .__stylexCssCodeSplit,
      ).toBe(true);
    });

    test('true is silently downgraded on unsupported adapters', () => {
      expect(
        unpluginFactory({ cssCodeSplit: true }, { framework: 'webpack' })
          .__stylexCssCodeSplit,
      ).toBe(false);
    });

    test('runtimeInjection takes precedence over cssCodeSplit', () => {
      expect(
        unpluginFactory(
          { runtimeInjection: true, cssCodeSplit: true },
          { framework: 'rollup' },
        ).__stylexCssCodeSplit,
      ).toBe(false);
    });

    test('runtimeInjection alone does not enable cssCodeSplit', () => {
      expect(
        unpluginFactory({ runtimeInjection: true }, { framework: 'rollup' })
          .__stylexCssCodeSplit,
      ).toBe(false);
    });
  });

  describe('cssCodeSplit: per-chunk stylesheets', () => {
    const ID_A = '/virtual/per-chunk/a.js';
    const ID_B = '/virtual/per-chunk/b.js';
    const ID_PLAIN = '/virtual/per-chunk/plain.js';

    const STYLE_A = `
      import * as stylex from '@stylexjs/stylex';
      export const styles = stylex.create({ root: { paddingTop: 17 } });
    `;
    const STYLE_B = `
      import * as stylex from '@stylexjs/stylex';
      export const styles = stylex.create({ root: { paddingBottom: 29 } });
    `;

    const CSS_CODE_SPLIT_OPTIONS = {
      cssCodeSplit: true,
      runtimeInjection: false,
      devPersistToDisk: false,
      dev: false,
    };

    function makeCtx(warnings) {
      return {
        warn: (message) => warnings.push(message),
        emitFile() {
          return 'ref_1';
        },
        getFileName() {
          return null;
        },
      };
    }

    function makeBundle() {
      return {
        'dist/a.js': {
          type: 'chunk',
          fileName: 'dist/a.js',
          code: 'export {};\n',
          modules: { [ID_A]: {} },
        },
        'dist/b.js': {
          type: 'chunk',
          fileName: 'dist/b.js',
          code: 'export {};\n',
          modules: { [ID_B]: {} },
        },
        'dist/plain.js': {
          type: 'chunk',
          fileName: 'dist/plain.js',
          code: 'export {};\n',
          modules: { [ID_PLAIN]: {} },
        },
      };
    }

    async function transformModules(plugin) {
      await plugin.transform.call({}, STYLE_A, ID_A);
      await plugin.transform.call({}, STYLE_B, ID_B);
      await plugin.transform.call({}, 'export {};\n', ID_PLAIN);
    }

    // Clear rules leaked from earlier tests.
    function makeCodeSplitPlugin() {
      const plugin = unplugin.rollup(CSS_CODE_SPLIT_OPTIONS);
      plugin.__stylexGetSharedStore().rulesById.clear();
      return plugin;
    }

    // Stand-in for the bundler's PluginContext: an explicit fileName lands
    // verbatim (first emission wins), a name-based emission goes under
    // `assets/` with numbering on collision.
    function makeBundleContext(bundle) {
      const taken = new Set(
        Object.values(bundle)
          .filter((file) => file && file.type === 'asset')
          .map((file) => file.fileName),
      );
      const byNameAndSource = new Map();
      const byRefId = new Map();
      let nextRefId = 0;
      return {
        emitFile(file) {
          if (file.fileName) {
            nextRefId += 1;
            const refId = `ref_${nextRefId}`;
            if (!bundle[file.fileName]) {
              taken.add(file.fileName);
              bundle[file.fileName] = {
                type: 'asset',
                fileName: file.fileName,
                source: file.source,
              };
            }
            byRefId.set(refId, file.fileName);
            return refId;
          }
          const key = `${file.name}\u0000${file.source}`;
          const existing = byNameAndSource.get(key);
          if (existing) return existing.refId;
          const ext = path.posix.extname(file.name);
          const stem = path.posix.basename(file.name, ext);
          let fileName = `assets/${stem}${ext}`;
          let n = 2;
          while (taken.has(fileName)) {
            fileName = `assets/${stem}${n}${ext}`;
            n += 1;
          }
          taken.add(fileName);
          nextRefId += 1;
          const refId = `ref_${nextRefId}`;
          byNameAndSource.set(key, { refId, fileName });
          byRefId.set(refId, fileName);
          bundle[fileName] = {
            type: 'asset',
            fileName,
            source: file.source,
          };
          return refId;
        },
        getFileName(refId) {
          return byRefId.get(refId) ?? null;
        },
      };
    }

    test('derives the stylesheet path and the import specifier from chunk file names', () => {
      expect(perChunkAssetName('a.js')).toBe('a.stylex.css');
      expect(perChunkAssetName('dist/a.mjs')).toBe('dist/a.stylex.css');
      expect(perChunkAssetName('assets/index-B5Jdbbfd.js')).toBe(
        'assets/index-B5Jdbbfd.stylex.css',
      );
      expect(cssImportSpecifier('a.js', 'a.stylex.css')).toBe('./a.stylex.css');
      expect(cssImportSpecifier('dist/a.js', 'dist/a.stylex.css')).toBe(
        './a.stylex.css',
      );
      expect(cssImportSpecifier('dist/a.js', 'a.stylex.css')).toBe(
        '../a.stylex.css',
      );
    });

    test('injectCssImport places the import at the very top when the chunk has no imports', () => {
      const chunk = { code: 'const y = 2;\nexport { y };' };
      injectCssImport(chunk, './b.stylex.css');
      expect(chunk.code).toBe(
        "import './b.stylex.css';\nconst y = 2;\nexport { y };",
      );
    });

    test('injectCssImport places the import after the last leading import', () => {
      const chunk = {
        code: "/*!\n * banner\n */\nimport './one.js';\nimport {\n  a as b,\n} from './dep.js';\nconst x = 1;\nexport { x };",
      };
      injectCssImport(chunk, './a.stylex.css');
      expect(chunk.code).toBe(
        "/*!\n * banner\n */\nimport './one.js';\nimport {\n  a as b,\n} from './dep.js';\nimport './a.stylex.css';\nconst x = 1;\nexport { x };",
      );
    });

    test('injectCssImport places the import after user stylesheets so StyleX wins the cascade', () => {
      const chunk = {
        code: "import './d.stylex.css';\nimport './d1.stylex.css';\nexport const styles = {};",
      };
      injectCssImport(chunk, './d2.stylex.css');
      expect(chunk.code).toBe(
        "import './d.stylex.css';\nimport './d1.stylex.css';\nimport './d2.stylex.css';\nexport const styles = {};",
      );
    });

    test('injectCssImport places the import before a comment that closes mid-line', () => {
      const chunk = { code: '/* c */ const x = 1;' };
      injectCssImport(chunk, './a.stylex.css');
      expect(chunk.code).toBe("import './a.stylex.css';\n/* c */ const x = 1;");
    });

    test('injectCssImport keeps a trailing sourceMappingURL comment as the last line', () => {
      const chunk = {
        code: 'import "./dep.js";\nexport const x = 1;\n//# sourceMappingURL=a.js.map',
      };
      injectCssImport(chunk, './a.stylex.css');
      expect(chunk.code).toBe(
        'import "./dep.js";\nimport \'./a.stylex.css\';\nexport const x = 1;\n//# sourceMappingURL=a.js.map',
      );
      expect(chunk.code.endsWith('//# sourceMappingURL=a.js.map')).toBe(true);
    });

    // The import must land after the prologue, not between its directives.
    test('injectCssImport keeps a directive prologue above the import', () => {
      const chunk = {
        code: "'use client';\nimport './dep.js';\nexport const x = 1;",
      };
      injectCssImport(chunk, './a.stylex.css');
      expect(chunk.code).toBe(
        "'use client';\nimport './dep.js';\nimport './a.stylex.css';\nexport const x = 1;",
      );
    });

    test('injectCssImport keeps hashbangs, comments and multiple directives above the import', () => {
      const chunk = {
        code: "#!/usr/bin/env node\n'use client';\n// served pre-rendered\n'use strict';\nimport './dep.js';",
      };
      injectCssImport(chunk, './a.stylex.css');
      expect(chunk.code).toBe(
        "#!/usr/bin/env node\n'use client';\n// served pre-rendered\n'use strict';\nimport './dep.js';\nimport './a.stylex.css';",
      );
    });

    test('emits one stylesheet per chunk and wires a side-effect import', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await transformModules(plugin);
      const bundle = makeBundle();
      plugin.generateBundle.call(
        makeBundleContext(bundle),
        { format: 'es' },
        bundle,
      );

      const aCss = bundle['dist/a.stylex.css'];
      const bCss = bundle['dist/b.stylex.css'];
      expect(aCss).toBeDefined();
      expect(bCss).toBeDefined();
      expect(aCss.source).toContain('padding-top: 17px');
      expect(bCss.source).toContain('padding-bottom: 29px');
      expect(aCss.source).not.toContain('padding-bottom');
      expect(bCss.source).not.toContain('padding-top');
      expect(bundle['dist/plain.stylex.css']).toBeUndefined();
      expect(bundle['dist/a.js'].code).toContain("import './a.stylex.css';");
      expect(bundle['dist/b.js'].code).toContain("import './b.stylex.css';");
      expect(bundle['dist/plain.js'].code).not.toContain('.css');
    });

    test('the Vite adapter emits per-chunk stylesheets as well', async () => {
      const plugin = unplugin.vite(CSS_CODE_SPLIT_OPTIONS);
      // Ignore rules collected by earlier tests.
      plugin.__stylexGetSharedStore().rulesById.clear();
      if (typeof plugin.buildStart === 'function') {
        plugin.buildStart();
      }
      await transformModules(plugin);
      const bundle = makeBundle();
      plugin.generateBundle.call(
        makeBundleContext(bundle),
        { format: 'es' },
        bundle,
      );

      expect(bundle['dist/a.stylex.css'].source).toContain('padding-top: 17px');
      expect(bundle['dist/b.stylex.css'].source).toContain(
        'padding-bottom: 29px',
      );
      expect(bundle['dist/a.js'].code).toContain("import './a.stylex.css';");
      expect(bundle['dist/b.js'].code).toContain("import './b.stylex.css';");
    });

    test('chunks with identical CSS share one stylesheet', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await plugin.transform.call({}, STYLE_A, ID_A);
      await plugin.transform.call({}, STYLE_A, ID_B);
      const bundle = {
        'dist/a.js': {
          type: 'chunk',
          fileName: 'dist/a.js',
          code: 'export {};\n',
          modules: { [ID_A]: {} },
        },
        'dist/b.js': {
          type: 'chunk',
          fileName: 'dist/b.js',
          code: 'export {};\n',
          modules: { [ID_B]: {} },
        },
      };
      plugin.generateBundle.call(
        makeBundleContext(bundle),
        { format: 'es' },
        bundle,
      );

      expect(bundle['dist/a.stylex.css'].source).toContain('padding-top: 17px');
      expect(bundle['dist/b.stylex.css']).toBeUndefined();
      expect(bundle['dist/a.js'].code).toContain("import './a.stylex.css';");
      expect(bundle['dist/b.js'].code).toContain("import './a.stylex.css';");
    });

    test('emits a stylesheet when the chunk already imports an external .stylex.css file', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await plugin.transform.call(
        {},
        `import './vendor.stylex.css';\n${STYLE_A}`,
        ID_A,
      );
      const bundle = {
        'dist/a.js': {
          type: 'chunk',
          fileName: 'dist/a.js',
          code: "import './vendor.stylex.css';\nexport {};\n",
          modules: { [ID_A]: {} },
        },
      };
      plugin.generateBundle.call(
        makeBundleContext(bundle),
        { format: 'es' },
        bundle,
      );

      const aCss = bundle['dist/a.stylex.css'];
      expect(aCss).toBeDefined();
      expect(aCss.source).toContain('padding-top: 17px');
      const code = bundle['dist/a.js'].code;
      const vendorMatch = /import\s*['"]([^'"]*vendor\.stylex\.css)['"]/.exec(
        code,
      );
      expect(vendorMatch).not.toBeNull();
      expect(code).toContain("import './a.stylex.css';");
      expect(code.indexOf("import './a.stylex.css';")).toBeGreaterThan(
        vendorMatch.index,
      );
    });

    test('does not collide with a user asset holding the same basename', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await plugin.transform.call({}, STYLE_A, ID_A);
      const bundle = {
        'dist/a.js': {
          type: 'chunk',
          fileName: 'dist/a.js',
          code: 'export {};\n',
          modules: { [ID_A]: {} },
        },
        'assets/a.stylex.css': {
          type: 'asset',
          fileName: 'assets/a.stylex.css',
          source: '/* user css */',
        },
      };
      plugin.generateBundle.call(
        makeBundleContext(bundle),
        { format: 'es' },
        bundle,
      );

      expect(bundle['assets/a.stylex.css'].source).toBe('/* user css */');
      expect(bundle['dist/a.stylex.css'].source).toContain('padding-top: 17px');
      expect(bundle['dist/a.js'].code).toContain("import './a.stylex.css';");
    });

    test('falls back to a numbered stylesheet when a user asset holds its path', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await plugin.transform.call({}, STYLE_A, ID_A);
      const bundle = {
        'dist/a.js': {
          type: 'chunk',
          fileName: 'dist/a.js',
          code: "import './a.stylex.css';\nexport {};\n",
          modules: { [ID_A]: {} },
        },
        'dist/a.stylex.css': {
          type: 'asset',
          fileName: 'dist/a.stylex.css',
          source: '/* user css */',
        },
      };
      const warnings = [];
      const ctx = {
        ...makeBundleContext(bundle),
        warn: (message) => warnings.push(message),
      };
      plugin.generateBundle.call(ctx, { format: 'es' }, bundle);

      // The user's file stays untouched; the chunk's CSS moves to a1.
      expect(bundle['dist/a.stylex.css'].source).toBe('/* user css */');
      expect(bundle['dist/a1.stylex.css'].source).toContain(
        'padding-top: 17px',
      );
      expect(bundle['dist/a.js'].code).toContain("import './a.stylex.css';");
      expect(bundle['dist/a.js'].code).toContain("import './a1.stylex.css';");
      expect(warnings).toHaveLength(1);
      expect(warnings[0]).toContain('"dist/a.stylex.css" already exists');
      expect(warnings[0]).toContain('"dist/a1.stylex.css"');

      // A repeated pass reuses a1 instead of spawning a2.
      plugin.generateBundle.call(ctx, { format: 'es' }, bundle);
      expect(bundle['dist/a2.stylex.css']).toBeUndefined();
      expect(warnings).toHaveLength(1);
      expect(
        bundle['dist/a.js'].code.match(/import '\.\/a1?\.stylex\.css';/g),
      ).toHaveLength(2);
    });

    test('falls back to the aggregated stylesheet when the output format is not ESM', async () => {
      const plugin = unplugin.rollup(CSS_CODE_SPLIT_OPTIONS);
      plugin.buildStart();
      await transformModules(plugin);
      const bundle = makeBundle();
      bundle['assets/index.css'] = {
        type: 'asset',
        fileName: 'assets/index.css',
        source: '/* existing css */',
      };
      plugin.generateBundle.call(makeCtx([]), { format: 'cjs' }, bundle);

      expect(bundle['assets/index.css'].source).toContain('padding-top');
      expect(bundle['assets/index.css'].source).toContain('padding-bottom');
      expect(bundle['dist/a.stylex.css']).toBeUndefined();
      expect(bundle['dist/a.js'].code).not.toContain('.css');
    });

    test('a non-ESM output does not downgrade later ESM outputs', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await transformModules(plugin);

      // First output: CJS, aggregated instead of per chunk.
      const cjsBundle = makeBundle();
      cjsBundle['assets/index-W1erjkBN.css'] = {
        type: 'asset',
        fileName: 'assets/index-W1erjkBN.css',
        source: '/* existing css */',
      };
      plugin.generateBundle.call(
        makeBundleContext(cjsBundle),
        { format: 'cjs' },
        cjsBundle,
      );

      const cjsCss = Object.values(cjsBundle).filter(
        (file) => file.type === 'asset',
      );
      expect(cjsCss).toHaveLength(1);
      expect(cjsCss[0].source).toContain('padding-top');
      expect(cjsCss[0].source).toContain('padding-bottom');
      expect(cjsCss.some((file) => file.source === '/* existing css */')).toBe(
        false,
      );

      // Later output: ESM — per-chunk again despite the earlier CJS output.
      const esBundle = makeBundle();
      esBundle['assets/index-W1erjkBN.css'] = {
        type: 'asset',
        fileName: 'assets/index-W1erjkBN.css',
        source: '/* existing css */',
      };
      plugin.generateBundle.call(
        makeBundleContext(esBundle),
        { format: 'es' },
        esBundle,
      );

      expect(esBundle['dist/a.stylex.css'].source).toContain(
        'padding-top: 17px',
      );
      expect(esBundle['dist/b.stylex.css'].source).toContain(
        'padding-bottom: 29px',
      );
      expect(esBundle['assets/index-W1erjkBN.css'].source).toBe(
        '/* existing css */',
      );
      expect(esBundle['dist/a.js'].code).toContain("import './a.stylex.css';");
    });

    test("an ESM per-chunk output does not suppress a later output's writeBundle fallback", async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await transformModules(plugin);

      // First output: ESM, so stylesheets are emitted per chunk.
      const esBundle = makeBundle();
      plugin.generateBundle.call(
        makeBundleContext(esBundle),
        { format: 'es' },
        esBundle,
      );
      expect(esBundle['dist/a.stylex.css']).toBeDefined();

      // Second output: CJS, no CSS asset — the writeBundle fallback must
      // still run.
      const tempDir = fs.mkdtempSync(
        path.join(os.tmpdir(), 'stylex-per-chunk-es-then-cjs-'),
      );
      try {
        const cjsBundle = makeBundle();
        plugin.generateBundle.call(makeCtx([]), { format: 'cjs' }, cjsBundle);
        await plugin.writeBundle({ dir: tempDir }, cjsBundle);
        const outfile = path.join(tempDir, 'assets', 'stylex.css');
        expect(fs.existsSync(outfile)).toBe(true);
        const written = fs.readFileSync(outfile, 'utf8');
        expect(written).toContain('padding-top: 17px');
        expect(written).toContain('padding-bottom: 29px');
      } finally {
        fs.rmSync(tempDir, { recursive: true, force: true });
      }
    });

    test("an earlier output's generateBundle injection does not suppress a later output's fallback (Vite)", async () => {
      const plugin = unplugin.vite(CSS_CODE_SPLIT_OPTIONS);
      // Ignore rules collected by earlier tests.
      plugin.__stylexGetSharedStore().rulesById.clear();
      if (typeof plugin.buildStart === 'function') {
        plugin.buildStart();
      }
      await transformModules(plugin);

      // First output: CJS with an entry stylesheet — the aggregated path runs.
      const firstBundle = makeBundle();
      firstBundle['assets/index-W1erjkBN.css'] = {
        type: 'asset',
        fileName: 'assets/index-W1erjkBN.css',
        source: '/* existing css */',
      };
      plugin.generateBundle.call(
        makeBundleContext(firstBundle),
        { format: 'cjs' },
        firstBundle,
      );
      expect(firstBundle['assets/index-W1erjkBN.css']).toBeUndefined();

      // Second output: CJS, no CSS asset — the writeBundle fallback must
      // still run.
      const tempDir = fs.mkdtempSync(
        path.join(os.tmpdir(), 'stylex-per-chunk-vite-cjs-'),
      );
      try {
        const secondBundle = makeBundle();
        plugin.generateBundle.call(
          makeCtx([]),
          { format: 'cjs' },
          secondBundle,
        );
        await plugin.writeBundle({ dir: tempDir }, secondBundle);
        const outfile = path.join(tempDir, 'assets', 'stylex.css');
        expect(fs.existsSync(outfile)).toBe(true);
        const written = fs.readFileSync(outfile, 'utf8');
        expect(written).toContain('padding-top: 17px');
        expect(written).toContain('padding-bottom: 29px');
      } finally {
        fs.rmSync(tempDir, { recursive: true, force: true });
      }
    });

    test('runtimeInjection wins over cssCodeSplit and emits no static CSS', async () => {
      const plugin = unplugin.rollup({
        ...CSS_CODE_SPLIT_OPTIONS,
        runtimeInjection: true,
      });
      // Ignore rules collected by earlier tests.
      plugin.__stylexGetSharedStore().rulesById.clear();
      plugin.buildStart();
      await transformModules(plugin);
      const bundle = makeBundle();
      bundle['assets/index.css'] = {
        type: 'asset',
        fileName: 'assets/index.css',
        source: '/* existing css */',
      };
      plugin.generateBundle.call(makeCtx([]), { format: 'es' }, bundle);
      expect(bundle['assets/index.css'].source).toBe('/* existing css */');
      expect(bundle['dist/a.stylex.css']).toBeUndefined();
      expect(bundle['dist/a.js'].code).not.toContain('.css');
    });

    test('writeBundle skips the aggregate fallback after per-chunk emission', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await transformModules(plugin);
      const bundle = makeBundle();
      plugin.generateBundle.call(
        makeBundleContext(bundle),
        { format: 'es' },
        bundle,
      );

      const tempDir = fs.mkdtempSync(
        path.join(os.tmpdir(), 'stylex-per-chunk-write-'),
      );
      try {
        // The fallback must be suppressed after per-chunk emission.
        await plugin.writeBundle({ dir: tempDir }, {});
        expect(fs.readdirSync(tempDir)).toEqual([]);
      } finally {
        fs.rmSync(tempDir, { recursive: true, force: true });
      }
    });

    test('is idempotent when generateBundle runs twice', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await transformModules(plugin);
      const bundle = makeBundle();
      const ctx = makeBundleContext(bundle);
      plugin.generateBundle.call(ctx, { format: 'es' }, bundle);
      const firstCode = bundle['dist/a.js'].code;
      const cssAssets = () =>
        Object.values(bundle).filter((file) => file.type === 'asset');
      const wiredChunks = () =>
        Object.values(bundle).filter(
          (file) => file.type === 'chunk' && file.code.includes("import './"),
        );
      expect(cssAssets()).toHaveLength(2);
      expect(wiredChunks()).toHaveLength(2);

      // Simulate a bundler that fires generateBundle twice.
      plugin.generateBundle.call(ctx, { format: 'es' }, bundle);

      expect(cssAssets()).toHaveLength(2);
      expect(wiredChunks()).toHaveLength(2);
      expect(bundle['dist/a.js'].code).toBe(firstCode);
    });

    test('a repeated generateBundle keeps the stylesheet stable', async () => {
      const plugin = makeCodeSplitPlugin();
      plugin.buildStart();
      await plugin.transform.call({}, STYLE_A, ID_A);
      const bundle = {
        'dist/a.js': {
          type: 'chunk',
          fileName: 'dist/a.js',
          code: 'export {};\n',
          modules: { [ID_A]: {} },
        },
      };
      const ctx = makeBundleContext(bundle);
      plugin.generateBundle.call(ctx, { format: 'es' }, bundle);
      expect(bundle['dist/a.stylex.css'].source).toContain('padding-top: 17px');

      // A second pass must not duplicate anything.
      plugin.generateBundle.call(ctx, { format: 'es' }, bundle);
      expect(bundle['dist/a.stylex.css'].source).toContain('padding-top: 17px');
      expect(
        Object.values(bundle).filter((file) => file.type === 'asset'),
      ).toHaveLength(1);
      expect(
        bundle['dist/a.js'].code.match(/import '\.\/a\.stylex\.css';/g),
      ).toHaveLength(1);
    });
  });
});
