/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type { UserOptions } from './core';
import type vite from './vite';

export { unpluginFactory } from './core';
export type { UserOptions } from './core';
export { createStylexBunPlugin } from './bun';

declare const stylex: {
  bun: (options?: Partial<UserOptions>) => any;
  esbuild: (options?: Partial<UserOptions>) => any;
  farm: (options?: Partial<UserOptions>) => any;
  rolldown: (options?: Partial<UserOptions>) => any;
  rollup: (options?: Partial<UserOptions>) => any;
  rspack: (options?: Partial<UserOptions>) => any;
  unloader: (options?: Partial<UserOptions>) => any;
  vite: typeof vite;
  webpack: (options?: Partial<UserOptions>) => any;
  raw: typeof import('./core').unpluginFactory;
};

export const unplugin: typeof stylex;

export default stylex;
