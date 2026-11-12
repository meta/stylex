/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

import './app.css';
import { globalTokens as $ } from '@/app/globalTokens.stylex';
import * as stylex from '@stylexjs/stylex';

export const metadata = {
  title: 'Next.js + StyleX',
  description: 'The expressive styling system for ambitious interfaces',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html {...stylex.props(styles.html, styles.reset)} lang="en">
      <body {...stylex.props(styles.reset, styles.body)}>{children}</body>
    </html>
  );
}

const styles = stylex.create({
  html: {
    colorScheme: 'light dark',
  },
  reset: {
    minHeight: '100%',
    padding: 0,
    margin: 0,
  },
  body: {
    backgroundColor: $.surfaceBg,
  },
});
