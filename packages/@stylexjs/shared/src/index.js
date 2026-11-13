/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import {
  default as _getPriority,
  getAtRulePriority as _getAtRulePriority,
  getPseudoElementPriority as _getPseudoElementPriority,
  getPseudoClassPriority as _getPseudoClassPriority,
  getDefaultPriority as _getDefaultPriority,
  PSEUDO_CLASS_PRIORITIES as _PSEUDO_CLASS_PRIORITIES,
  AT_RULE_PRIORITIES as _AT_RULE_PRIORITIES,
  PSEUDO_ELEMENT_PRIORITY as _PSEUDO_ELEMENT_PRIORITY,
} from './utils/property-priorities';
import {
  buildRelationalSelector as _buildRelationalSelector,
  parseRelationalSelector as _parseRelationalSelector,
} from './utils/relational-selectors';

export type {
  RelationalKind,
  RelationalSelector,
} from './utils/relational-selectors';

export const buildRelationalSelector: typeof _buildRelationalSelector =
  _buildRelationalSelector;
export const parseRelationalSelector: typeof _parseRelationalSelector =
  _parseRelationalSelector;

export const getAtRulePriority: typeof _getAtRulePriority = _getAtRulePriority;
export const getPseudoElementPriority: typeof _getPseudoElementPriority =
  _getPseudoElementPriority;
export const getPseudoClassPriority: typeof _getPseudoClassPriority =
  _getPseudoClassPriority;
export const getDefaultPriority: typeof _getDefaultPriority =
  _getDefaultPriority;
export const getPriority: typeof _getPriority = _getPriority;

export const PSEUDO_CLASS_PRIORITIES: typeof _PSEUDO_CLASS_PRIORITIES =
  _PSEUDO_CLASS_PRIORITIES;
export const AT_RULE_PRIORITIES: typeof _AT_RULE_PRIORITIES =
  _AT_RULE_PRIORITIES;
export const PSEUDO_ELEMENT_PRIORITY: typeof _PSEUDO_ELEMENT_PRIORITY =
  _PSEUDO_ELEMENT_PRIORITY;
