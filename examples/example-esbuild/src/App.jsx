/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

'use strict';

import './global.css';
import * as React from 'react';
import { createRoot } from 'react-dom/client';
import * as stylex from '@stylexjs/stylex';
import { colors, fonts, sizes } from './globalTokens.stylex';

const styles = stylex.create({
  main: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '100vw',
    height: '100vh',
    backgroundColor: colors.pink7,
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

function App() {
  return (
    <div {...stylex.props(styles.main)}>
      <div {...stylex.props(styles.card)}>
        <span>Blue rounded rectangle</span>
      </div>
    </div>
  );
}

createRoot(document.getElementById('root')).render(<App />);
