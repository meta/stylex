/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import stylex from '@stylexjs/unplugin';
import rsc from '@vitejs/plugin-rsc/plugin';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    stylex.vite({
      useCSSLayers: true,
      runtimeInjection: false,
    }),
    react(),
    rsc({
      entries: {
        client: 'src/entry.browser.tsx',
        rsc: 'src/entry.rsc.tsx',
        ssr: 'src/entry.ssr.tsx',
      },
    }),
  ],
});
