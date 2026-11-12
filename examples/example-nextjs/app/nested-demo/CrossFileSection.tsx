/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * Cross-file demo: imports nested tokens from tokens.stylex.ts and
 * uses them in a separate stylex.create() call. This exercises the
 * cross-file resolution path for unstable_defineVarsNested.
 */

import * as stylex from '@stylexjs/stylex';
import { tokens, consts } from './tokens.stylex';

export default function CrossFileSection() {
  return (
    <section {...stylex.props(s.section)}>
      <p {...stylex.props(s.description)}>
        Cross-file import works: You can define your nested tokens in{' '}
        <code {...stylex.props(s.code)}>tokens.stylex.ts</code> and import{' '}
        <code {...stylex.props(s.code)}>tokens</code> /{' '}
        <code {...stylex.props(s.code)}>consts</code> into any other file. The
        nested member expressions like{' '}
        <code {...stylex.props(s.code)}>tokens.badge.info.bg</code> resolve
        across file boundaries.
      </p>
      <div {...stylex.props(s.grid)}>
        <div {...stylex.props(s.card)}>
          <p {...stylex.props(s.cardTitle)}>Badge swatches</p>
          <div {...stylex.props(s.swatchRow)}>
            <div {...stylex.props(s.swatch, s.infoSwatch)} title="info" />
            <div {...stylex.props(s.swatch, s.successSwatch)} title="success" />
            <div {...stylex.props(s.swatch, s.warningSwatch)} title="warning" />
            <div {...stylex.props(s.swatch, s.errorSwatch)} title="error" />
          </div>
          <p {...stylex.props(s.footer)}>
            <code {...stylex.props(s.code)}>tokens.badge.*.bg</code>
          </p>
        </div>
        <div {...stylex.props(s.card)}>
          <p {...stylex.props(s.cardTitle)}>Resolved tokens</p>
          <div {...stylex.props(s.tokenList)}>
            <div {...stylex.props(s.tokenRow)}>
              <span {...stylex.props(s.tokenDot, s.accentDot)} />
              accent.base
            </div>
            <div {...stylex.props(s.tokenRow)}>
              <span {...stylex.props(s.tokenDot, s.surfaceDot)} />
              surface.card
            </div>
            <div {...stylex.props(s.tokenRow)}>
              <span {...stylex.props(s.tokenDot, s.hoverDot)} />
              surface.hover
            </div>
          </div>
          <p {...stylex.props(s.footer)}>
            <code {...stylex.props(s.code)}>tokens.surface.*</code>
          </p>
        </div>
      </div>
    </section>
  );
}

const DARK = '@media (prefers-color-scheme: dark)';

const s = stylex.create({
  section: {
    display: 'flex',
    flexDirection: 'column',
    gap: '1rem',
    width: '100%',
    maxWidth: 640,
  },
  description: {
    margin: 0,
    fontFamily: consts.font.sans,
    fontSize: '0.875rem',
    lineHeight: 1.6,
    color: tokens.text.secondary,
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
    gap: '0.75rem',
  },
  card: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.75rem',
    padding: '1rem',
    backgroundColor: tokens.surface.card,
    borderRadius: consts.radius.md,
    boxShadow: tokens.surface.cardShadow,
  },
  cardTitle: {
    margin: 0,
    fontFamily: consts.font.sans,
    fontSize: '0.8rem',
    fontWeight: 600,
    color: tokens.text.muted,
    textTransform: 'uppercase',
    letterSpacing: '0.04em',
  },
  swatchRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '0.375rem',
  },
  swatch: {
    width: 36,
    height: 36,
    borderStyle: 'solid',
    borderWidth: 2,
    borderRadius: consts.radius.pill,
    boxShadow: {
      default: 'none',
      ':hover': '0 2px 8px rgba(0,0,0,0.15)',
    },
    transform: {
      default: null,
      ':hover': 'scale(1.15)',
    },
    transitionDuration: '200ms',
    transitionProperty: 'transform, box-shadow',
  },
  infoSwatch: {
    backgroundColor: tokens.badge.info.bg,
    borderColor: tokens.badge.info.border,
  },
  successSwatch: {
    backgroundColor: tokens.badge.success.bg,
    borderColor: tokens.badge.success.border,
  },
  warningSwatch: {
    backgroundColor: tokens.badge.warning.bg,
    borderColor: tokens.badge.warning.border,
  },
  errorSwatch: {
    backgroundColor: tokens.badge.error.bg,
    borderColor: tokens.badge.error.border,
  },
  tokenList: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.375rem',
  },
  tokenRow: {
    display: 'flex',
    gap: '0.5rem',
    alignItems: 'center',
    fontFamily: consts.font.mono,
    fontSize: '0.8rem',
    color: tokens.text.secondary,
  },
  tokenDot: {
    flexShrink: 0,
    width: 12,
    height: 12,
    borderRadius: '50%',
  },
  accentDot: {
    backgroundColor: tokens.accent.base,
  },
  surfaceDot: {
    backgroundColor: tokens.surface.card,
    borderColor: tokens.surface.divider,
    borderStyle: 'solid',
    borderWidth: 1,
  },
  hoverDot: {
    backgroundColor: tokens.surface.hover,
    borderColor: tokens.surface.divider,
    borderStyle: 'solid',
    borderWidth: 1,
  },
  code: {
    paddingBlock: '0.15em',
    paddingInline: '0.35em',
    fontFamily: consts.font.mono,
    fontSize: '0.8em',
    backgroundColor: {
      [DARK]: 'rgba(255,255,255,0.08)',
      default: 'rgba(0,0,0,0.04)',
    },
    borderRadius: '4px',
  },
  footer: {
    margin: 0,
    fontFamily: consts.font.sans,
    fontSize: '0.75rem',
    color: tokens.text.muted,
    textAlign: 'center',
  },
});
