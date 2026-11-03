/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import * as stylex from '@stylexjs/stylex';

const colorsValue = {
  bg: 'light-dark(#ffffff, #282828)',
  bgRaised: 'light-dark(#f6f8fa, #282828)',
  textPrimary: 'light-dark(#000000, #ffffff)',
  textMuted: 'light-dark(#757575, #999999)',
  textAccent: 'light-dark(#dc362e, rgb(92 213 251))',
  secondaryAccent: 'light-dark(#0F7913, #73C89C)',
  border: 'light-dark(#d3e3fd, #5e5e5eff)',
  separator: 'light-dark(rgb(0 0 0 / 0.06), rgb(255 255 255 / 0.07))',
};

export const colors: typeof colorsValue = stylex.defineConsts(colorsValue);
