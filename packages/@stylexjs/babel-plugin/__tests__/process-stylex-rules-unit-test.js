/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import stylexPlugin from '../src/index';

describe('@stylexjs/babel-plugin processStylexRules()', () => {
  test('resolves constants, preserves first class order, and wraps direction-aware rules', () => {
    const output = stylexPlugin.processStylexRules(
      [
        [
          'constColor',
          { ltr: '', constKey: 'constColor', constVal: 'var(--brand-color)' },
          0,
        ],
        [
          'brand-color',
          { ltr: '', constKey: 'brand-color', constVal: '#123456' },
          0,
        ],
        [
          'alpha',
          {
            ltr: '.alpha{color:var(--constColor)}',
            rtl: '@media print{.alpha{color:var(--constColor)}}',
          },
          3000,
        ],
        [
          'alpha',
          {
            ltr: '.alpha{color:red}',
            rtl: '@media print{.alpha{color:red}}',
          },
          3000,
        ],
      ],
      {
        useLayers: {
          before: ['reset'],
          after: ['utilities'],
          prefix: 'stylex',
        },
        enableLTRRTLComments: true,
      },
    );

    expect(output)
      .toBe(`\n@layer reset, stylex.priority1, utilities;\n@layer stylex.priority1{
/* @ltr begin */.alpha{color:#123456}/* @ltr end */
/* @rtl begin */@media print{.alpha{color:#123456}}/* @rtl end */
}`);
  });

  test('injects logical float variables before processed rules', () => {
    const output = stylexPlugin.processStylexRules([
      [
        'logicalFloat',
        {
          ltr: '.logicalFloat{float:var(--stylex-logical-start)}',
          rtl: null,
        },
        3000,
      ],
    ]);

    expect(output).toBe(`:root, [dir="ltr"] {
  --stylex-logical-start: left;
  --stylex-logical-end: right;
}
[dir="rtl"] {
  --stylex-logical-start: right;
  --stylex-logical-end: left;
}
.logicalFloat{float:var(--stylex-logical-start)}`);
  });
});
