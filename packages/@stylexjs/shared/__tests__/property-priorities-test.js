/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

import getPriority from '../src/utils/property-priorities';

describe('getPriority', () => {
  describe('compound pseudo selectors', () => {
    test('sums the priorities of simple pseudo-classes and pseudo-elements', () => {
      expect(getPriority(':hover::after')).toBe(5130);
      expect(getPriority(':hover:focus')).toBe(280);
    });

    test('opts out of functional pseudo-classes', () => {
      expect(getPriority(':nth-child(2):hover')).toBe(60);
      expect(getPriority(':not(:is(:hover)):focus')).toBe(40);
      expect(getPriority(':hover:has([title="(x)"])')).toBe(40);
    });

    test('preserves existing behavior for unclosed parens and functional pseudo-elements', () => {
      expect(getPriority(':hover:focus(')).toBe(280);
      expect(getPriority(':hover::part(x)')).toBe(5130);
    });

    test('runs in linear time on repeated unclosed functional pseudo-classes', () => {
      const key = ':-('.repeat(50000);
      const start = Date.now();
      getPriority(key);
      expect(Date.now() - start).toBeLessThan(1000);
    });
  });
});
