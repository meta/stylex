/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

import * as stylex from '@stylexjs/stylex';
import type { CSSProperties } from '@stylexjs/stylex';

/* eslint-disable no-unused-vars */

/**
 * `(string & {})` OPEN UNIONS
 *
 * `StyleXCSSTypes.d.ts` uses `(string & {})` instead of a bare `string` so that
 * literal members survive in the union (and keep their editor autocomplete)
 * while arbitrary strings are still accepted. Flow cannot express this, which
 * is why the TypeScript definitions are maintained separately rather than
 * translated from the Flow types.
 */

// Arbitrary strings are still accepted -- the union is genuinely open.
const arbitrary: CSSProperties = {
  appearance: 'some-future-value',
  filter: 'blur(2px) saturate(1.5)',
  color: 'oklch(0.7 0.1 200)',
  height: 'calc(100% - 10px)',
};

// `var()` references are accepted on open properties.
const vars: CSSProperties = {
  color: 'var(--my-color)',
  filter: 'var(--my-filter)',
};

// `null` remains assignable. This is significant in StyleX: `null` is how a
// property is unset, e.g. in a conditional style. `{}` in TypeScript excludes
// `null`, so this asserts that the `string & {}` intersection did not leak into
// the `null` member of the `all` type.
const nulls: CSSProperties = {
  appearance: null,
  color: null,
  filter: null,
  height: null,
  display: null,
};

// The global keywords from `all` survive alongside `(string & {})`.
const globals: CSSProperties = {
  color: 'inherit',
  height: 'initial',
  filter: 'unset',
};

// Literal members are preserved rather than being absorbed by `string`.
// NOTE: do not use `NonNullable` to strip `null` here -- it is defined as
// `T & {}` in TypeScript >=4.9, and that intersection distributes over the
// union and flattens `'textfield' | (string & {})` back down to `string`,
// which would make this assertion vacuously pass.
type Strip<T> = T extends null | undefined ? never : T;
type Appearance = Strip<CSSProperties['appearance']>;
const literalsSurvive: Extract<Appearance, 'textfield'> = 'textfield';

// ...and a literal that is NOT part of the union is still rejected, so the
// open union has not silently become a plain `string`.
// @ts-expect-error - 'textfeild' is a typo, not a member of the union.
const typoRejected: Extract<Appearance, 'textfeild'> = 'textfeild';

// A bare `string` type is NOT assignable to a *closed* union, proving the
// distinction between open and closed properties is still enforced.
declare const someString: string;
// @ts-expect-error - `backfaceVisibility` is a closed union.
const closedStillClosed: CSSProperties = { backfaceVisibility: someString };

// `null` unsetting inside a real `stylex.create` call with conditions.
const styles = stylex.create({
  conditional: (isActive: boolean) => ({
    color: isActive ? 'red' : null,
    appearance: isActive ? 'none' : null,
  }),
});
