/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import stylexPlugin from '../src/index';
import { PreRule } from '../src/shared/preprocess-rules/PreRule';
import createTheme from '../src/shared/stylex-create-theme';
import { defaultOptions } from '../src/shared/utils/default-options';

const assemblyOptions = { useLayers: false, legacyDisableLayers: true };

function rule(name, ltr) {
  return [name, { ltr, rtl: null }, 3000];
}

describe('UTF-16 code-unit ordering', () => {
  test.each([
    [
      'legacy class name',
      rule('xB', '.xB{color:red}'),
      rule('xa', '.xa{color:blue}'),
      { useLegacyClassnamesSort: true },
    ],
    [
      'declared property name',
      rule('x1', '.x1{--B:red}'),
      rule('x2', '.x2{--a:red}'),
      {},
    ],
    [
      'surrounding at-rule context before breakpoint width',
      rule(
        'x1',
        '@container B (width > 10px){@media (min-width: 900px){.x1{color:red}}}',
      ),
      rule(
        'x2',
        '@container a (width > 10px){@media (min-width: 100px){.x2{color:red}}}',
      ),
      {},
    ],
    [
      'declaration text (case-sensitive)',
      rule('x1', '.x1::after{content:"B"}'),
      rule('x2', '.x2::after{content:"a"}'),
      {},
    ],
    [
      'declaration text (including surrogate pairs)',
      rule('x1', '.x1::after{content:"\u{1F600}"}'),
      rule('x2', '.x2::after{content:"\uFF21"}'),
      {},
    ],
    [
      'full rule text',
      rule('xB', '.xB{color:red}'),
      rule('xa', '.xa{color:red}'),
      {},
    ],
  ])('sorts by %s', (_name, first, second, overrides) => {
    const options = { ...assemblyOptions, ...overrides };
    const expected = `${first[1].ltr}\n${second[1].ltr}`;
    expect(stylexPlugin.processStylexRules([first, second], options)).toBe(
      expected,
    );
    expect(stylexPlugin.processStylexRules([second, first], options)).toBe(
      expected,
    );
  });

  test('canonicalizes pseudo-classes around a pseudo-element for the class hash', () => {
    const compile = (pseudos) =>
      new PreRule('color', 'red', pseudos).compiled(defaultOptions)[0];
    const [className, style] = compile([
      ':lang(a)',
      ':lang(B)',
      '::part(label)',
      ':hover',
      ':active',
    ]);
    const [permutedClass, permutedStyle] = compile([
      ':lang(B)',
      ':lang(a)',
      '::part(label)',
      ':active',
      ':hover',
    ]);
    expect(permutedClass).toBe(className);
    expect(permutedStyle.ltr).toBe(style.ltr);
    expect(className).toMatchInlineSnapshot('"xpbmbui"');
    expect(style.ltr).toBe(
      `.${className}:lang(B):lang(a):active:hover::part(label){color:red}`,
    );
  });

  test('hashes theme overrides in code-unit order with the default first', () => {
    const vars = { __varGroupHash__: 'xBase', color: 'var(--color)' };
    const upper = '@container B (width > 10px)';
    const lower = '@container a (width > 10px)';
    const [theme, rules] = createTheme(vars, {
      color: { [lower]: 'blue', [upper]: 'red', default: 'black' },
    });
    expect(
      createTheme(vars, {
        color: { default: 'black', [upper]: 'red', [lower]: 'blue' },
      }),
    ).toEqual([theme, rules]);
    expect(theme.xBase).toMatchInlineSnapshot('"x15ir5s4 xBase"');
    const [className] = theme.xBase.split(' ');
    const selector = `.${className}, .${className}:root`;
    expect(Object.values(rules).map(({ ltr }) => ltr)).toEqual([
      `${selector}{--color:black;}`,
      `${upper}{${selector}{--color:red;}}`,
      `${lower}{${selector}{--color:blue;}}`,
    ]);
  });
});
