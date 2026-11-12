/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * XDS-style demo: shows the three-tier token architecture
 * (Primitives → Semantics → Themes) using the nested APIs.
 */

'use client';

import { useState } from 'react';
import * as stylex from '@stylexjs/stylex';
import {
  primitives,
  tokens,
  purpleTheme,
  greenTheme,
  orangeTheme,
} from './tokens.stylex';

const DARK = '@media (prefers-color-scheme: dark)';
const MOBILE = '@media (max-width: 700px)';

const THEMES = [
  { name: 'Default', theme: null, accent: '#2563eb' },
  { name: 'Purple', theme: purpleTheme, accent: '#9333ea' },
  { name: 'Green', theme: greenTheme, accent: '#16a34a' },
  { name: 'Orange', theme: orangeTheme, accent: '#ea580c' },
] as const;

export default function XdsDemoPage() {
  const [themeIndex, setThemeIndex] = useState(0);

  return (
    <main {...stylex.props(s.page, THEMES[themeIndex].theme)}>
      {/* ── Header ──────────────────────────────────── */}
      <header {...stylex.props(s.header)}>
        <a {...stylex.props(s.backLink)} href="/theming-demos">
          ← Back to Theming Demos
        </a>
        <h1 {...stylex.props(s.title)}>
          Design System <span {...stylex.props(s.titleAccent)}>Theming</span>{' '}
          Demo
        </h1>
        <p {...stylex.props(s.subtitle)}>
          A showcase of three-tier token architecture (Primitives → Semantics →
          Themes) written with{' '}
          <a {...stylex.props(s.inlineLink)} href="/nested-demo">
            the nested APIs
          </a>
          .
        </p>
        <p {...stylex.props(s.subtitle)}>
          Express three-tier token architecture natively in StyleX. Primitives
          (defineConstsNested) feed into semantic tokens (defineVarsNested)
          which are overridden by themes (createThemeNested). Switch themes to
          re-skin everything.
        </p>
      </header>

      {/* ── Theme Picker ───────────────────────────── */}
      <nav {...stylex.props(s.picker)}>
        <span {...stylex.props(s.pickerLabel)}>Theme</span>
        <div {...stylex.props(s.pickerRow)}>
          {THEMES.map((t, i) => (
            <button
              key={t.name}
              {...stylex.props(
                s.pickerBtn,
                i === themeIndex && s.pickerBtnActive,
              )}
              onClick={() => setThemeIndex(i)}
            >
              {t.name}
            </button>
          ))}
        </div>
      </nav>

      {/* ── Architecture Diagram ───────────────────── */}
      <section {...stylex.props(s.section)}>
        <h2 {...stylex.props(s.sectionTitle)}>How It Works</h2>
        <div {...stylex.props(s.tierRow)}>
          <div {...stylex.props(s.tierCard)}>
            <div {...stylex.props(s.tierBadge, s.tierBadge1)}>Tier 1</div>
            <h3 {...stylex.props(s.tierName)}>Primitives</h3>
            <code {...stylex.props(s.tierApi)}>defineConstsNested</code>
            <p {...stylex.props(s.tierDesc)}>
              Raw palette values inlined at compile time. Zero runtime cost.
            </p>
            <div {...stylex.props(s.tierExample)}>
              <code {...stylex.props(s.code)}>primitives.color.blue[600]</code>
            </div>
          </div>
          <div {...stylex.props(s.tierArrow)}>→</div>
          <div {...stylex.props(s.tierCard)}>
            <div {...stylex.props(s.tierBadge, s.tierBadge2)}>Tier 2</div>
            <h3 {...stylex.props(s.tierName)}>Semantic Tokens</h3>
            <code {...stylex.props(s.tierApi)}>defineVarsNested</code>
            <p {...stylex.props(s.tierDesc)}>
              Intent-based CSS variables that components reference. Themeable.
            </p>
            <div {...stylex.props(s.tierExample)}>
              <code {...stylex.props(s.code)}>tokens.color.accent</code>
            </div>
          </div>
          <div {...stylex.props(s.tierArrow)}>→</div>
          <div {...stylex.props(s.tierCard)}>
            <div {...stylex.props(s.tierBadge, s.tierBadge3)}>Tier 3</div>
            <h3 {...stylex.props(s.tierName)}>Themes</h3>
            <code {...stylex.props(s.tierApi)}>createThemeNested</code>
            <p {...stylex.props(s.tierDesc)}>
              Override any semantic token. Partial overrides supported.
            </p>
            <div {...stylex.props(s.tierExample)}>
              <code {...stylex.props(s.code)}>purpleTheme, greenTheme…</code>
            </div>
          </div>
        </div>
      </section>

      {/* ── Component Showcase ─────────────────────── */}
      <section {...stylex.props(s.section)}>
        <h2 {...stylex.props(s.sectionTitle)}>Components</h2>
        <p {...stylex.props(s.sectionDesc)}>
          Every component below references only{' '}
          <code {...stylex.props(s.code)}>tokens.*</code> — no hard-coded
          colors. Switching themes re-skins everything instantly.
        </p>

        <div {...stylex.props(s.componentGrid)}>
          {/* Buttons */}
          <div {...stylex.props(s.demoCard)}>
            <p {...stylex.props(s.demoLabel)}>Buttons</p>
            <div {...stylex.props(s.demoRow)}>
              <button {...stylex.props(s.btnPrimary)}>Primary</button>
              <button {...stylex.props(s.btnSecondary)}>Secondary</button>
              <button {...stylex.props(s.btnGhost)}>Ghost</button>
            </div>
          </div>

          {/* Status badges */}
          <div {...stylex.props(s.demoCard)}>
            <p {...stylex.props(s.demoLabel)}>Status Badges</p>
            <div {...stylex.props(s.demoRow)}>
              <span {...stylex.props(s.badge, s.badgeSuccess)}>Success</span>
              <span {...stylex.props(s.badge, s.badgeWarning)}>Warning</span>
              <span {...stylex.props(s.badge, s.badgeError)}>Error</span>
              <span {...stylex.props(s.badge, s.badgeAccent)}>Accent</span>
            </div>
          </div>

          {/* Card */}
          <div {...stylex.props(s.demoCard)}>
            <p {...stylex.props(s.demoLabel)}>Surface Card</p>
            <div {...stylex.props(s.innerCard)}>
              <p {...stylex.props(s.innerCardTitle)}>Nested surface</p>
              <p {...stylex.props(s.innerCardBody)}>
                Background, shadow, and divider colors all come from{' '}
                <code {...stylex.props(s.code)}>tokens.color.surface</code> and{' '}
                <code {...stylex.props(s.code)}>tokens.elevation.*</code>.
              </p>
            </div>
          </div>

          {/* Input */}
          <div {...stylex.props(s.demoCard)}>
            <p {...stylex.props(s.demoLabel)}>Form Input</p>
            <div {...stylex.props(s.inputWrapper)}>
              <input
                {...stylex.props(s.input)}
                placeholder="Type something…"
                readOnly
              />
              <button {...stylex.props(s.btnPrimary, s.inputBtn)}>
                Submit
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* ── Token Reference ────────────────────────── */}
      <section {...stylex.props(s.section)}>
        <h2 {...stylex.props(s.sectionTitle)}>Live Token Values</h2>
        <p {...stylex.props(s.sectionDesc)}>
          These swatches reflect the current theme's semantic tokens in real
          time.
        </p>
        <div {...stylex.props(s.swatchGrid)}>
          {[
            { label: 'color.accent', style: s.swAccent },
            { label: 'color.accentHover', style: s.swAccentHover },
            { label: 'color.accentSoft', style: s.swAccentSoft },
            { label: 'color.success', style: s.swSuccess },
            { label: 'color.warning', style: s.swWarning },
            { label: 'color.error', style: s.swError },
            { label: 'color.surface', style: s.swSurface },
            { label: 'color.surfaceHover', style: s.swSurfaceHover },
            { label: 'color.divider', style: s.swDivider },
          ].map((item) => (
            <div key={item.label} {...stylex.props(s.swatchItem)}>
              <div {...stylex.props(s.swatch, item.style)} />
              <code {...stylex.props(s.swatchCode)}>{item.label}</code>
            </div>
          ))}
        </div>
      </section>

      {/* ── Footer ─────────────────────────────────── */}
      <footer {...stylex.props(s.footer)}>
        Source: <code {...stylex.props(s.code)}>tokens.stylex.ts</code>
      </footer>
    </main>
  );
}

