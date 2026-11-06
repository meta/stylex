/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { RelationalKind } from '../src/utils/relational-selectors';

import getPriority from '../src/utils/property-priorities';
import {
  buildRelationalSelector,
  parseRelationalSelector,
} from '../src/utils/relational-selectors';

const KINDS: $ReadOnlyArray<RelationalKind> = [
  'ancestor',
  'descendant',
  'anySibling',
  'siblingBefore',
  'siblingAfter',
];

const PSEUDOS = [
  ':hover',
  ':hover:focus',
  ':focus-visible',
  ':nth-child(2)',
  ':has(> *)',
  ':has(*)',
  ':has(~ *)',
  ':is(.a, .b)',
  ':is(.a ~ *)',
  ':is(.a ~ *, :has(~ .b))',
  ':not(:has(.a *))',
  '[data-open]',
  '[data-state="a *)"]',
  '[data-state=" ~ *, :has(~ "]',
];

describe('relational selectors', () => {
  describe('keys', () => {
    test('keep the shape that class names are hashed from', () => {
      const selector = { marker: 'm', pseudo: ':hover' };
      expect(
        KINDS.map((kind) => buildRelationalSelector({ ...selector, kind })),
      ).toEqual([
        ':where(.m:hover *)',
        ':where(:has(.m:hover))',
        ':where(.m:hover ~ *, :has(~ .m:hover))',
        ':where(.m:hover ~ *)',
        ':where(:has(~ .m:hover))',
      ]);
    });
  });

  describe('parseRelationalSelector', () => {
    for (const kind of KINDS) {
      test.each(PSEUDOS)(`reads back ${kind} with %s`, (pseudo) => {
        const selector = { kind, marker: 'x-marker_1', pseudo };
        expect(
          parseRelationalSelector(buildRelationalSelector(selector)),
        ).toEqual(selector);
      });
    }

    test.each([
      'color',
      '::before',
      ':hover',
      ':where(.a)',
      ':where(:hover *)',
      ':where(.a *)',
      ':where(.a:hover *):focus',
      ':where(.a:hover .b)',
      ':is(.a:hover *)',
      ':where(.a:hover ~ *, :has(~ .b:hover))',
      ':where(.a:hover ~ *, :has(~ .a:focus))',
    ])('ignores %s', (key) => {
      expect(parseRelationalSelector(key)).toBeNull();
    });
  });

  describe('priorities', () => {
    test.each([
      [':where(.m:hover *)', 11.3],
      [':where(:has(.m:hover))', 16.3],
      [':where(.m:hover ~ *, :has(~ .m:hover))', 21.3],
      [':where(.m:hover ~ *)', 31.3],
      [':where(:has(~ .m:hover))', 41.3],
      [':where(.m:unknown-pseudo *)', 10.4],
      // Hand-written variants of the same shapes.
      [':where(.m:hover   *)', 11.3],
      [':where(.m:hover  ~  *)', 31.3],
      [':where(.a:hover ~ *, :has(~ .b:active))', 21.7],
    ])('prices the simple pseudo in %s', (key, priority) => {
      expect(getPriority(key)).toBeCloseTo(priority);
    });

    test.each([
      ':where(.m:has(> *) *)',
      ':where(.m:hover:focus *)',
      ':where(.m[data-open] ~ *)',
      ':where(:has(.m:nth-child(2)))',
      ':where(:has(~ .m:is(.a, .b)))',
      ':where(.m:is(.a, .b) ~ *, :has(~ .m:is(.a, .b)))',
    ])('prices %s as a plain :where()', (key) => {
      expect(getPriority(key)).toBe(40);
    });
  });
});
