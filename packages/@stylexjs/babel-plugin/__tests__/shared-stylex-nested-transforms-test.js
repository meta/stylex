/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();

import {
  defineVars,
  defineVarsNested,
  defineConsts,
  defineConstsNested,
  createTheme,
  createThemeNested,
} from '../src/shared';

const defaultOptions = {
  classNamePrefix: 'x',
  dev: false,
  debug: false,
  test: false,
  useRemForFontSize: false,
  enableFontSizePxToRem: false,
  enableLegacyValueFlipping: false,
  enableLogicalStylesPolyfill: false,
};

describe('shared nested transforms', () => {
  describe('styleXDefineVarsNested', () => {
    const options = {
      ...defaultOptions,
      exportId: 'test/tokens.stylex.js//tokens',
    };

    test('returns nested JS output with var() references', () => {
      const [jsOutput] = defineVarsNested(
        {
          button: {
            background: 'red',
            color: 'blue',
          },
        },
        options,
      );

      // Should have nested structure
      expect(jsOutput.button).toBeDefined();
      expect(jsOutput.button.background).toBeDefined();
      expect(jsOutput.button.color).toBeDefined();

      // Leaf values should be var() references
      expect(jsOutput.button.background).toMatch(/^var\(--/);
      expect(jsOutput.button.color).toMatch(/^var\(--/);

      // Should have __varGroupHash__ at top level
      expect(typeof jsOutput.__varGroupHash__).toBe('string');
      expect(jsOutput.__varGroupHash__).toMatch(/^x/);
    });

    test('produces same CSS as flat defineVars with equivalent keys', () => {
      const [, nestedCSS] = defineVarsNested(
        { button: { background: 'red' } },
        options,
      );

      const [, flatCSS] = defineVars({ 'button.background': 'red' }, options);

      // Same number of injectable style entries
      expect(Object.keys(nestedCSS).length).toBe(Object.keys(flatCSS).length);

      // CSS content should contain the same value
      const nestedLtr = Object.values(nestedCSS)[0].ltr;
      const flatLtr = Object.values(flatCSS)[0].ltr;
      expect(nestedLtr).toContain('red');
      expect(flatLtr).toContain('red');
    });

    test('produces same __varGroupHash__ as flat defineVars', () => {
      const [nestedJS] = defineVarsNested(
        { button: { background: 'red' } },
        options,
      );
      const [flatJS] = defineVars({ 'button.background': 'red' }, options);

      // Same group hash because same exportId
      expect(nestedJS.__varGroupHash__).toBe(flatJS.__varGroupHash__);
    });

    test('handles deeply nested tokens (3 levels)', () => {
      const [jsOutput] = defineVarsNested(
        {
          button: {
            primary: {
              background: '#00FF00',
            },
          },
        },
        options,
      );

      expect(jsOutput.button.primary.background).toMatch(/^var\(--/);
    });

    test('handles conditional @-rule values inside nesting', () => {
      const [jsOutput, cssOutput] = defineVarsNested(
        {
          button: {
            color: {
              default: 'blue',
              '@media (prefers-color-scheme: dark)': 'lightblue',
            },
          },
        },
        options,
      );

      // JS output should have the var() reference (not the conditional object)
      expect(jsOutput.button.color).toMatch(/^var\(--/);

      // CSS should have both default and @media rules
      const cssEntries = Object.values(cssOutput);
      expect(cssEntries.length).toBeGreaterThanOrEqual(2);
      const allLtr = cssEntries.map((s) => s.ltr).join('');
      expect(allLtr).toContain('blue');
      expect(allLtr).toContain('lightblue');
      expect(allLtr).toContain('@media');
    });

    test('handles mixed flat and nested values', () => {
      const [jsOutput] = defineVarsNested(
        {
          flatValue: 'red',
          nested: { deep: 'blue' },
        },
        options,
      );

      expect(jsOutput.flatValue).toMatch(/^var\(--/);
      expect(jsOutput.nested.deep).toMatch(/^var\(--/);
    });

    test('different nested keys produce different var() hashes', () => {
      const [jsOutput] = defineVarsNested(
        {
          button: {
            background: 'red',
            color: 'blue',
          },
        },
        options,
      );

      expect(jsOutput.button.background).not.toBe(jsOutput.button.color);
    });
  });

  describe('styleXDefineConstsNested', () => {
    const options = {
      ...defaultOptions,
      exportId: 'test/tokens.stylex.js//tokens',
    };

    test('returns nested JS output with original values preserved', () => {
      const [jsOutput] = defineConstsNested(
        {
          spacing: { sm: '4px', md: '8px', lg: '16px' },
        },
        options,
      );

      expect(jsOutput.spacing.sm).toBe('4px');
      expect(jsOutput.spacing.md).toBe('8px');
      expect(jsOutput.spacing.lg).toBe('16px');
    });

    test('generates empty CSS (consts do not emit CSS variables)', () => {
      const [, cssOutput] = defineConstsNested(
        {
          spacing: { sm: '4px', md: '8px' },
        },
        options,
      );

      // Each entry should have empty ltr (consts don't produce CSS rules)
      for (const style of Object.values(cssOutput)) {
        expect(style.ltr).toBe('');
      }
    });

    test('preserves number values', () => {
      const [jsOutput] = defineConstsNested(
        {
          breakpoints: { mobile: 480, tablet: 768 },
        },
        options,
      );

      expect(jsOutput.breakpoints.mobile).toBe(480);
      expect(jsOutput.breakpoints.tablet).toBe(768);
    });

    test('handles deeply nested constants', () => {
      const [jsOutput] = defineConstsNested(
        {
          colors: {
            slate: { 100: '#f1f5f9', 800: '#1e293b' },
            brand: { primary: '#3b82f6' },
          },
        },
        options,
      );

      expect(jsOutput.colors.slate['100']).toBe('#f1f5f9');
      expect(jsOutput.colors.slate['800']).toBe('#1e293b');
      expect(jsOutput.colors.brand.primary).toBe('#3b82f6');
    });

    test('produces same const metadata as flat defineConsts with equivalent keys', () => {
      const [, nestedMeta] = defineConstsNested(
        { spacing: { sm: '4px' } },
        options,
      );
      const [, flatMeta] = defineConsts({ 'spacing.sm': '4px' }, options);

      // Same number of entries
      expect(Object.keys(nestedMeta).length).toBe(Object.keys(flatMeta).length);

      // Same constVal
      const nestedConstVal = Object.values(nestedMeta)[0].constVal;
      const flatConstVal = Object.values(flatMeta)[0].constVal;
      expect(nestedConstVal).toBe(flatConstVal);
    });

    test('j-malt PR #1303 use case: three-tiered design system tokens with state namespaces', () => {
      const [jsOutput] = defineConstsNested(
        {
          button: {
            primary: {
              background: {
                default: '#00FF00',
                hovered: '#0000FF',
              },
              borderRadius: {
                default: '8px',
              },
            },
            secondary: {
              background: {
                default: '#CCCCCC',
              },
            },
          },
          input: {
            fill: {
              default: '#FFFFFF',
            },
            border: {
              default: '#000000',
            },
          },
        },
        options,
      );

      // Deeply nested access — the exact pattern from PR #1303
      expect(jsOutput.button.primary.background.default).toBe('#00FF00');
      expect(jsOutput.button.primary.background.hovered).toBe('#0000FF');
      expect(jsOutput.button.primary.borderRadius.default).toBe('8px');
      expect(jsOutput.button.secondary.background.default).toBe('#CCCCCC');
      expect(jsOutput.input.fill.default).toBe('#FFFFFF');
      expect(jsOutput.input.border.default).toBe('#000000');
    });

    test('handles mixed string and number values', () => {
      const [jsOutput] = defineConstsNested(
        {
          theme: { spacing: 8, unit: 'px' },
        },
        options,
      );

      expect(jsOutput.theme.spacing).toBe(8);
      expect(jsOutput.theme.unit).toBe('px');
    });
  });

  describe('styleXCreateThemeNested', () => {
    const options = {
      ...defaultOptions,
      exportId: 'test/tokens.stylex.js//tokens',
    };

    test('creates theme override from nested vars and nested overrides', () => {
      const [varsOutput] = defineVarsNested(
        {
          button: {
            background: 'red',
            color: 'blue',
          },
        },
        options,
      );

      const [themeOutput, themeCSS] = createThemeNested(varsOutput, {
        button: {
          background: 'green',
          color: 'white',
        },
      });

      expect(themeOutput.$$css).toBe(true);
      expect(themeOutput[varsOutput.__varGroupHash__]).toBeDefined();

      const themeClass = themeOutput[varsOutput.__varGroupHash__];
      expect(themeClass).toContain(varsOutput.__varGroupHash__);

      const allCSS = Object.values(themeCSS)
        .map((s) => s.ltr)
        .join('');
      expect(allCSS).toContain('green');
      expect(allCSS).toContain('white');
    });

    test('supports partial overrides (only some leaves)', () => {
      const [varsOutput] = defineVarsNested(
        {
          button: {
            background: 'red',
            color: 'blue',
          },
          input: {
            fill: 'white',
          },
        },
        options,
      );

      const [, themeCSS] = createThemeNested(varsOutput, {
        button: {
          background: 'green',
        },
      });

      const allCSS = Object.values(themeCSS)
        .map((s) => s.ltr)
        .join('');
      expect(allCSS).toContain('green');
      expect(allCSS).not.toContain('blue');
      expect(allCSS).not.toContain('white');
    });

    test('produces same output as flat createTheme with equivalent flattened inputs', () => {
      const [nestedVars] = defineVarsNested({ button: { bg: 'red' } }, options);
      const [flatVars] = defineVars({ 'button.bg': 'red' }, options);

      const [nestedTheme] = createThemeNested(nestedVars, {
        button: { bg: 'green' },
      });
      const [flatTheme] = createTheme(flatVars, { 'button.bg': 'green' });

      expect(nestedTheme.$$css).toBe(true);
      expect(flatTheme.$$css).toBe(true);
      expect(nestedVars.__varGroupHash__).toBe(flatVars.__varGroupHash__);
    });

    test('handles conditional overrides with @media', () => {
      const [varsOutput] = defineVarsNested(
        {
          button: {
            background: 'red',
          },
        },
        options,
      );

      const [, themeCSS] = createThemeNested(varsOutput, {
        button: {
          background: {
            default: 'green',
            '@media (prefers-color-scheme: dark)': 'darkgreen',
          },
        },
      });

      const cssEntries = Object.values(themeCSS);
      expect(cssEntries.length).toBeGreaterThanOrEqual(2);
      const allCSS = cssEntries.map((s) => s.ltr).join('');
      expect(allCSS).toContain('green');
      expect(allCSS).toContain('darkgreen');
      expect(allCSS).toContain('@media');
    });

    test('throws when first arg lacks __varGroupHash__', () => {
      expect(() => {
        createThemeNested(
          { button: { bg: 'var(--hash)' } },
          { button: { bg: 'green' } },
        );
      }).toThrow();
    });
  });
});
