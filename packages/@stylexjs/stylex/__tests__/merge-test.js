/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import { merge } from '../src/merge';
import { styleq } from 'styleq';

describe('merge', () => {
  test('later styles win, including nulls', () => {
    const a = { color: 'red', margin: 'm0', $$css: true };
    const b = { color: 'blue', margin: null, $$css: true };
    expect(merge([a, b])).toEqual(['blue', null, '']);
    expect(merge([b, a])).toEqual(['red m0', null, '']);
  });

  test('cached results match uncached results', () => {
    const a = { color: 'red', padding: 'p1', $$css: true };
    const b = { padding: 'p2', $$css: true };
    const first = merge([a, false, [b]]);
    expect(merge([a, false, [b]])).toEqual(first);
    expect(merge([b, a])).toEqual(['red p1', null, '']);
    expect(merge([a])).toEqual(['red p1', null, '']);
  });

  test('inline styles merge with compiled styles', () => {
    const a = { color: 'red', opacity: 'o1', $$css: true };
    expect(merge([a, { opacity: 0.5, width: undefined }])).toEqual([
      'red',
      { opacity: 0.5 },
      '',
    ]);
    expect(merge([{ opacity: 0.5 }, a])).toEqual(['red o1', null, '']);
    expect(merge([a, { opacity: null }])).toEqual(['red', null, '']);
  });

  test('joins debug strings from compiled styles', () => {
    const a = { color: 'red', $$css: 'a.js:1' };
    const b = { width: 'w1', $$css: 'b.js:2' };
    expect(merge([a, b])).toEqual(['red w1', null, 'a.js:1; b.js:2']);
    expect(merge([a, b])).toEqual(['red w1', null, 'a.js:1; b.js:2']);
    expect(merge([b])).toEqual(['w1', null, 'b.js:2']);
  });

  test('handles many distinct properties', () => {
    const many = {};
    for (let i = 0; i < 2000; i++) {
      many[`p${i}`] = `c${i}`;
    }
    many.$$css = true;
    const override = { p0: 'x', p1999: null, $$css: true };
    expect(merge([many, override])).toEqual(styleq([many, override]));
    expect(merge([override, many, { p5: 'y' }])).toEqual(
      styleq([override, many, { p5: 'y' }]),
    );
  });

  test('matches styleq on random input', () => {
    let seed = 7;
    const random = () => {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      return seed / 0x7fffffff;
    };
    const keys = [
      'color',
      'margin',
      'marginTop',
      'padding',
      '--x',
      'kAbc',
      '0',
      '10',
      '',
      'constructor',
      'toString',
    ];
    const proto = { inherited: 'xi', $$css: true };
    const compiled = Array.from({ length: 40 }, (_, n) => {
      const style = n % 10 === 0 ? Object.create(proto) : {};
      for (const key of keys) {
        if (random() < 0.4) {
          style[key] = random() < 0.2 ? null : `x${Math.floor(random() * 20)}`;
        }
      }
      style.$$css =
        random() < 0.3 ? `file${Math.floor(random() * 3)}.js` : true;
      return style;
    });
    const inline = () => {
      const style = {};
      for (const key of keys) {
        if (random() < 0.3) {
          const r = random();
          style[key] =
            r < 0.1 ? null : r < 0.2 ? undefined : r < 0.5 ? r : `${r}px`;
        }
      }
      return style;
    };
    const arg = (depth) => {
      const r = random();
      if (r < 0.08) return false;
      if (r < 0.12) return null;
      if (r < 0.14) return undefined;
      if (r < 0.15) return true;
      if (r < 0.27 && depth < 3) {
        return Array.from({ length: Math.floor(random() * 4) }, () =>
          arg(depth + 1),
        );
      }
      if (r < 0.37) return inline();
      const style = compiled[Math.floor(random() * compiled.length)];
      return r < 0.42 ? { ...style } : style;
    };
    const calls = Array.from({ length: 2000 }, () =>
      Array.from({ length: Math.floor(random() * 6) }, () => arg(0)),
    );
    for (let round = 0; round < 2; round++) {
      for (const styles of calls) {
        expect(merge(styles)).toEqual(styleq(styles));
      }
    }
  });
});
