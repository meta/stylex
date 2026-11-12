/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import { createWebpackPlugin } from 'unplugin';

import { INDEX_CSS_RE, STYLE_CSS_RE, unpluginFactory } from './core';

export function attachWebpackHooks(plugin, hookName = 'webpack') {
  const cssInjectionTarget = plugin.__stylexCssInjectionTarget;

  return {
    ...plugin,
    [hookName](compiler) {
      const PLUGIN_NAME = '@stylexjs/unplugin';
      compiler.hooks.thisCompilation.tap(PLUGIN_NAME, (compilation) => {
        plugin.__stylexResetState?.();

        const wp = compiler.webpack || compiler.rspack || undefined;
        const stage = wp?.Compilation?.PROCESS_ASSETS_STAGE_SUMMARIZE;
        const tapOptions =
          stage != null ? { name: PLUGIN_NAME, stage } : PLUGIN_NAME;
        const toRawSource = (content) => {
          const RawSource = wp?.sources?.RawSource;
          return RawSource
            ? new RawSource(content)
            : { source: () => content, size: () => Buffer.byteLength(content) };
        };
        compilation.hooks.processAssets.tap(tapOptions, (assets) => {
          const css = plugin.__stylexCollectCss?.();
          if (!css) return;

          const cssAssets = Object.keys(assets).filter((f) =>
            f.endsWith('.css'),
          );
          if (cssAssets.length === 0) {
            compilation.warnings.push(
              new Error(
                '[stylex] No CSS asset found to inject into. Skipping.',
              ),
            );
            return;
          }

          const pickName =
            (typeof cssInjectionTarget === 'function' &&
              cssAssets.find((f) => cssInjectionTarget(f))) ||
            cssAssets.find((f) => INDEX_CSS_RE.test(f)) ||
            cssAssets.find((f) => STYLE_CSS_RE.test(f)) ||
            cssAssets[0];

          const asset = compilation.getAsset(pickName);
          if (!asset) return;
          const existing = asset.source.source().toString();
          const next = existing ? existing + '\n' + css : css;
          compilation.updateAsset(pickName, toRawSource(next));
        });
      });
    },
  };
}

export default createWebpackPlugin((options, metaOptions) =>
  attachWebpackHooks(unpluginFactory(options, metaOptions)),
);
