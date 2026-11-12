/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import { createRspackPlugin } from 'unplugin';

import { unpluginFactory } from './core';
import { attachWebpackHooks } from './webpack';

export default createRspackPlugin((options, metaOptions) =>
  attachWebpackHooks(unpluginFactory(options, metaOptions), 'rspack'),
);
