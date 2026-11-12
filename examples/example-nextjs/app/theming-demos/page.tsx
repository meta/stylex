/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 */

import * as stylex from '@stylexjs/stylex';
import {
  globalTokens as $,
  spacing,
  text,
  colors,
} from '../globalTokens.stylex';

const DARK = '@media (prefers-color-scheme: dark)' as const;
const MOBILE = '@media (max-width: 700px)' as const;

const DEMOS = [
  {
    href: '/nested-demo',
    icon: '✨',
    title: 'Nested APIs Demo',
    subtitle:
      'unstable_defineVarsNested · unstable_defineConstsNested · unstable_createThemeNested',
    description:
      'Define design tokens as nested objects instead of flat key-value pairs. ' +
      'The compiler flattens them into the same CSS, with no performance cost. ',
  },
  {
    href: '/ds-demo',
    icon: '🎨',
    title: 'Design System Theming',
    subtitle: 'Primitives → Semantics → Themes (three-tier architecture)',
    description:
      'Express three-tier token architecture natively in StyleX. ' +
      'Primitives (defineConstsNested) feed into semantic tokens (defineVarsNested) ' +
      'which are overridden by themes (createThemeNested). Switch themes to re-skin everything.',
  },
];

export default function DemosPage() {
  return (
    <main {...stylex.props(s.page)}>
      <header {...stylex.props(s.header)}>
        <a {...stylex.props(s.backLink)} href="/">
          ← Back to main
        </a>
        <h1 {...stylex.props(s.title)}>
          Theming <span {...stylex.props(s.titleAccent)}>Demos</span>
        </h1>
        <p {...stylex.props(s.subtitle)}>
          Experimental APIs for nested design tokens in StyleX. Define
          hierarchical token structures, compose themes across packages, and let
          the compiler flatten everything to optimized CSS.
        </p>
      </header>

      <div {...stylex.props(s.grid)}>
        {DEMOS.map((demo) => (
          <a key={demo.href} {...stylex.props(s.card)} href={demo.href}>
            <div {...stylex.props(s.cardHeader)}>
              <span {...stylex.props(s.cardIcon)}>{demo.icon}</span>
              <div {...stylex.props(s.cardTitleGroup)}>
                <h2 {...stylex.props(s.cardTitle)}>{demo.title}</h2>
                <p {...stylex.props(s.cardSubtitle)}>{demo.subtitle}</p>
              </div>
              <span {...stylex.props(s.cardArrow)}>→</span>
            </div>
            <p {...stylex.props(s.cardDesc)}>{demo.description}</p>
          </a>
        ))}
      </div>
    </main>
  );
}

const s = stylex.create({
  page: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.xxl,
    alignItems: 'center',
    minHeight: '100vh',
    paddingInline: spacing.lg,
    paddingTop: spacing.xxxl,
    paddingBottom: spacing.xxxl,
    fontFamily: $.fontSans,
    color: `rgba(${$.foregroundR}, ${$.foregroundG}, ${$.foregroundB}, 1)`,
    backgroundColor: $.surfaceBg,
  },

  header: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.sm,
    alignItems: 'center',
    maxWidth: 640,
    textAlign: 'center',
  },
  backLink: {
    fontSize: text.sm,
    color: colors.accent,
    textDecoration: 'none',
    opacity: { default: 0.7, ':hover': 1 },
    transitionDuration: '200ms',
    transitionProperty: 'opacity',
  },
  title: {
    margin: 0,
    fontSize: { [MOBILE]: text.h2, default: text.h1 },
    fontWeight: 700,
    lineHeight: 1.1,
    letterSpacing: '-0.02em',
  },
  titleAccent: {
    color: colors.accent,
  },
  subtitle: {
    margin: 0,
    fontSize: text.p,
    lineHeight: 1.6,
    color: { [DARK]: colors.gray6, default: colors.gray5 },
    textWrap: 'balance',
  },
  grid: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.md,
    width: '100%',
    maxWidth: 640,
  },
  card: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacing.sm,
    paddingBlock: spacing.md,
    paddingInline: spacing.lg,
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
    boxShadow: {
      default: 'none',
      ':hover': '0 4px 16px rgba(0, 0, 0, 0.08)',
    },
    transform: {
      default: null,
      ':hover': 'translateY(-2px)',
    },
    transitionDuration: '200ms',
    transitionProperty: 'background-color, border-color, transform, box-shadow',
  },
  cardHeader: {
    display: 'flex',
    gap: spacing.md,
    alignItems: 'center',
  },
  cardIcon: {
    flexShrink: 0,
    fontSize: text.h4,
  },
  cardTitleGroup: {
    display: 'flex',
    flex: '1',
    flexDirection: 'column',
    gap: '0.125rem',
  },
  cardTitle: {
    margin: 0,
    fontSize: text.h5,
    fontWeight: 600,
  },
  cardSubtitle: {
    margin: 0,
    fontFamily: $.fontMono,
    fontSize: text.xs,
    opacity: 0.6,
  },
  cardArrow: {
    flexShrink: 0,
    marginLeft: 'auto',
    fontSize: text.h5,
    color: colors.accent,
    transitionDuration: '200ms',
    transitionProperty: 'transform',
  },
  cardDesc: {
    margin: 0,
    fontSize: text.sm,
    lineHeight: 1.6,
    opacity: 0.7,
  },
});
