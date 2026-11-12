/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

import type { CSSProperties, StyleXStyles } from '@stylexjs/stylex';
export type Atom = StyleXStyles<{ readonly [$$Key$$: string]: unknown }>;
export type AtomProperty = {
  readonly [$$Key$$: string]: Atom;
  (value: string | number): Atom;
};
export type Atoms = { readonly [Key in keyof CSSProperties]-?: AtomProperty };
declare const atoms: Atoms;
export default atoms;
