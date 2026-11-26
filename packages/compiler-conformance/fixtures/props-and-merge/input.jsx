/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as stylex from '@stylexjs/stylex';
import React from 'react';

const styles = stylex.create({
  base: {
    color: 'red',
  },
  hover: {
    ':hover': {
      color: 'blue',
    },
  },
});

export function Button(props) {
  return <button {...stylex.props(styles.base, props.active && styles.hover)} />;
}
