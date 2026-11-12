/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import inject from '../src/inject';

describe('inject', () => {
  test('@keyframes', () => {
    const cssText =
      '@keyframes name { from: { color: red }, to: { color: blue } }';
    expect(inject({ ltr: cssText, priority: 10 })).toMatchInlineSnapshot(
      '"@keyframes name { from: { color: red }, to: { color: blue } }"',
    );
  });

  test('@positionTry', () => {
    const cssText =
      '@position-try --name { top: anchor(bottom); left: anchor(left); }';
    expect(inject({ ltr: cssText, priority: 10 })).toMatchInlineSnapshot(
      '"@position-try --name { top: anchor(bottom); left: anchor(left); }"',
    );
  });

  test('::view-transition', () => {
    const cssText =
      '::view-transition-group(*.name){transition-property:none;}::view-transition-image-pair(*.name){border-radius:16px;}::view-transition-old(*.name){animation-duration:.5s;}::view-transition-new(*.name){animation-timing-function:ease-out;}';
    expect(inject({ ltr: cssText, priority: 10 })).toMatchInlineSnapshot(
      '"::view-transition-group(*.name){transition-property:none;}::view-transition-image-pair(*.name){border-radius:16px;}::view-transition-old(*.name){animation-duration:.5s;}::view-transition-new(*.name){animation-timing-function:ease-out;}"',
    );
  });

  test('@media', () => {
    const cssText = '@media (min-width: 320px) { .color { color: red } }';
    expect(inject({ ltr: cssText, priority: 200 })).toMatchInlineSnapshot(
      '"@media (min-width: 320px) { .color { color: red } }"',
    );
  });

  test('::before', () => {
    const cssText = '.color::before { color: red }';
    expect(inject({ ltr: cssText, priority: 5000 })).toMatchInlineSnapshot(
      '".color:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#)::before { color: red }"',
    );
  });

  test(':hover', () => {
    const cssText = '.color:hover { color: red }';
    expect(inject({ ltr: cssText, priority: 130 })).toMatchInlineSnapshot(
      '".color:hover { color: red }"',
    );
  });

  test('::before:hover', () => {
    const cssText = '.color::before:hover { color: red }';
    expect(inject({ ltr: cssText, priority: 5000 })).toMatchInlineSnapshot(
      '".color:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#)::before:hover { color: red }"',
    );
  });

  describe('defineConsts runtime injection', () => {
    test('registers a constant', () => {
      const result = inject({
        ltr: '',
        priority: 0,
        constKey: 'x1abc123',
        constVal: 'rebeccapurple',
      });
      expect(result).toBe('');
    });

    test('inlines constant in CSS rule', () => {
      inject({ ltr: '', priority: 0, constKey: 'x2def456', constVal: 'blue' });

      const cssText = '.test { color: var(--x2def456) }';
      const result = inject({ ltr: cssText, priority: 3000 });

      expect(result).toMatchInlineSnapshot(
        '".test:not(#\\#):not(#\\#):not(#\\#){ color: blue }"',
      );
    });

    test('inlines multiple constants in one rule', () => {
      inject({ ltr: '', priority: 0, constKey: 'xcolor1', constVal: 'red' });
      inject({ ltr: '', priority: 0, constKey: 'xcolor2', constVal: 'green' });

      const cssText =
        '.multi { color: var(--xcolor1); background-color: var(--xcolor2) }';
      const result = inject({ ltr: cssText, priority: 3000 });

      expect(result).toMatchInlineSnapshot(
        '".multi:not(#\\#):not(#\\#):not(#\\#){ color: red; background-color: green }"',
      );
    });

    test('handles numeric constants', () => {
      inject({ ltr: '', priority: 0, constKey: 'xnum123', constVal: 10 });

      const cssText = '.numeric { padding: var(--xnum123) }';
      const result = inject({ ltr: cssText, priority: 1000 });

      expect(result).toMatchInlineSnapshot(
        '".numeric:not(#\\#){ padding: 10 }"',
      );
    });

    test('substitutes constants inside calc() expressions', () => {
      inject({ ltr: '', priority: 0, constKey: 'xcalca1', constVal: 26 });
      inject({ ltr: '', priority: 0, constKey: 'xcalcb2', constVal: 14 });

      const cssText =
        '.calc { z-index: calc(var(--xcalca1) + var(--xcalcb2)) }';
      const result = inject({ ltr: cssText, priority: 1000 });

      expect(result).toMatchInlineSnapshot(
        '".calc:not(#\\#){ z-index: calc(26 + 14) }"',
      );
    });

    test('substitutes unit constants inside calc() expressions', () => {
      inject({ ltr: '', priority: 0, constKey: 'xcalcpx1', constVal: '16px' });

      const cssText = '.calcpx { padding: calc(var(--xcalcpx1) * 2) }';
      const result = inject({ ltr: cssText, priority: 1000 });

      expect(result).toMatchInlineSnapshot(
        '".calcpx:not(#\\#){ padding: calc(16px * 2) }"',
      );
    });

    test('leaves unresolved var() inside calc() intact', () => {
      inject({ ltr: '', priority: 0, constKey: 'xcalcc3', constVal: 26 });

      const cssText =
        '.calcvar { margin: calc(var(--xcalcc3) * var(--xunresolved)) }';
      const result = inject({ ltr: cssText, priority: 1000 });

      expect(result).toMatchInlineSnapshot(
        '".calcvar:not(#\\#){ margin: calc(26 * var(--xunresolved)) }"',
      );
    });

    test('leaves unreferenced constants unchanged in CSS', () => {
      inject({ ltr: '', priority: 0, constKey: 'xunused', constVal: 'purple' });

      const cssText = '.normal { color: orange }';
      const result = inject({ ltr: cssText, priority: 3000 });

      expect(result).toMatchInlineSnapshot(
        '".normal:not(#\\#):not(#\\#):not(#\\#){ color: orange }"',
      );
    });

    test('handles constants in media queries', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xbp768',
        constVal: '@media (min-width: 768px)',
      });

      const cssText = 'var(--xbp768){.xbs0o1n.xbs0o1n{color:blue}}';
      const result = inject({ ltr: cssText, priority: 6000 });

      expect(result).toMatchInlineSnapshot(
        '"@media (min-width: 768px){.xbs0o1n.xbs0o1n:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#){color:blue}}"',
      );
    });

    test('handles nested at-rules with constants', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xbig',
        constVal: '@media (min-width: 1024px)',
      });
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xsmall',
        constVal: '@media (min-width: 768px)',
      });

      const cssText =
        'var(--xsmall){var(--xbig){.xrf68et.xrf68et.xrf68et{color:red}}}';
      const result = inject({ ltr: cssText, priority: 9000 });

      expect(result).toMatchInlineSnapshot(
        '"@media (min-width: 768px){@media (min-width: 1024px){.xrf68et.xrf68et.xrf68et:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#){color:red}}}"',
      );
    });

    test('handles constants for container queries', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xcq500',
        constVal: '@media (min-width: 500px)',
      });

      const cssText = 'var(--xcq500){.container-class{padding:20px}}';
      const result = inject({ ltr: cssText, priority: 6000 });

      expect(result).toMatchInlineSnapshot(
        '"@media (min-width: 500px){.container-class:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#){padding:20px}}"',
      );
    });

    test('handles constants for supports queries', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xsupports',
        constVal: '@media (display: grid)',
      });

      const cssText = 'var(--xsupports){.grid-container{display:grid}}';
      const result = inject({ ltr: cssText, priority: 6000 });

      expect(result).toMatchInlineSnapshot(
        '"@media (display: grid){.grid-container:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#){display:grid}}"',
      );
    });

    test('handles constants in CSS variable definitions', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xprimary',
        constVal: 'rebeccapurple',
      });

      const cssText = ':root { --my-color: var(--xprimary) }';
      const result = inject({ ltr: cssText, priority: 0.1 });

      expect(result).toMatchInlineSnapshot(
        '":root { --my-color: rebeccapurple }"',
      );
    });

    test('handles constants with special characters in values', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xspecial',
        constVal: 'url("image.png")',
      });

      const cssText = '.bg { background-image: var(--xspecial) }';
      const result = inject({ ltr: cssText, priority: 3000 });

      expect(result).toMatchInlineSnapshot(
        '".bg:not(#\\#):not(#\\#):not(#\\#){ background-image: url("image.png") }"',
      );
    });

    test('preserves other var() references that are not constants', () => {
      inject({ ltr: '', priority: 0, constKey: 'xknown', constVal: 'blue' });

      const cssText =
        '.mixed { color: var(--xknown); background: var(--unknown-var) }';
      const result = inject({ ltr: cssText, priority: 3000 });

      expect(result).toMatchInlineSnapshot(
        '".mixed:not(#\\#):not(#\\#):not(#\\#){ color: blue; background: var(--unknown-var) }"',
      );
    });

    test('updates dependent rules when constant changes', () => {
      inject({ ltr: '', priority: 0, constKey: 'xdynamic', constVal: 'red' });

      const cssText = '.dynamic { color: var(--xdynamic) }';
      const result1 = inject({ ltr: cssText, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".dynamic:not(#\\#):not(#\\#):not(#\\#){ color: red }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xdynamic', constVal: 'green' });

      const cssText2 = '.dynamic2 { background: var(--xdynamic) }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".dynamic2:not(#\\#):not(#\\#):not(#\\#){ background: green }"',
      );
    });

    test('handles constants with zero values', () => {
      inject({ ltr: '', priority: 0, constKey: 'xzero', constVal: 0 });

      const cssText = '.zero { margin: var(--xzero) }';
      const result = inject({ ltr: cssText, priority: 1000 });

      expect(result).toMatchInlineSnapshot('".zero:not(#\\#){ margin: 0 }"');
    });

    test('handles constants in complex selectors', () => {
      inject({ ltr: '', priority: 0, constKey: 'xhover', constVal: 'blue' });

      const cssText = '.link:hover::after { color: var(--xhover) }';
      const result = inject({ ltr: cssText, priority: 5000 });

      expect(result).toMatchInlineSnapshot(
        '".link:hover:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#)::after { color: blue }"',
      );
    });

    test('updates multiple dependent rules when constant changes', () => {
      inject({ ltr: '', priority: 0, constKey: 'xchanging', constVal: 'red' });

      const cssText1 = '.rule1 { color: var(--xchanging) }';
      const result1 = inject({ ltr: cssText1, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".rule1:not(#\\#):not(#\\#):not(#\\#){ color: red }"',
      );

      const cssText2 = '.rule2 { background: var(--xchanging) }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".rule2:not(#\\#):not(#\\#):not(#\\#){ background: red }"',
      );

      const cssText3 = '.rule3 { border-color: var(--xchanging) }';
      const result3 = inject({ ltr: cssText3, priority: 3000 });
      expect(result3).toMatchInlineSnapshot(
        '".rule3:not(#\\#):not(#\\#):not(#\\#){ border-color: red }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xchanging', constVal: 'blue' });

      const cssText4 = '.rule4 { fill: var(--xchanging) }';
      const result4 = inject({ ltr: cssText4, priority: 3000 });
      expect(result4).toMatchInlineSnapshot(
        '".rule4:not(#\\#):not(#\\#):not(#\\#){ fill: blue }"',
      );
    });

    test('does not update dependent rules if constant value stays the same', () => {
      inject({ ltr: '', priority: 0, constKey: 'xstatic', constVal: 'purple' });

      const cssText = '.static { color: var(--xstatic) }';
      const result1 = inject({ ltr: cssText, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".static:not(#\\#):not(#\\#):not(#\\#){ color: purple }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xstatic', constVal: 'purple' });

      const cssText2 = '.static2 { background: var(--xstatic) }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".static2:not(#\\#):not(#\\#):not(#\\#){ background: purple }"',
      );
    });

    test('updates dependent at-rules when constant changes', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xbpchanging',
        constVal: '@media (min-width: 768px)',
      });

      const cssText1 = 'var(--xbpchanging){.responsive{display:flex}}';
      const result1 = inject({ ltr: cssText1, priority: 6000 });
      expect(result1).toMatchInlineSnapshot(
        '"@media (min-width: 768px){.responsive:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#){display:flex}}"',
      );

      inject({
        ltr: '',
        priority: 0,
        constKey: 'xbpchanging',
        constVal: '@media (min-width: 1024px)',
      });

      const cssText2 = 'var(--xbpchanging){.responsive2{display:grid}}';
      const result2 = inject({ ltr: cssText2, priority: 6000 });
      expect(result2).toMatchInlineSnapshot(
        '"@media (min-width: 1024px){.responsive2:not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#):not(#\\#){display:grid}}"',
      );
    });

    test('prevents duplicate dependency tracking', () => {
      inject({ ltr: '', priority: 0, constKey: 'xnodupe', constVal: 'purple' });

      const cssText = '.nodupe { color: var(--xnodupe) }';
      inject({ ltr: cssText, priority: 3000 });
      inject({ ltr: cssText, priority: 3000 });
      inject({ ltr: cssText, priority: 3000 });

      inject({ ltr: '', priority: 0, constKey: 'xnodupe', constVal: 'orange' });

      const cssText2 = '.nodupe2 { background: var(--xnodupe) }';
      const result = inject({ ltr: cssText2, priority: 3000 });
      expect(result).toMatchInlineSnapshot(
        '".nodupe2:not(#\\#):not(#\\#):not(#\\#){ background: orange }"',
      );
    });

    test('tracks dependencies for multiple constants in one rule', () => {
      inject({ ltr: '', priority: 0, constKey: 'xmulti1', constVal: 'red' });
      inject({ ltr: '', priority: 0, constKey: 'xmulti2', constVal: 'blue' });

      const cssText =
        '.multi { color: var(--xmulti1); background: var(--xmulti2) }';
      const result1 = inject({ ltr: cssText, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".multi:not(#\\#):not(#\\#):not(#\\#){ color: red; background: blue }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xmulti1', constVal: 'green' });

      const cssText2 = '.multi2 { border-color: var(--xmulti1) }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".multi2:not(#\\#):not(#\\#):not(#\\#){ border-color: green }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xmulti2', constVal: 'yellow' });

      const cssText3 = '.multi3 { fill: var(--xmulti2) }';
      const result3 = inject({ ltr: cssText3, priority: 3000 });
      expect(result3).toMatchInlineSnapshot(
        '".multi3:not(#\\#):not(#\\#):not(#\\#){ fill: yellow }"',
      );
    });

    test('correctly updates stylesheet when constant value changes', () => {
      inject({
        ltr: '',
        priority: 0,
        constKey: 'xupdatetest',
        constVal: 'red',
      });

      const cssText = '.updatetest { color: var(--xupdatetest) }';
      const result1 = inject({ ltr: cssText, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".updatetest:not(#\\#):not(#\\#):not(#\\#){ color: red }"',
      );

      inject({
        ltr: '',
        priority: 0,
        constKey: 'xupdatetest',
        constVal: 'green',
      });

      const cssText2 = '.updatetest2 { background: var(--xupdatetest) }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".updatetest2:not(#\\#):not(#\\#):not(#\\#){ background: green }"',
      );
    });

    test('handles constant updates with at-rules and regular properties', () => {
      inject({ ltr: '', priority: 0, constKey: 'xflexible', constVal: 'red' });

      const cssText1 = '.prop { color: var(--xflexible) }';
      const result1 = inject({ ltr: cssText1, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".prop:not(#\\#):not(#\\#):not(#\\#){ color: red }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xflexible', constVal: 'blue' });

      const cssText2 = '.prop2 { border-color: var(--xflexible) }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".prop2:not(#\\#):not(#\\#):not(#\\#){ border-color: blue }"',
      );
    });

    test('handles mixed constant and non-constant values', () => {
      inject({ ltr: '', priority: 0, constKey: 'xmixed', constVal: '10px' });

      const cssText = '.mixed { padding: var(--xmixed); margin: 20px }';
      const result1 = inject({ ltr: cssText, priority: 3000 });
      expect(result1).toMatchInlineSnapshot(
        '".mixed:not(#\\#):not(#\\#):not(#\\#){ padding: 10px; margin: 20px }"',
      );

      inject({ ltr: '', priority: 0, constKey: 'xmixed', constVal: '15px' });

      const cssText2 = '.mixed2 { padding: var(--xmixed); margin: 20px }';
      const result2 = inject({ ltr: cssText2, priority: 3000 });
      expect(result2).toMatchInlineSnapshot(
        '".mixed2:not(#\\#):not(#\\#):not(#\\#){ padding: 15px; margin: 20px }"',
      );
    });
  });
});
