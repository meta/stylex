/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

export type RelationalKind =
  | 'ancestor'
  | 'descendant'
  | 'anySibling'
  | 'siblingBefore'
  | 'siblingAfter';

export type RelationalSelector = $ReadOnly<{
  kind: RelationalKind,
  marker: string,
  pseudo: string,
}>;

// Each pattern captures the marker class and the pseudo. Keys that a pattern
// could read in more than one way resolve by order: `.m:is(.a) ~ *` also fits
// the ancestor pattern, so the sibling patterns come first.
const RELATIONAL_SELECTORS: $ReadOnlyArray<[RelationalKind, RegExp]> = [
  [
    'anySibling',
    /^:where\(\.([0-9a-zA-Z_-]+)([:[][\s\S]*?)\s+~\s+\*,\s+:has\(~\s\.\1\2\)\)$/,
  ],
  ['siblingBefore', /^:where\(\.([0-9a-zA-Z_-]+)([:[][\s\S]*?)\s+~\s+\*\)$/],
  ['ancestor', /^:where\(\.([0-9a-zA-Z_-]+)([:[][\s\S]*?)\s+\*\)$/],
  ['siblingAfter', /^:where\(:has\(~\s\.([0-9a-zA-Z_-]+)([:[][\s\S]*?)\)\)$/],
  ['descendant', /^:where\(:has\(\.([0-9a-zA-Z_-]+)([:[][\s\S]*?)\)\)$/],
];

/**
 * Builds the selector for a relational `stylex.when.*` key.
 *
 * `subject` replaces the `*` that the element being styled matches. Keys always
 * use the default, because class names are hashed from them. The CSS builder
 * passes the rule's own class. `descendant` and `siblingAfter` have no `*`, since
 * their subject is the `:has()` itself, so they ignore `subject`.
 */
export function buildRelationalSelector(
  { kind, marker, pseudo }: RelationalSelector,
  subject: string = '*',
): string {
  const target = `.${marker}${pseudo}`;
  switch (kind) {
    case 'ancestor':
      return `:where(${target} ${subject})`;
    case 'descendant':
      return `:where(:has(${target}))`;
    case 'anySibling':
      return `:where(${target} ~ ${subject}, :has(~ ${target}))`;
    case 'siblingBefore':
      return `:where(${target} ~ ${subject})`;
    case 'siblingAfter':
      return `:where(:has(~ ${target}))`;
    default:
      kind as empty;
      throw new Error(`Unknown relational selector kind: ${kind}`);
  }
}

/**
 * Reads a key built by `buildRelationalSelector`. Returns null for anything else.
 */
export function parseRelationalSelector(key: string): ?RelationalSelector {
  if (!key.startsWith(':where(')) {
    return null;
  }
  for (const [kind, pattern] of RELATIONAL_SELECTORS) {
    const match = pattern.exec(key);
    if (match) {
      return { kind, marker: match[1], pseudo: match[2] };
    }
  }
  return null;
}
