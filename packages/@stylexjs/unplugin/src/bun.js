/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import path from 'node:path';
import fsp from 'node:fs/promises';

import { unpluginFactory } from './core';

const loaders = {
  '.js': 'js',
  '.jsx': 'jsx',
  '.ts': 'ts',
  '.tsx': 'tsx',
};

export const createStylexBunPlugin = (userOptions = {}) => {
  const options = { ...userOptions };
  if (options.dev == null) options.dev = true;
  if (options.runtimeInjection == null) options.runtimeInjection = false;
  if (options.useCSSLayers == null) options.useCSSLayers = true;

  const plugin = unpluginFactory(options, { framework: 'bun' });
  const cssOutput =
    options.bunDevCssOutput ||
    path.resolve(process.cwd(), 'dist', 'stylex.dev.css');
  let lastCss = null;
  let writeQueue = Promise.resolve();

  const writeCss = (force = false) => {
    const css = plugin.__stylexCollectCss?.() || '';
    const next = css
      ? `:root { --stylex-injection: 0; }\n${css}`
      : ':root { --stylex-injection: 0; }';
    if (!force && next === lastCss) return writeQueue;
    lastCss = next;
    writeQueue = writeQueue
      .then(async () => {
        await fsp.mkdir(path.dirname(cssOutput), { recursive: true });
        await fsp.writeFile(cssOutput, next, 'utf8');
      })
      .catch(() => {});
    return writeQueue;
  };

  return {
    name: '@stylexjs/unplugin-bun',
    async setup(build) {
      if (plugin.buildStart) {
        build.onStart(async () => {
          await plugin.buildStart();
          await writeCss();
        });
      } else {
        build.onStart(async () => {
          await writeCss();
        });
      }

      if (plugin.buildEnd) {
        build.onEnd(async () => {
          await plugin.buildEnd();
          await writeCss(true);
        });
      } else {
        build.onEnd(async () => {
          await writeCss(true);
        });
      }

      build.onLoad({ filter: /\.[cm]?[jt]sx?$/ }, async (args) => {
        const code = await Bun.file(args.path).text();
        const result = plugin.transform
          ? await plugin.transform(code, args.path)
          : null;
        const nextCode = result?.code ?? code;
        await writeCss();
        return {
          contents: nextCode,
          loader: loaders[path.extname(args.path)] || 'js',
        };
      });
    },
  };
};

const defaultBunPlugin = createStylexBunPlugin({});

export default defaultBunPlugin;
