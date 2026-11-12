/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// themeVars.stylex.js
import * as stylex from '@stylexjs/stylex';
import { breakpoints } from './constants.stylex';

export const themeVars = stylex.defineVars({
  accent: {
    default: 'blue',
    [breakpoints.small]: 'red',
  },
});
