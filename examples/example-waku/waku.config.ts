/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import react from '@vitejs/plugin-react';
import stylex from '@stylexjs/unplugin';
import { defineConfig } from 'waku/config';

const stylexPlugin = stylex.vite({
  debug: process.env.NODE_ENV === 'development',
  enableDevClassNames: false,
  useCSSLayers: true,
  devMode: 'css-only',
  devPersistToDisk: true,
  runtimeInjection: false,
});

export default defineConfig({
  vite: {
    plugins: [
      // @ts-ignore
      stylexPlugin,
      // @ts-ignore
      react({
        babel: {
          // There is a bug with react compiler at the moment.
          // plugins: ['babel-plugin-react-compiler'],
        },
      }),
    ],
  },
});
