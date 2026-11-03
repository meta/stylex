/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { createInspectedPageRuntime } from './runtime';
import { INSPECTED_RUNTIME_KEY } from './runtimeKey';

(globalThis as any)[Symbol.for(INSPECTED_RUNTIME_KEY)] =
  createInspectedPageRuntime();
