/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { StyleXOptions } from '../common-types';

import { buildRelationalSelector } from '@stylexjs/shared';
import { defaultOptions } from '../utils/default-options';

function fromProxy(value: mixed): ?string {
  if (
    typeof value === 'object' &&
    value != null &&
    value.__IS_PROXY === true &&
    typeof value.toString === 'function'
  ) {
    return value.toString();
  }
  return null;
}

function fromStyleXStyle(value: mixed): ?string {
  if (typeof value === 'object' && value != null && value.$$css === true) {
    return Object.keys(value).find((key) => key !== '$$css');
  }
  return null;
}

function getDefaultMarkerClassName(
  options: StyleXOptions = defaultOptions,
): string {
  const valueFromProxy = fromProxy(options);
  if (valueFromProxy != null) {
    return valueFromProxy;
  }
  const valueFromStyleXStyle = fromStyleXStyle(options);
  if (valueFromStyleXStyle != null) {
    return valueFromStyleXStyle;
  }
  const prefix =
    options.classNamePrefix != null ? `${options.classNamePrefix}-` : '';
  return `${prefix}default-marker`;
}

type WhenSelector = StringPrefix<':'> | StringPrefix<'['>;

/**
 * Validates that a pseudo selector starts with ':' but not '::',
 * or is an attribute selector that starts with '[' and ends with ']'
 */
function validatePseudoSelector(pseudo: string): void {
  if (!(pseudo.startsWith(':') || pseudo.startsWith('['))) {
    throw new Error('Pseudo selector must start with ":" or "["');
  }
  if (pseudo.startsWith('::')) {
    throw new Error(
      'Pseudo selector cannot start with "::" (pseudo-elements are not supported)',
    );
  }
  if (pseudo.startsWith('[') && !pseudo.endsWith(']')) {
    throw new Error('Attribute selector must end with "]"');
  }
}
/**
 * Creates selector that observes if the given pseudo-class is
 * active on an ancestor with the "defaultMarker"
 *
 *
 * @param pseudo - The pseudo selector (e.g., ':hover', ':focus')
 * @returns A :where() clause for the ancestor observer
 */
export function ancestor(
  pseudo: WhenSelector,
  options: string | StyleXOptions = defaultOptions,
): string {
  validatePseudoSelector(pseudo);
  const defaultMarker =
    typeof options === 'string' ? options : getDefaultMarkerClassName(options);

  return buildRelationalSelector({
    kind: 'ancestor',
    marker: defaultMarker,
    pseudo,
  });
}

/**
 * Creates selector that observes if the given pseudo-class is
 * active on a descendant with the "defaultMarker"
 *
 * @param pseudo - The pseudo selector (e.g., ':hover', ':focus')
 * @returns A :has() clause for the descendant observer
 */
export function descendant(
  pseudo: WhenSelector,
  options: string | StyleXOptions = defaultOptions,
): string {
  validatePseudoSelector(pseudo);
  const defaultMarker =
    typeof options === 'string' ? options : getDefaultMarkerClassName(options);

  return buildRelationalSelector({
    kind: 'descendant',
    marker: defaultMarker,
    pseudo,
  });
}

/**
 * Creates selector that observes if the given pseudo-class is
 * active on a previous sibling with the "defaultMarker"
 *
 * @param pseudo - The pseudo selector (e.g., ':hover', ':focus')
 * @returns A :where() clause for the previous sibling observer
 */
export function siblingBefore(
  pseudo: WhenSelector,
  options: string | StyleXOptions = defaultOptions,
): string {
  validatePseudoSelector(pseudo);
  const defaultMarker =
    typeof options === 'string' ? options : getDefaultMarkerClassName(options);

  return buildRelationalSelector({
    kind: 'siblingBefore',
    marker: defaultMarker,
    pseudo,
  });
}

/**
 * Creates selector that observes if the given pseudo-class is
 * active on a next sibling with the "defaultMarker"
 *
 * @param pseudo - The pseudo selector (e.g., ':hover', ':focus')
 * @returns A :has() clause for the next sibling observer
 */
export function siblingAfter(
  pseudo: WhenSelector,
  options: string | StyleXOptions = defaultOptions,
): string {
  validatePseudoSelector(pseudo);
  const defaultMarker =
    typeof options === 'string' ? options : getDefaultMarkerClassName(options);

  return buildRelationalSelector({
    kind: 'siblingAfter',
    marker: defaultMarker,
    pseudo,
  });
}

/**
 * Creates selector that observes if the given pseudo-class is
 * active on any sibling with the "defaultMarker"
 *
 * @param pseudo - The pseudo selector (e.g., ':hover', ':focus')
 * @returns A :where() clause for the any sibling observer
 */
export function anySibling(
  pseudo: WhenSelector,
  options: string | StyleXOptions = defaultOptions,
): string {
  validatePseudoSelector(pseudo);
  const defaultMarker =
    typeof options === 'string' ? options : getDefaultMarkerClassName(options);

  return buildRelationalSelector({
    kind: 'anySibling',
    marker: defaultMarker,
    pseudo,
  });
}
