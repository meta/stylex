/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import '../global.css';

import * as stylex from '@stylexjs/stylex';
import type { ReactNode } from 'react';
import { Footer } from '../components/footer';
import { Header } from '../components/header';
import { DevStyleXInject } from '../components/DevStyleXInject';

type RootLayoutProps = { children: ReactNode };

export default async function RootLayout({ children }: RootLayoutProps) {
  const data = await getData();

  return (
    <div {...stylex.props(styles.root)}>
      <meta content={data.description} name="description" />
      <link href={data.icon} rel="icon" type="image/png" />
      <link href="https://fonts.googleapis.com" rel="preconnect" />
      <link crossOrigin="" href="https://fonts.gstatic.com" rel="preconnect" />
      <link
        href="https://fonts.googleapis.com/css2?family=Nunito:ital,wght@0,400;0,700;1,400;1,700&display=swap"
        // eslint-disable-next-line
        precedence="font"
        rel="stylesheet"
      />
      <DevStyleXInject cssHref="/stylex.css" />
      <Header />
      <main {...stylex.props(styles.main)}>{children}</main>
      <Footer />
    </div>
  );
}

const getData = async () => {
  const data = {
    description: 'An internet website!',
    icon: '/images/favicon.png',
  };

  return data;
};

export const getConfig = async () => {
  return {
    render: 'static',
  } as const;
};

const styles = stylex.create({
  root: {
    minHeight: '100vh',
    fontFamily:
      "'Nunito', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    color: '#0f172a',
    backgroundColor: '#f5f7fb',
  },
  main: {
    display: 'flex',
    flexDirection: 'column',
    gap: '1.5rem',
    alignItems: 'center',
    justifyContent: {
      default: 'flex-start',
      '@media (min-width: 1024px)': 'center',
    },
    minHeight: {
      default: 'auto',
      '@media (min-width: 1024px)': '100svh',
    },
    margin: {
      default: '1.5rem',
      '@media (min-width: 1024px)': 0,
    },
  },
});
