/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as stylex from '@stylexjs/stylex';

export const vars = stylex.defineVars({
  fg: 'red',
  bg: {
    default: 'white',
    '@media (prefers-color-scheme: dark)': 'black',
  },
});

export const theme = stylex.createTheme(vars, {
  fg: 'blue',
  bg: 'yellow',
});
