/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type {
  ClassOverride,
  InlineOverride,
  RuleOverride,
  StylexOverride,
} from '../types.js';
import type { RuleOverrideSheet } from './ruleOverrideSheet.js';

export type InlineOverrideRecord = {
  kind: 'inline',
  publicOverride: InlineOverride,
  originalPresent: boolean,
  originalValue: string,
  originalPriority: string,
};

export type ClassOverrideRecord = {
  kind: 'class',
  publicOverride: ClassOverride,
  originalClassName: string,
  originalClassPresent: boolean,
  replacementClassName: string,
  replacementClassPresent: boolean,
};

export type RuleOverrideRecord = {
  kind: 'rule',
  publicOverride: RuleOverride,
  ruleSheet: RuleOverrideSheet,
  targetAttributeOriginalPresent: boolean,
  targetAttributeOriginalValue: string,
};

export type OverrideRecord =
  | InlineOverrideRecord
  | RuleOverrideRecord
  | ClassOverrideRecord;

type Selection = {
  id: string,
  element: Element,
};

const selectionIds: WeakMap<Element, string> = new WeakMap();
const overrideRecords: WeakMap<
  Element,
  Map<string, OverrideRecord>,
> = new WeakMap();
let nextSelectionId = 1;
let currentSelection: Selection | null = null;

export function isElementNode(value: mixed): implies value is Element {
  return value instanceof Element;
}

export function registerSelection(element: Element): string {
  let id = selectionIds.get(element);
  if (id == null) {
    id = `selection-${nextSelectionId}`;
    nextSelectionId += 1;
    selectionIds.set(element, id);
  }
  currentSelection = { id, element };
  return id;
}

export function clearSelection(): void {
  currentSelection = null;
}

export function identifySelection(value: mixed): string {
  if (value == null) {
    clearSelection();
    return 'none';
  }
  if (!isElementNode(value)) {
    clearSelection();
    return 'non-element';
  }
  return registerSelection(value);
}

export function getCurrentSelection(): Selection | null {
  return currentSelection;
}

export function getOverrideRecordMap(
  element: Element,
): Map<string, OverrideRecord> {
  return overrideRecords.get(element) ?? new Map();
}

export function setOverrideRecordMap(
  element: Element,
  records: Map<string, OverrideRecord>,
): void {
  if (records.size === 0) {
    overrideRecords.delete(element);
  } else {
    overrideRecords.set(element, records);
  }
}

export function getPublicOverrides(element: Element): Array<StylexOverride> {
  return Array.from(getOverrideRecordMap(element).values(), (record) => ({
    ...record.publicOverride,
  }));
}
