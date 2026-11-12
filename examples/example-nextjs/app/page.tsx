/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

'use client';

import { useState, useEffect } from 'react';
import * as stylex from '@stylexjs/stylex';
import Card from '@/components/Card';
import {
  globalTokens as $,
  spacing,
  text,
  colors,
} from './globalTokens.stylex';
import InteractiveCard, { THEMES } from './InteractiveCard';
import { darkTheme, lightTheme } from './darkMode.stylex';

const HOMEPAGE = 'https://stylexjs.com';

export default function Home() {
  const [themeIndex, setThemeIndex] = useState(2);
  const [isDark, setIsDark] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    setIsDark(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsDark(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  return (
    <main
      {...stylex.props(
        style.main,
        THEMES[themeIndex].theme,
        isDark ? darkTheme : lightTheme,
      )}
    >
      <div {...stylex.props(style.hero)}>
        <h1 {...stylex.props(style.h1)}>
          Next.js <span {...stylex.props(style.plus)}>+</span> StyleX
        </h1>
        <p {...stylex.props(style.subtitle)}>
          The expressive styling system for ambitious interfaces
        </p>
        <InteractiveCard
          isDark={isDark}
          onDarkModeChange={setIsDark}
          onThemeChange={setThemeIndex}
          themeIndex={themeIndex}
        />
      </div>

      {/* Link to nested theming demos hub */}
      <a {...stylex.props(style.nestedLink)} href="/theming-demos">
        <span {...stylex.props(style.nestedLinkIcon)}>🎨</span>
        <span>
          <strong>Theming Demos</strong>
          <br />
          <span {...stylex.props(style.nestedLinkSub)}>
            Experimental APIs for expressing nested design tokens · Design
            system theming
          </span>
        </span>
        <span {...stylex.props(style.nestedLinkArrow)}>→</span>
      </a>

      <div {...stylex.props(style.grid)}>
        <Card
          body="Learn how to use StyleX to build UIs"
          href={`${HOMEPAGE}/docs/learn/`}
          title="Docs"
        />
        <Card
          body="Browse through the StyleX API reference"
          href={`${HOMEPAGE}/docs/api/`}
          title="API"
        />
        <Card
          body="Play with StyleX and look at the compile outputs"
          href={`${HOMEPAGE}/playground/`}
          title="Playground"
        />
        <Card
          body="Get started with a NextJS+StyleX project"
          href="https://github.com/nmn/nextjs-app-dir-stylex"
          title="Templates"
        />
      </div>
    </main>
  );
}

const MEDIA_MOBILE = '@media (max-width: 700px)' as const;
const MEDIA_TABLET =
  '@media (min-width: 701px) and (max-width: 1120px)' as const;
const DARK = '@media (prefers-color-scheme: dark)' as const;

const style = stylex.create({
  main: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.xxxl,
    alignItems: 'center',
    justifyContent: 'center',
    minHeight: '100vh',
    paddingInline: spacing.md,
    paddingTop: spacing.xxxl,
    paddingBottom: {
      [MEDIA_MOBILE]: spacing.lg,
      default: spacing.xxl,
    },
    color: `rgba(${$.foregroundR}, ${$.foregroundG}, ${$.foregroundB}, 1)`,
    backgroundColor: $.surfaceBg,
  },
  hero: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.lg,
    alignItems: 'center',
    justifyContent: 'center',
    width: '100%',
    maxWidth: 480,
  },
  h1: {
    flexDirection: {
      [MEDIA_MOBILE]: 'column',
      default: 'row',
    },
    fontFamily: $.fontSans,
    fontSize: text.h1,
    fontWeight: 700,
    lineHeight: 1,
    textAlign: 'center',
    letterSpacing: '-0.02em',
    whiteSpace: 'nowrap',
  },
  plus: {
    fontWeight: 300,
    color: colors.accent,
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },
  subtitle: {
    maxWidth: '36ch',
    fontFamily: $.fontSans,
    fontSize: text.p,
    lineHeight: 1.6,
    color: {
      [DARK]: colors.gray6,
      default: colors.gray5,
    },
    textAlign: 'center',
    textWrap: 'balance',
  },
  nestedLink: {
    display: 'flex',
    gap: spacing.md,
    alignItems: 'center',
    width: '100%',
    maxWidth: 480,
    paddingBlock: spacing.md,
    paddingInline: spacing.lg,
    fontFamily: $.fontSans,
    fontSize: text.sm,
    lineHeight: 1.4,
    color: 'inherit',
    textDecoration: 'none',
    backgroundColor: {
      default: colors.accentLight,
      ':hover': colors.accentFaded,
    },
    borderColor: colors.accent,
    borderStyle: 'solid',
    borderWidth: 2,
    borderRadius: spacing.sm,
    transform: {
      default: null,
      ':hover': 'translateY(-2px)',
    },
    transitionDuration: '200ms',
    transitionProperty: 'background-color, border-color, transform',
  },
  nestedLinkIcon: {
    flexShrink: 0,
    fontSize: text.h4,
  },
  nestedLinkSub: {
    fontSize: text.xs,
    opacity: 0.6,
  },
  nestedLinkArrow: {
    marginLeft: 'auto',
    fontSize: text.h5,
    color: colors.accent,
    transitionDuration: '200ms',
    transitionProperty: 'transform',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: {
      [MEDIA_MOBILE]: '1fr',
      [MEDIA_TABLET]: 'repeat(2, 50%)',
      default: 'repeat(4, minmax(25%, auto))',
    },
    gap: spacing.sm,
    width: $.maxWidth,
    maxWidth: {
      [MEDIA_MOBILE]: 320,
      default: '100%',
    },
    textAlign: { [MEDIA_MOBILE]: 'center' },
  },
});
