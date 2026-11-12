/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * Dedicated demo route for the nested design-token APIs.
 *   - unstable_defineVarsNested  (tokens.stylex.ts)
 *   - unstable_defineConstsNested (tokens.stylex.ts)
 *   - unstable_createThemeNested  (tokens.stylex.ts)
 */

'use client';

import { useState } from 'react';
import * as stylex from '@stylexjs/stylex';
import { tokens, consts, sunsetTheme, forestTheme } from './tokens.stylex';
import CrossFileSection from './CrossFileSection';

const DARK = '@media (prefers-color-scheme: dark)';
const MOBILE = '@media (max-width: 700px)';

const THEMES = [
  { name: '🌊 Ocean', theme: null },
  { name: '🌅 Sunset', theme: sunsetTheme },
  { name: '🌲 Forest', theme: forestTheme },
] as const;

export default function NestedDemoPage() {
  const [themeIndex, setThemeIndex] = useState(0);

  return (
    <main {...stylex.props(s.main, THEMES[themeIndex].theme)}>
      {/* ── Hero ────────────────────────────────────── */}
      <div {...stylex.props(s.hero)}>
        <a {...stylex.props(s.backLink)} href="/theming-demos">
          ← Back to Theming Demos
        </a>
        <h1 {...stylex.props(s.h1)}>
          Nested <span {...stylex.props(s.plus)}>APIs</span> Demo
        </h1>
        <p {...stylex.props(s.subtitle)}>
          Experimental APIs for nested design tokens in StyleX. Define design
          tokens as nested objects instead of flat key-value pairs — the
          compiler flattens them into the same CSS with zero performance cost.
          This lets you compose hierarchical token structures and themes across
          packages.
        </p>
      </div>

      <div {...stylex.props(s.apiGrid)}>
        {[
          {
            api: 'unstable_defineVarsNested',
            desc: 'Define nested CSS custom properties that can be themed at runtime.',
            file: 'tokens.stylex.ts',
          },
          {
            api: 'unstable_defineConstsNested',
            desc: 'Define nested compile-time constants that are inlined — zero runtime cost.',
            file: 'tokens.stylex.ts',
          },
          {
            api: 'unstable_createThemeNested',
            desc: 'Override nested token values for a subtree, just like createTheme.',
            file: 'tokens.stylex.ts',
          },
          {
            api: 'unstable_conditional',
            desc: 'Wrapper for conditional CSS at-rules (@media, @supports) that disambiguates from namespace keys.',
            file: 'tokens.stylex.ts',
          },
        ].map((item) => (
          <div key={item.api} {...stylex.props(s.apiCard)}>
            <code {...stylex.props(s.apiName)}>{item.api}</code>
            <p {...stylex.props(s.apiDesc)}>{item.desc}</p>
            <p {...stylex.props(s.apiFile)}>
              <code {...stylex.props(s.code)}>{item.file}</code>
            </p>
          </div>
        ))}
      </div>

      {/* ── Theme Picker ───────────────────────────── */}
      <div {...stylex.props(s.themePicker)}>
        <span {...stylex.props(s.themeLabel)}>Theme</span>
        <div {...stylex.props(s.themeRow)}>
          {THEMES.map((t, i) => (
            <button
              key={t.name}
              {...stylex.props(
                s.themeButton,
                i === themeIndex && s.themeButtonActive,
              )}
              onClick={() => setThemeIndex(i)}
            >
              {t.name}
            </button>
          ))}
        </div>
      </div>

      {/* ── Badges (same-file consumption) ─────────── */}
      <section {...stylex.props(s.section)}>
        <h2 {...stylex.props(s.sectionTitle)}>
          <span {...stylex.props(s.sectionIcon)}>🏷️</span>
          Themed Badges
        </h2>
        <p {...stylex.props(s.sectionDesc)}>
          Colors from <code {...stylex.props(s.code)}>tokens.badge.*</code>,
          sizing from <code {...stylex.props(s.code)}>consts.badge.*</code>
        </p>
        <div {...stylex.props(s.badgeRow)}>
          <span {...stylex.props(s.badgeBase, s.badgeInfo)}>Info</span>
          <span {...stylex.props(s.badgeBase, s.badgeSuccess)}>Success</span>
          <span {...stylex.props(s.badgeBase, s.badgeWarning)}>Warning</span>
          <span {...stylex.props(s.badgeBase, s.badgeError)}>Error</span>
        </div>
      </section>

      {/* ── Surface Card (same-file consumption) ───── */}
      <section {...stylex.props(s.section)}>
        <h2 {...stylex.props(s.sectionTitle)}>
          <span {...stylex.props(s.sectionIcon)}>🎨</span>
          Surface Tokens
        </h2>
        <p {...stylex.props(s.sectionDesc)}>
          Card background, shadow, and hover states from{' '}
          <code {...stylex.props(s.code)}>tokens.surface.*</code>
        </p>
        <div {...stylex.props(s.cardGrid)}>
          <div {...stylex.props(s.demoCard)}>
            <p {...stylex.props(s.cardLabel)}>surface.card</p>
            <p {...stylex.props(s.cardBody)}>
              Background and shadow adapt to the selected theme.
            </p>
          </div>
          <div {...stylex.props(s.demoCard, s.demoCardHover)}>
            <p {...stylex.props(s.cardLabel)}>surface.hover</p>
            <p {...stylex.props(s.cardBody)}>
              Hover tint applied as a background color.
            </p>
          </div>
          <div {...stylex.props(s.demoCard, s.demoCardAccent)}>
            <p {...stylex.props(s.cardLabel)}>accent.*</p>
            <p {...stylex.props(s.cardBody)}>
              Border and label colored with nested accent tokens.
            </p>
          </div>
        </div>
      </section>

      {/* ── unstable_conditional (cond) ────────────── */}
      <section {...stylex.props(s.section)}>
        <div {...stylex.props(s.condHighlight)}>
          <p {...stylex.props(s.condHighlightText)}>
            💡 Use the{' '}
            <code {...stylex.props(s.code)}>unstable_conditional()</code>{' '}
            wrapper for conditional CSS at-rules (
            <code {...stylex.props(s.code)}>@media</code>,{' '}
            <code {...stylex.props(s.code)}>@supports</code>) for type safety.
          </p>
          <pre {...stylex.props(s.condPre)}>
            {`import { unstable_conditional as cond }
  from '@stylexjs/stylex';

const DARK = '@media (prefers-color-scheme: dark)';

accent: cond({
  default: '#2563eb',  // base value
  [DARK]: '#60a5fa',   // dark mode override
})`}
          </pre>
          <p {...stylex.props(s.condNote)}>
            With cond() compiler knows `default` is a conditional rule not a
            namespace
          </p>
        </div>
      </section>

      <CrossFileSection />

      <a {...stylex.props(s.demoLink)} href="/ds-demo">
        <span {...stylex.props(s.demoLinkIcon)}>🎨</span>
        <span>
          <strong>Design System Theming</strong>
          <br />
          <span {...stylex.props(s.demoLinkSub)}>
            Expressing the Primitives → Semantics → Themes three-tier design
            system architecture using the nested APIs
          </span>
        </span>
        <span {...stylex.props(s.demoLinkArrow)}>→</span>
      </a>
    </main>
  );
}

