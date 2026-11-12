/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import path from 'node:path';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import { createVitePlugin } from 'unplugin';

import { devInjectMiddleware } from './dev-inject-middleware';
import {
  DEV_CSS_PATH,
  VIRTUAL_STYLEX_RUNTIME_SCRIPT,
  VIRTUAL_STYLEX_CSS_ONLY_SCRIPT,
} from './consts';
import {
  INDEX_CSS_RE,
  STYLE_CSS_RE,
  pickCssAssetFromRollupBundle,
  replaceCssAssetWithHashedCopy,
  unpluginFactory,
} from './core';

function attachViteHooks(plugin) {
  const cssInjectionTarget = plugin.__stylexCssInjectionTarget;
  const devMode = plugin.__stylexDevMode;
  const stylexPackages = plugin.__stylexPackages;
  const shared = plugin.__stylexGetSharedStore?.();
  let viteServer = null;
  let viteOutDir = null;
  let cssInjectedInGenerateBundle = false;

  return {
    ...plugin,
    config(config) {
      if (!stylexPackages || stylexPackages.length === 0) return;
      const addExcludes = (existing = []) =>
        Array.from(new Set([...existing, ...stylexPackages]));
      return {
        optimizeDeps: {
          ...(config?.optimizeDeps || {}),
          exclude: addExcludes(config?.optimizeDeps?.exclude || []),
        },
        ssr: {
          ...(config?.ssr || {}),
          optimizeDeps: {
            ...(config?.ssr?.optimizeDeps || {}),
            exclude: addExcludes(config?.ssr?.optimizeDeps?.exclude || []),
          },
        },
      };
    },
    configResolved(config) {
      try {
        viteOutDir = config.build?.outDir || viteOutDir;
      } catch {}
    },
    configureServer(server) {
      viteServer = server;
      if (devMode === 'full') {
        server.middlewares.use(devInjectMiddleware);
      }
      server.middlewares.use((req, res, next) => {
        if (!req.url) return next();
        if (req.url.startsWith(DEV_CSS_PATH)) {
          res.statusCode = 200;
          res.setHeader('Content-Type', 'text/css');
          res.setHeader('Cache-Control', 'no-store');
          const css = plugin.__stylexCollectCss?.();
          res.end(css || '');
          return;
        }
        next();
      });
      if (shared) {
        let lastVersion = shared.version;
        const interval = setInterval(() => {
          const curr = shared.version;
          if (curr !== lastVersion) {
            lastVersion = curr;
            try {
              server.ws.send({
                type: 'custom',
                event: 'stylex:css-update',
              });
            } catch {}
          }
        }, 150);
        // This dev-only polling timer must never keep the process alive. Under
        // Vitest (and Vite middleware mode) the server has no `httpServer`, so
        // the `close` cleanup below is skipped; a still-referenced interval
        // would then leak and hang the process on exit (~10s under
        // `vitest run`) (#1836).
        if (typeof interval.unref === 'function') {
          interval.unref();
        }
        server.httpServer?.once('close', () => clearInterval(interval));
        // Exposed so tests (and hosts that tear the server down without an
        // `httpServer` close event) can observe/clear the timer.
        server.__stylexSharedPollingInterval = interval;
      }
    },
    resolveId(id) {
      if (devMode === 'full' && id.includes('virtual:stylex:runtime'))
        return id;
      if (devMode === 'css-only' && id.includes('virtual:stylex:css-only'))
        return id;
      return null;
    },
    load(id) {
      if (devMode === 'full' && id.includes('virtual:stylex:runtime')) {
        return VIRTUAL_STYLEX_RUNTIME_SCRIPT;
      }
      if (devMode === 'css-only' && id.includes('virtual:stylex:css-only')) {
        return VIRTUAL_STYLEX_CSS_ONLY_SCRIPT;
      }
      return null;
    },
    transformIndexHtml() {
      if (devMode !== 'full') return null;
      if (!viteServer) return null;
      const base = viteServer.config.base ?? '';
      return [
        {
          tag: 'script',
          attrs: {
            type: 'module',
            src: path.join(base, '/@id/virtual:stylex:runtime'),
          },
          injectTo: 'head',
        },
        {
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: path.join(base, DEV_CSS_PATH),
          },
          injectTo: 'head',
        },
      ];
    },
    async handleHotUpdate(ctx) {
      const base = ctx.server.config.base ?? '';
      const cssMod = ctx.server.moduleGraph.getModuleById(
        path.join(base, 'virtual:stylex:css-module'),
      );
      if (cssMod) {
        ctx.server.moduleGraph.invalidateModule(cssMod);
      }
      try {
        ctx.server.ws.send({
          type: 'custom',
          event: 'stylex:css-update',
        });
      } catch {}
    },
    generateBundle(_opts, bundle) {
      const css = plugin.__stylexCollectCss?.();
      if (!css) return;
      const target = pickCssAssetFromRollupBundle(bundle, cssInjectionTarget);
      if (target) {
        const current =
          typeof target.source === 'string'
            ? target.source
            : target.source?.toString() || '';
        const nextSource = current ? current + '\n' + css : css;
        replaceCssAssetWithHashedCopy(this, bundle, target, nextSource);
        cssInjectedInGenerateBundle = true;
      }
    },
    async writeBundle(options, bundle) {
      if (cssInjectedInGenerateBundle) return;
      const css = plugin.__stylexCollectCss?.();
      if (!css) return;
      const target = pickCssAssetFromRollupBundle(bundle, cssInjectionTarget);
      const outDir =
        options?.dir ||
        (options?.file ? path.dirname(options.file) : viteOutDir);
      if (!outDir) return;
      try {
        await fsp.mkdir(outDir, { recursive: true });
      } catch {}
      let outfile;
      if (!target) {
        try {
          const assetsDir = path.join(outDir, 'assets');
          if (fs.existsSync(assetsDir)) {
            const files = fs
              .readdirSync(assetsDir)
              .filter((f) => f.endsWith('.css'));
            const pick =
              files.find((f) => INDEX_CSS_RE.test(f)) ||
              files.find((f) => STYLE_CSS_RE.test(f)) ||
              files[0];
            if (pick) outfile = path.join(assetsDir, pick);
          }
        } catch {}
        if (!outfile) {
          const assetsDir = path.join(outDir, 'assets');
          try {
            await fsp.mkdir(assetsDir, { recursive: true });
          } catch {}
          const fallback = path.join(assetsDir, 'stylex.css');
          await fsp.writeFile(fallback, css, 'utf8');
          return;
        }
      } else {
        outfile = path.join(outDir, target.fileName);
      }
      try {
        const current = fs.readFileSync(outfile, 'utf8');
        if (!current.includes(css)) {
          await fsp.writeFile(
            outfile,
            current ? current + '\n' + css : css,
            'utf8',
          );
        }
      } catch {}
    },
  };
}

export default createVitePlugin((options, metaOptions) =>
  attachViteHooks(unpluginFactory(options, metaOptions)),
);
