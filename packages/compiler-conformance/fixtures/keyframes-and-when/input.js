/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as stylex from '@stylexjs/stylex';

export const styles = stylex.create({
  root: {
    animationName: stylex.keyframes({
      '0%': {
        opacity: 0,
      },
      '100%': {
        opacity: 1,
      },
    }),
    color: {
      default: 'red',
      [stylex.when.ancestor(':focus')]: 'blue',
    },
  },
});