// ─── Styles ──────────────────────────────────────────────────
const s = stylex.create({
  // Layout
  page: {
    display: 'flex',
    flexDirection: 'column',
    gap: '2.5rem',
    alignItems: 'center',
    minHeight: '100vh',
    paddingInline: '1.5rem',
    paddingTop: '3rem',
    paddingBottom: '4rem',
    fontFamily: primitives.font.sans,
    color: tokens.color.textPrimary,
    backgroundColor: tokens.color.bg,
    transitionDuration: '250ms',
    transitionProperty: 'background-color, color',
  },

  // Header
  header: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.75rem',
    alignItems: 'center',
    textAlign: 'center',
  },
  backLink: {
    fontSize: '0.8rem',
    color: tokens.color.textMuted,
    textDecoration: 'none',
    opacity: { default: 0.8, ':hover': 1 },
  },
  title: {
    margin: 0,
    fontSize: { [MOBILE]: '1.75rem', default: '2.5rem' },
    fontWeight: primitives.fontWeight.bold,
    lineHeight: 1.1,
    letterSpacing: '-0.02em',
  },
  titleAccent: {
    color: tokens.color.accent,
    transitionDuration: '250ms',
    transitionProperty: 'color',
  },
  subtitle: {
    maxWidth: '50ch',
    margin: 0,
    fontSize: '1rem',
    color: tokens.color.textSecondary,
    textWrap: 'balance',
  },
  inlineLink: {
    fontWeight: primitives.fontWeight.medium,
    color: tokens.color.accent,
    textDecoration: 'none',
    opacity: { default: 0.9, ':hover': 1 },
    transitionDuration: '200ms',
    transitionProperty: 'opacity',
  },

  // Theme picker
  picker: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.5rem',
    alignItems: 'center',
  },
  pickerLabel: {
    fontSize: '0.7rem',
    fontWeight: primitives.fontWeight.bold,
    color: tokens.color.accent,
    textTransform: 'uppercase',
    letterSpacing: '0.06em',
    transitionDuration: '250ms',
    transitionProperty: 'color',
  },
  pickerRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '0.375rem',
    justifyContent: 'center',
  },
  pickerBtn: {
    paddingBlock: '0.4rem',
    paddingInline: '0.875rem',
    fontFamily: primitives.font.sans,
    fontSize: '0.8rem',
    color: tokens.color.textPrimary,
    cursor: 'pointer',
    backgroundColor: {
      default: 'transparent',
      ':hover': tokens.color.surfaceHover,
    },
    borderColor: tokens.color.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.md,
    transitionDuration: '200ms',
    transitionProperty: 'all',
  },
  pickerBtnActive: {
    fontWeight: primitives.fontWeight.semibold,
    color: tokens.color.accent,
    backgroundColor: tokens.color.accentSoft,
    borderColor: tokens.color.accent,
  },

  // Sections
  section: {
    display: 'flex',
    flexDirection: 'column',
    gap: '1rem',
    alignItems: 'center',
    width: '100%',
    maxWidth: 720,
  },
  sectionTitle: {
    margin: 0,
    fontSize: '1.25rem',
    fontWeight: primitives.fontWeight.semibold,
  },
  sectionDesc: {
    maxWidth: '50ch',
    margin: 0,
    fontSize: '0.875rem',
    lineHeight: 1.6,
    color: tokens.color.textSecondary,
    textAlign: 'center',
  },

  // Tier diagram
  tierRow: {
    display: 'flex',
    flexDirection: { [MOBILE]: 'column', default: 'row' },
    gap: '0.75rem',
    alignItems: 'stretch',
    width: '100%',
  },
  tierArrow: {
    display: { [MOBILE]: 'none', default: 'flex' },
    alignItems: 'center',
    fontSize: '1.25rem',
    color: tokens.color.textMuted,
  },
  tierCard: {
    display: 'flex',
    flex: '1',
    flexDirection: 'column',
    gap: '0.5rem',
    padding: '1.25rem',
    backgroundColor: tokens.color.surface,
    borderColor: tokens.color.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.lg,
    boxShadow: tokens.elevation.sm,
    transitionDuration: '250ms',
    transitionProperty: 'background-color, border-color, box-shadow',
  },
  tierBadge: {
    alignSelf: 'flex-start',
    paddingBlock: '0.15rem',
    paddingInline: '0.5rem',
    fontSize: '0.65rem',
    fontWeight: primitives.fontWeight.bold,
    textTransform: 'uppercase',
    letterSpacing: '0.04em',
    borderRadius: primitives.radius.sm,
  },
  tierBadge1: {
    color: tokens.color.accent,
    backgroundColor: tokens.color.accentSoft,
  },
  tierBadge2: {
    color: tokens.color.accent,
    backgroundColor: tokens.color.accentSoft,
  },
  tierBadge3: {
    color: tokens.color.accent,
    backgroundColor: tokens.color.accentSoft,
  },
  tierName: {
    margin: 0,
    fontSize: '0.95rem',
    fontWeight: primitives.fontWeight.semibold,
  },
  tierApi: {
    fontFamily: primitives.font.mono,
    fontSize: '0.7rem',
    color: tokens.color.accent,
    transitionDuration: '250ms',
    transitionProperty: 'color',
  },
  tierDesc: {
    margin: 0,
    fontSize: '0.775rem',
    lineHeight: 1.5,
    color: tokens.color.textSecondary,
  },
  tierExample: {
    paddingTop: '0.5rem',
    marginTop: 'auto',
    borderTopColor: tokens.color.divider,
    borderTopStyle: 'solid',
    borderTopWidth: 1,
  },

  // Component showcase
  componentGrid: {
    display: 'grid',
    gridTemplateColumns: { [MOBILE]: '1fr', default: 'repeat(2, 1fr)' },
    gap: '0.75rem',
    width: '100%',
  },
  demoCard: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.75rem',
    padding: '1.25rem',
    backgroundColor: tokens.color.surface,
    borderColor: tokens.color.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.lg,
    boxShadow: tokens.elevation.sm,
    transitionDuration: '250ms',
    transitionProperty: 'background-color, border-color, box-shadow',
  },
  demoLabel: {
    margin: 0,
    fontSize: '0.7rem',
    fontWeight: primitives.fontWeight.bold,
    color: tokens.color.textMuted,
    textTransform: 'uppercase',
    letterSpacing: '0.04em',
  },
  demoRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '0.5rem',
  },

  // Buttons
  btnPrimary: {
    paddingBlock: '0.5rem',
    paddingInline: '1rem',
    fontFamily: primitives.font.sans,
    fontSize: '0.8rem',
    fontWeight: primitives.fontWeight.semibold,
    color: tokens.color.accentText,
    cursor: 'pointer',
    backgroundColor: tokens.color.accent,
    borderWidth: 0,
    borderRadius: primitives.radius.md,
    transitionDuration: '200ms',
    transitionProperty: 'background-color',
  },
  btnSecondary: {
    paddingBlock: '0.5rem',
    paddingInline: '1rem',
    fontFamily: primitives.font.sans,
    fontSize: '0.8rem',
    fontWeight: primitives.fontWeight.semibold,
    color: tokens.color.accent,
    cursor: 'pointer',
    backgroundColor: tokens.color.accentSoft,
    borderColor: tokens.color.accent,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.md,
    transitionDuration: '200ms',
    transitionProperty: 'all',
  },
  btnGhost: {
    paddingBlock: '0.5rem',
    paddingInline: '1rem',
    fontFamily: primitives.font.sans,
    fontSize: '0.8rem',
    fontWeight: primitives.fontWeight.medium,
    color: tokens.color.textPrimary,
    cursor: 'pointer',
    backgroundColor: {
      default: 'transparent',
      ':hover': tokens.color.surfaceHover,
    },
    borderColor: tokens.color.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.md,
    transitionDuration: '200ms',
    transitionProperty: 'all',
  },

  // Badges
  badge: {
    paddingBlock: '0.2rem',
    paddingInline: '0.625rem',
    fontSize: '0.7rem',
    fontWeight: primitives.fontWeight.semibold,
    textTransform: 'uppercase',
    letterSpacing: '0.02em',
    borderRadius: primitives.radius.full,
  },
  badgeSuccess: {
    color: tokens.color.success,
    backgroundColor: tokens.color.successSoft,
  },
  badgeWarning: {
    color: tokens.color.warning,
    backgroundColor: tokens.color.warningSoft,
  },
  badgeError: {
    color: tokens.color.error,
    backgroundColor: tokens.color.errorSoft,
  },
  badgeAccent: {
    color: tokens.color.accent,
    backgroundColor: tokens.color.accentSoft,
  },

  // Inner card
  innerCard: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.375rem',
    padding: '1rem',
    backgroundColor: tokens.color.surfaceHover,
    borderRadius: primitives.radius.md,
  },
  innerCardTitle: {
    margin: 0,
    fontSize: '0.8rem',
    fontWeight: primitives.fontWeight.semibold,
  },
  innerCardBody: {
    margin: 0,
    fontSize: '0.775rem',
    lineHeight: 1.5,
    color: tokens.color.textSecondary,
  },

  // Input
  inputWrapper: {
    display: 'flex',
    gap: '0.5rem',
  },
  input: {
    flex: '1',
    paddingBlock: '0.5rem',
    paddingInline: '0.75rem',
    fontFamily: primitives.font.sans,
    fontSize: '0.8rem',
    color: tokens.color.textPrimary,
    outline: 'none',
    backgroundColor: tokens.color.surface,
    borderColor: tokens.color.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.md,
    transitionDuration: '200ms',
    transitionProperty: 'border-color',
  },
  inputBtn: {
    flexShrink: 0,
  },

  // Swatches
  swatchGrid: {
    display: 'grid',
    gridTemplateColumns: {
      [MOBILE]: 'repeat(2, 1fr)',
      default: 'repeat(3, 1fr)',
    },
    gap: '0.75rem',
    width: '100%',
  },
  swatchItem: {
    display: 'flex',
    gap: '0.625rem',
    alignItems: 'center',
    padding: '0.625rem',
    backgroundColor: tokens.color.surface,
    borderColor: tokens.color.divider,
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.md,
    transitionDuration: '250ms',
    transitionProperty: 'background-color, border-color',
  },
  swatch: {
    flexShrink: 0,
    width: 28,
    height: 28,
    borderColor: 'rgba(0,0,0,0.06)',
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: primitives.radius.sm,
    transitionDuration: '250ms',
    transitionProperty: 'background-color',
  },
  swAccent: { backgroundColor: tokens.color.accent },
  swAccentHover: { backgroundColor: tokens.color.accentHover },
  swAccentSoft: { backgroundColor: tokens.color.accentSoft },
  swSuccess: { backgroundColor: tokens.color.success },
  swWarning: { backgroundColor: tokens.color.warning },
  swError: { backgroundColor: tokens.color.error },
  swSurface: { backgroundColor: tokens.color.surface },
  swSurfaceHover: { backgroundColor: tokens.color.surfaceHover },
  swDivider: { backgroundColor: tokens.color.divider },
  swatchCode: {
    fontFamily: primitives.font.mono,
    fontSize: '0.675rem',
    color: tokens.color.textSecondary,
  },

  // Shared
  code: {
    paddingBlock: '0.1em',
    paddingInline: '0.3em',
    fontFamily: primitives.font.mono,
    fontSize: '0.8em',
    backgroundColor: {
      [DARK]: 'rgba(255,255,255,0.08)',
      default: 'rgba(0,0,0,0.04)',
    },
    borderRadius: '4px',
  },

  // Footer
  footer: {
    fontSize: '0.75rem',
    color: tokens.color.textMuted,
    textAlign: 'center',
  },
});
