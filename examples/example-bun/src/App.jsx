/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as React from 'react';
import * as stylex from '@stylexjs/stylex';
import { colors, fonts, sizes } from './globalTokens.stylex';

export default function App() {
  return (
    <main {...stylex.props(styles.main)}>
      <div {...stylex.props(styles.card)}>
        <span>StyleX + Bun + unplugin</span>
      </div>
    </main>
  );
}

const styles = stylex.create({
  main: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    minHeight: '100vh',
    backgroundColor: colors.gray0,
  },
  card: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: sizes.spacing5,
    fontFamily: fonts.mono,
    color: colors.gray0,
    backgroundColor: colors.blue9,
    borderRadius: sizes.spacing2,
  },
});
