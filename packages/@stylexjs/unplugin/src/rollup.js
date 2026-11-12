/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import path from 'node:path';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import { createRollupPlugin } from 'unplugin';

import {
  INDEX_CSS_RE,
  STYLE_CSS_RE,
  pickCssAssetFromRollupBundle,
  replaceCssAssetWithHashedCopy,
  unpluginFactory,
} from './core';

export function attachRollupHooks(plugin) {
  const cssInjectionTarget = plugin.__stylexCssInjectionTarget;

  return {
    ...plugin,
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
      }
    },
    async writeBundle(options, bundle) {
      const css = plugin.__stylexCollectCss?.();
      if (!css) return;
      const target = pickCssAssetFromRollupBundle(bundle, cssInjectionTarget);
      const outDir =
        options?.dir || (options?.file ? path.dirname(options.file) : null);
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

export default createRollupPlugin((options, metaOptions) =>
  attachRollupHooks(unpluginFactory(options, metaOptions)),
);