// ─── Styles (same-file nested token consumption) ─────────────
const s = stylex.create({
  main: {
    display: 'flex',
    flexDirection: 'column',
    gap: '2.5rem',
    alignItems: 'center',
    minHeight: '100vh',
    paddingInline: '1.5rem',
    paddingTop: '4rem',
    paddingBottom: '4rem',
    fontFamily: consts.font.sans,
    color: tokens.text.primary,
    backgroundColor: tokens.surface.bg,
    transitionDuration: '300ms',
    transitionProperty: 'background-color, color',
  },

  // Hero
  hero: {
    display: 'flex',
    flexDirection: 'column',
    gap: '1rem',
    alignItems: 'center',
    maxWidth: 640,
    textAlign: 'center',
  },
  backLink: {
    fontFamily: consts.font.sans,
    fontSize: '0.875rem',
    color: tokens.accent.base,
    textDecoration: 'none',
    opacity: {
      default: 0.7,
      ':hover': 1,
    },
    transitionDuration: '200ms',
    transitionProperty: 'opacity',
  },
  h1: {
    margin: 0,
    fontSize: {
      [MOBILE]: '2rem',
      default: '2.75rem',
    },
    fontWeight: 700,
    lineHeight: 1,
    letterSpacing: '-0.02em',
  },
  plus: {
    fontWeight: 300,
    color: tokens.accent.base,
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },
  subtitle: {
    maxWidth: '42ch',
    margin: 0,
    fontSize: '1rem',
    lineHeight: 1.6,
    color: tokens.text.secondary,
    textWrap: 'balance',
  },

  // Theme Picker
  themePicker: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.75rem',
    alignItems: 'center',
  },
  themeLabel: {
    fontSize: '0.75rem',
    fontWeight: 700,
    color: tokens.accent.base,
    textTransform: 'uppercase',
    letterSpacing: '0.05em',
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },
  themeRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '0.5rem',
    justifyContent: 'center',
  },
  themeButton: {
    paddingBlock: '0.5rem',
    paddingInline: '1rem',
    fontFamily: consts.font.sans,
    fontSize: '0.875rem',
    color: tokens.text.primary,
    cursor: 'pointer',
    backgroundColor: {
      default: 'transparent',
      ':hover': tokens.surface.hover,
    },
    borderColor: tokens.surface.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: consts.radius.sm,
    transitionDuration: '200ms',
    transitionProperty: 'background-color, border-color',
  },
  themeButtonActive: {
    fontWeight: 600,
    color: tokens.accent.base,
    backgroundColor: tokens.accent.light,
    borderColor: tokens.accent.base,
  },

  // Design System demo link card
  demoLink: {
    display: 'flex',
    gap: '1rem',
    alignItems: 'center',
    width: '100%',
    maxWidth: 640,
    paddingBlock: '1rem',
    paddingInline: '1.5rem',
    fontFamily: consts.font.sans,
    fontSize: '0.875rem',
    lineHeight: 1.4,
    color: 'inherit',
    textDecoration: 'none',
    backgroundColor: {
      default: tokens.accent.light,
      ':hover': tokens.accent.faded,
    },
    borderColor: tokens.accent.base,
    borderStyle: 'solid',
    borderWidth: 2,
    borderRadius: consts.radius.lg,
    transform: {
      default: null,
      ':hover': 'translateY(-2px)',
    },
    transitionDuration: '200ms',
    transitionProperty: 'background-color, border-color, transform',
  },
  demoLinkIcon: {
    flexShrink: 0,
    fontSize: '1.5rem',
  },
  demoLinkSub: {
    fontSize: '0.75rem',
    opacity: 0.6,
  },
  demoLinkArrow: {
    marginLeft: 'auto',
    fontSize: '1.25rem',
    color: tokens.accent.base,
    transitionDuration: '200ms',
    transitionProperty: 'transform',
  },

  // Section
  section: {
    display: 'flex',
    flexDirection: 'column',
    gap: '1rem',
    alignItems: 'center',
    width: '100%',
    maxWidth: 640,
  },
  sectionTitle: {
    display: 'flex',
    gap: '0.5rem',
    alignItems: 'center',
    margin: 0,
    fontSize: '1.25rem',
    fontWeight: 600,
  },
  sectionIcon: {
    fontSize: '1.25rem',
  },
  sectionDesc: {
    margin: 0,
    fontSize: '0.875rem',
    lineHeight: 1.6,
    color: tokens.text.secondary,
    textAlign: 'center',
  },

  // Badges
  badgeRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '0.5rem',
    justifyContent: 'center',
  },
  badgeBase: {
    paddingBlock: consts.badge.paddingBlock,
    paddingInline: consts.badge.paddingInline,
    fontSize: consts.badge.fontSize,
    fontWeight: consts.badge.fontWeight,
    textTransform: 'uppercase',
    letterSpacing: consts.badge.letterSpacing,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: consts.radius.pill,
  },
  badgeInfo: {
    color: tokens.badge.info.text,
    backgroundColor: tokens.badge.info.bg,
    borderColor: tokens.badge.info.border,
  },
  badgeSuccess: {
    color: tokens.badge.success.text,
    backgroundColor: tokens.badge.success.bg,
    borderColor: tokens.badge.success.border,
  },
  badgeWarning: {
    color: tokens.badge.warning.text,
    backgroundColor: tokens.badge.warning.bg,
    borderColor: tokens.badge.warning.border,
  },
  badgeError: {
    color: tokens.badge.error.text,
    backgroundColor: tokens.badge.error.bg,
    borderColor: tokens.badge.error.border,
  },

  // Demo cards
  cardGrid: {
    display: 'grid',
    gridTemplateColumns: {
      [MOBILE]: '1fr',
      default: 'repeat(3, 1fr)',
    },
    gap: '0.75rem',
    width: '100%',
  },
  demoCard: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.5rem',
    padding: '1.25rem',
    backgroundColor: tokens.surface.card,
    borderRadius: consts.radius.lg,
    boxShadow: tokens.surface.cardShadow,
    transitionDuration: '300ms',
    transitionProperty: 'background-color, box-shadow',
  },
  demoCardHover: {
    backgroundColor: tokens.surface.hover,
  },
  demoCardAccent: {
    borderColor: tokens.accent.border,
    borderStyle: 'solid',
    borderWidth: 2,
  },
  cardLabel: {
    margin: 0,
    fontSize: '0.75rem',
    fontWeight: 600,
    color: tokens.accent.base,
    textTransform: 'uppercase',
    letterSpacing: '0.04em',
  },
  cardBody: {
    margin: 0,
    fontSize: '0.8rem',
    lineHeight: 1.5,
    color: tokens.text.secondary,
  },

  // API reference cards
  apiGrid: {
    display: 'grid',
    gridTemplateColumns: {
      [MOBILE]: '1fr',
      default: 'repeat(4, 1fr)',
    },
    gap: '0.75rem',
    width: '100%',
    maxWidth: 840,
  },
  apiCard: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.5rem',
    padding: '1.25rem',
    backgroundColor: tokens.surface.card,
    borderColor: tokens.surface.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: consts.radius.lg,
    boxShadow: tokens.surface.cardShadow,
    transitionDuration: '300ms',
    transitionProperty: 'border-color, box-shadow',
  },
  apiName: {
    fontFamily: consts.font.mono,
    fontSize: '0.8rem',
    fontWeight: 600,
    color: tokens.accent.base,
  },
  apiDesc: {
    margin: 0,
    fontSize: '0.8rem',
    lineHeight: 1.5,
    color: tokens.text.secondary,
  },
  apiFile: {
    margin: 0,
    marginTop: 'auto',
    fontSize: '0.7rem',
    color: tokens.text.muted,
  },
  condPre: {
    padding: '0.75rem',
    margin: 0,
    overflow: 'auto',
    fontFamily: consts.font.mono,
    fontSize: '0.72rem',
    lineHeight: 1.5,
    color: tokens.text.primary,
    backgroundColor: {
      [DARK]: 'rgba(255,255,255,0.05)',
      default: 'rgba(0,0,0,0.03)',
    },
    borderRadius: consts.radius.sm,
  },
  condNote: {
    margin: 0,
    fontSize: '0.75rem',
    fontStyle: 'italic',
    lineHeight: 1.5,
    color: tokens.text.secondary,
  },
  condHighlight: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.75rem',
    width: '100%',
    padding: '1.25rem',
    borderColor: tokens.accent.base,
    borderStyle: 'solid',
    borderWidth: 2,
    borderRadius: consts.radius.lg,
    transitionDuration: '300ms',
    transitionProperty: 'border-color',
  },
  condHighlightText: {
    margin: 0,
    fontSize: '0.875rem',
    fontWeight: 600,
    lineHeight: 1.6,
    color: tokens.accent.base,
    transitionDuration: '300ms',
    transitionProperty: 'color',
  },

  // Shared
  code: {
    paddingBlock: '0.1em',
    paddingInline: '0.3em',
    fontFamily: consts.font.mono,
    fontSize: '0.85em',
    backgroundColor: {
      [DARK]: 'rgba(255,255,255,0.08)',
      default: 'rgba(0,0,0,0.04)',
    },
    borderRadius: '4px',
  },
});
