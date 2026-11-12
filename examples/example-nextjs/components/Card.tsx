/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

import * as stylex from '@stylexjs/stylex';
import {
  globalTokens as $,
  spacing,
  text,
  colors,
} from '@/app/globalTokens.stylex';
import { cardMarker, headingMarker } from './CardTokens.stylex';

type Props = Readonly<{
  title: string;
  body: string;
  href: string;
}>;

export default function Card({ title, body, href }: Props) {
  return (
    <a
      {...stylex.props(styles.link, cardMarker)}
      href={href}
      rel="noopener noreferrer"
      target="_blank"
    >
      <h2 {...stylex.props(styles.h2, headingMarker)}>
        {title} <span {...stylex.props(styles.span)}>→</span>
      </h2>
      <p {...stylex.props(styles.p)}>{body}</p>
    </a>
  );
}

type TMobile = '@media (max-width: 700px)';

const MOBILE: TMobile = '@media (max-width: 700px)' as TMobile;
const REDUCE_MOTION = '@media (prefers-reduced-motion: reduce)' as const;

const DARK = '@media (prefers-color-scheme: dark)' as const;

const styles = stylex.create({
  link: {
    display: {
      [MOBILE]: 'block',
      default: 'flex',
    },
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'flex-start',
    padding: spacing.md,
    fontFamily: $.fontSans,
    color: 'inherit',
    textAlign: 'center',
    textDecoration: 'none',
    backgroundColor: {
      default: 'transparent',
      ':hover': $.surfaceHover,
    },
    borderColor: {
      [DARK]: {
        default: colors.gray8,
        ':hover': colors.accent,
      },
      default: colors.gray2,
      ':hover': colors.accent,
    },
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: spacing.sm,
    boxShadow: {
      default: 'none',
      ':hover': '0 4px 16px rgba(0, 0, 0, 0.08)',
    },
    transform: {
      default: null,
      ':hover': 'translateY(-2px)',
    },
    transitionDuration: '300ms',
    transitionProperty: 'background-color, border-color, transform, box-shadow',
  },
  h2: {
    marginBottom: {
      [MOBILE]: spacing.xxs,
      default: spacing.xs,
    },
    fontSize: text.h4,
    fontWeight: 600,
    color: colors.accent,
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },
  span: {
    display: 'inline-block',
    transform: {
      default: null,
      [stylex.when.ancestor(':hover', cardMarker)]: 'translateX(10px)',
      [stylex.when.ancestor(':hover', headingMarker)]: 'translateX(4px)',
    },
    transitionDuration: {
      [REDUCE_MOTION]: '0s',
      default: '200ms',
    },
    transitionProperty: 'transform',
  },
  p: {
    maxWidth: '30ch',
    margin: 0,
    fontSize: text.p,
    lineHeight: 1.5,
    textWrap: 'balance',
    opacity: 0.6,
  },
});
