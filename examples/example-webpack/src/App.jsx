/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import React from 'react';
import webpackLogo from './assets/webpack.svg';
import stylexLogo from './assets/stylex.svg';
import * as stylex from '@stylexjs/stylex';
import { colors } from './tokens.stylex';

import './app.css';
import CtaButton from './components/CTAButton';

export default function App() {
  return (
    <main {...stylex.props(styles.main)}>
      <div {...stylex.props(styles.logoContainer)}>
        <a href="https://webpack.js.org" target="_blank">
          <img
            alt="Webpack Logo"
            src={webpackLogo}
            {...stylex.props(styles.logo)}
          />
        </a>
        <a href="https://stylexjs.com" target="_blank">
          <img
            alt="StyleX Logo"
            src={stylexLogo}
            {...stylex.props(styles.logo)}
          />
        </a>
      </div>
      <h1 {...stylex.props(styles.header)}>Webpack + StyleX</h1>
      <div {...stylex.props(styles.card)}>
        <CtaButton color="pink" to="https://stylexjs.com">
          Get Started
        </CtaButton>
        <CtaButton
          color="blue"
          to="https://stylexjs.com/docs/learn/thinking-in-stylex/"
        >
          Thinking in StyleX
        </CtaButton>
      </div>
    </main>
  );
}

const styles = stylex.create({
  main: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2rem',
    columnGap: '2rem',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100vh',
    padding: '2rem',
  },
  logoContainer: {
    display: 'flex',
    rowGap: '2rem',
    columnGap: '2rem',
  },
  logo: {
    height: '6em',
    filter: {
      default: null,
      ':hover': 'drop-shadow(0 0 2em #646cffaa)',
    },
    transition: 'filter 300ms',
    willChange: 'filter',
  },
  header: {
    fontSize: '3.2em',
    lineHeight: 1.1,
    color: colors.textPrimary,
  },
  card: {
    display: 'flex',
    rowGap: '2rem',
    columnGap: '2rem',
  },
});
