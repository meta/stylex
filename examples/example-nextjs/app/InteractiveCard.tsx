/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

'use client';

import * as stylex from '@stylexjs/stylex';
import {
  spacing,
  text,
  globalTokens as $,
  colors,
} from './globalTokens.stylex';
import { useState } from 'react';

export const redTheme = stylex.createTheme(colors, {
  accent: '#e03131',
  accentLight: 'rgba(224, 49, 49, 0.08)',
  accentFaded: 'rgba(224, 49, 49, 0.19)',
});

export const greenTheme = stylex.createTheme(colors, {
  accent: '#2f9e44',
  accentLight: 'rgba(47, 158, 68, 0.08)',
  accentFaded: 'rgba(47, 158, 68, 0.19)',
});

export const blueTheme = stylex.createTheme(colors, {
  accent: '#1c7ed6',
  accentLight: 'rgba(28, 126, 214, 0.08)',
  accentFaded: 'rgba(28, 126, 214, 0.19)',
});

export const THEMES = [
  { name: 'Red', theme: redTheme },
  { name: 'Green', theme: greenTheme },
  { name: 'Blue', theme: blueTheme },
] as const;

type Props = Readonly<{
  themeIndex: number;
  onThemeChange: (_index: number) => void;
  isDark: boolean;
  onDarkModeChange: (_isDark: boolean) => void;
}>;

export default function InteractiveCard({
  themeIndex,
  onThemeChange,
  isDark,
  onDarkModeChange,
}: Props) {
  const [count, setCount] = useState(0);

  return (
    <div {...stylex.props(styles.card)}>
      <div {...stylex.props(styles.label)}>Theme</div>
      <div {...stylex.props(styles.themeRow)}>
        {THEMES.map((t, i) => (
          <button
            key={t.name}
            {...stylex.props(
              styles.themeButton,
              i === themeIndex && styles.themeButtonActive,
            )}
            onClick={() => onThemeChange(i)}
          >
            <span {...stylex.props(t.theme, styles.dot)} />
            {t.name}
          </button>
        ))}
      </div>
      <div {...stylex.props(styles.divider)} />
      <div {...stylex.props(styles.label)}>Mode</div>
      <div {...stylex.props(styles.themeRow)}>
        <button
          {...stylex.props(
            styles.themeButton,
            !isDark && styles.themeButtonActive,
          )}
          onClick={() => onDarkModeChange(false)}
        >
          Light
        </button>
        <button
          {...stylex.props(
            styles.themeButton,
            isDark && styles.themeButtonActive,
          )}
          onClick={() => onDarkModeChange(true)}
        >
          Dark
        </button>
      </div>
      <div {...stylex.props(styles.divider)} />
      <div {...stylex.props(styles.label)}>Counter</div>
      <div {...stylex.props(styles.counterRow)}>
        <button
          {...stylex.props(styles.counterButton)}
          onClick={() => setCount((c) => c - 1)}
        >
          -
        </button>
        <div
          {...stylex.props(
            styles.count,
            Math.abs(count) > 99 && styles.largeNumber,
          )}
        >
          {count}
        </div>
        <button
          {...stylex.props(styles.counterButton)}
          onClick={() => setCount((c) => c + 1)}
        >
          +
        </button>
      </div>
    </div>
  );
}

const DARK = '@media (prefers-color-scheme: dark)' as const;

const styles = stylex.create({
  card: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.sm,
    width: '100%',
    maxWidth: 400,
    padding: spacing.lg,
    fontFamily: $.fontSans,
    backgroundColor: $.surfaceCard,
    borderColor: colors.accent,
    borderStyle: 'solid',
    borderWidth: 2,
    borderRadius: spacing.md,
    boxShadow: $.surfaceCardShadow,
    transitionDuration: '300ms',
    transitionProperty: 'border-color, box-shadow',
  },
  label: {
    fontSize: text.xs,
    fontWeight: 700,
    color: colors.accent,
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },
  themeRow: {
    display: 'flex',
    gap: spacing.xs,
    justifyContent: 'center',
  },
  themeButton: {
    display: 'flex',
    gap: spacing.xxs,
    alignItems: 'center',
    paddingBlock: spacing.xs,
    paddingInline: spacing.sm,
    fontFamily: $.fontSans,
    fontSize: text.sm,
    color: 'inherit',
    cursor: 'pointer',
    backgroundColor: {
      default: 'transparent',
      ':hover': {
        [DARK]: colors.gray9,
        default: colors.gray2,
      },
    },
    borderColor: {
      [DARK]: colors.gray8,
      default: colors.gray3,
    },
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: spacing.xs,
    transitionDuration: '200ms',
    transitionProperty: 'background-color, border-color',
  },
  themeButtonActive: {
    fontWeight: 600,
    backgroundColor: colors.accentLight,
    borderColor: colors.accent,
  },
  dot: {
    flexShrink: 0,
    width: 10,
    height: 10,
    backgroundColor: colors.accent,
    borderRadius: '50%',
  },
  divider: {
    height: 2,
    marginBlock: spacing.xxs,
    backgroundColor: colors.accentFaded,
    borderRadius: 1,
    transitionDuration: '300ms',
    transitionProperty: 'background-color',
  },
  counterRow: {
    display: 'flex',
    gap: spacing.md,
    alignItems: 'center',
    justifyContent: 'center',
  },
  counterButton: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: 48,
    height: 48,
    fontFamily: $.fontSans,
    fontSize: text.h4,
    fontWeight: 300,
    color: colors.accent,
    cursor: 'pointer',
    backgroundColor: {
      default: 'transparent',
      ':hover': {
        [DARK]: colors.gray9,
        default: colors.gray2,
      },
    },
    borderColor: colors.accent,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: spacing.xs,
    transform: {
      default: null,
      ':active': 'scale(0.95)',
    },
    transitionDuration: '200ms',
    transitionProperty: 'background-color, transform, color, border-color',
  },
  count: {
    minWidth: 80,
    fontFamily: $.fontMono,
    fontSize: text.h2,
    fontWeight: 200,
    color: colors.accent,
    textAlign: 'center',
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },
  largeNumber: {
    fontSize: text.h3,
  },
});
