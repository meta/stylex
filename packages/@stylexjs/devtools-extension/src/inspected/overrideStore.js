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
  OverrideCommand,
  RuleOverride,
} from '../types.js';
import { normalizeCssProperty } from '../utils/css.js';
import type { ClassOverrideRecord, OverrideRecord } from './runtimeState.js';
import {
  createRuleOverrideSheet,
  removeRuleOverrideSheet,
  restoreRuleOverrideSheet,
  RULE_TARGET_ATTRIBUTE,
} from './ruleOverrideSheet.js';
import {
  getCurrentSelection,
  getOverrideRecordMap,
  isElementNode,
  setOverrideRecordMap,
} from './runtimeState.js';

function tupleKey(parts: $ReadOnlyArray<mixed>): string {
  return JSON.stringify(parts);
}

function inlineOverrideId(property: string): string {
  return tupleKey(['inline', normalizeCssProperty(property)]);
}

function classOverrideId(originalClassName: string): string {
  return tupleKey(['class', originalClassName]);
}

function ruleOverrideId(pseudoElement: string, property: string): string {
  return tupleKey(['rule', pseudoElement, normalizeCssProperty(property)]);
}

function hasInlineProperty(style: CSSStyleDeclaration, property: string) {
  const normalizedProperty = normalizeCssProperty(property);
  for (let index = 0; index < style.length; index += 1) {
    if (normalizeCssProperty(style[index]) === normalizedProperty) return true;
  }
  return false;
}

function validateCssValue(property: string, value: string): void {
  const cssApi = (globalThis as any).CSS;
  if (
    !property.startsWith('--') &&
    typeof cssApi?.supports === 'function' &&
    !cssApi.supports(property, value)
  ) {
    throw new Error(`Invalid value for ${property}: ${value}`);
  }
}

function setClassPresence(
  element: Element,
  className: string,
  shouldBePresent: boolean,
): void {
  if (shouldBePresent) {
    element.classList.add(className);
  } else {
    element.classList.remove(className);
  }
}

function getStyle(element: Element): CSSStyleDeclaration {
  const style = (element as any).style;
  if (style == null || typeof style.setProperty !== 'function') {
    throw new Error('The selected element does not support inline styles.');
  }
  return style;
}

function restoreRecord(element: Element, record: OverrideRecord): void {
  if (record.kind === 'inline') {
    const style = getStyle(element);
    if (record.originalPresent) {
      style.setProperty(
        record.publicOverride.property,
        record.originalValue,
        record.originalPriority,
      );
    } else {
      style.removeProperty(record.publicOverride.property);
    }
    return;
  }

  if (record.kind === 'rule') {
    removeRuleOverrideSheet(record.ruleSheet);
    return;
  }

  setClassPresence(
    element,
    record.originalClassName,
    record.originalClassPresent,
  );
  if (record.replacementClassName !== record.originalClassName) {
    setClassPresence(
      element,
      record.replacementClassName,
      record.replacementClassPresent,
    );
  }
}

function removeRecords(
  element: Element,
  records: Map<string, OverrideRecord>,
  ids: $ReadOnlyArray<string>,
): Array<ClassOverrideRecord> {
  const removedClassRecords = [];
  for (const id of new Set(ids)) {
    const record = records.get(id);
    if (record == null) continue;
    restoreRecord(element, record);
    records.delete(id);
    if (record.kind === 'class') removedClassRecords.push(record);
  }
  return removedClassRecords;
}

function findExistingClassRecord(
  records: Map<string, OverrideRecord>,
  command: OverrideCommand,
): ClassOverrideRecord | null {
  if (command.type !== 'swap-class') return null;
  for (const record of records.values()) {
    if (
      record.kind === 'class' &&
      (record.replacementClassName === command.fromClassName ||
        record.originalClassName === command.fromClassName ||
        (command.sourceEntryKey != null &&
          record.publicOverride.sourceEntryKey === command.sourceEntryKey))
    ) {
      return record;
    }
  }
  return null;
}

function setInlineOverride(
  element: Element,
  records: Map<string, OverrideRecord>,
  command: OverrideCommand,
): void {
  if (command.type !== 'set-inline') return;
  const property = normalizeCssProperty(command.property);
  if (property === '' || command.value.trim() === '') {
    throw new Error('Inline overrides require a property and value.');
  }
  validateCssValue(property, command.value);

  const id = inlineOverrideId(property);
  const existing = records.get(id);
  const style = getStyle(element);
  const originalPresent =
    existing?.kind === 'inline'
      ? existing.originalPresent
      : hasInlineProperty(style, property);
  const originalValue =
    existing?.kind === 'inline'
      ? existing.originalValue
      : style.getPropertyValue(property);
  const originalPriority =
    existing?.kind === 'inline'
      ? existing.originalPriority
      : style.getPropertyPriority(property);

  const publicOverride: InlineOverride = {
    id,
    kind: 'inline',
    contextKey: command.contextKey,
    property,
    value: command.value,
    important: command.important,
    conditions: command.conditions,
  };
  if (command.sourceEntryKey != null) {
    publicOverride.sourceEntryKey = command.sourceEntryKey;
  }

  style.setProperty(
    property,
    command.value,
    command.important ? 'important' : '',
  );
  if (
    !hasInlineProperty(style, property) ||
    style.getPropertyValue(property).trim() === '' ||
    style.getPropertyPriority(property) !==
      (command.important ? 'important' : '')
  ) {
    throw new Error(`The browser rejected the ${property} override.`);
  }
  records.set(id, {
    kind: 'inline',
    publicOverride,
    originalPresent,
    originalValue,
    originalPriority,
  });
}

function setRuleOverride(
  element: Element,
  records: Map<string, OverrideRecord>,
  command: OverrideCommand,
  targetAttributeOriginal: { present: boolean, value: string },
): void {
  if (command.type !== 'set-rule') return;
  const property = normalizeCssProperty(command.property);
  const pseudoElement = command.pseudoElement.trim();
  if (property === '' || command.value.trim() === '' || pseudoElement === '') {
    throw new Error(
      'Rule overrides require a property, value, and pseudo-element.',
    );
  }
  validateCssValue(property, command.value);

  const id = ruleOverrideId(pseudoElement, property);
  const existing = records.get(id);
  if (existing != null) {
    restoreRecord(element, existing);
    records.delete(id);
  }

  element.setAttribute(RULE_TARGET_ATTRIBUTE, command.selectionId);
  const ruleSheet = createRuleOverrideSheet(
    element,
    command.selectionId,
    pseudoElement,
    property,
    command.value,
    command.important,
    command.conditions,
  );
  const publicOverride: RuleOverride = {
    id,
    kind: 'rule',
    contextKey: command.contextKey,
    property,
    value: command.value,
    important: command.important,
    conditions: command.conditions,
    pseudoElement,
  };
  if (command.sourceEntryKey != null) {
    publicOverride.sourceEntryKey = command.sourceEntryKey;
  }
  records.set(id, {
    kind: 'rule',
    publicOverride,
    ruleSheet,
    targetAttributeOriginalPresent: targetAttributeOriginal.present,
    targetAttributeOriginalValue: targetAttributeOriginal.value,
  });
}

function swapClassOverride(
  element: Element,
  records: Map<string, OverrideRecord>,
  command: OverrideCommand,
  previousClassRecord: ClassOverrideRecord | null,
): void {
  if (command.type !== 'swap-class') return;
  const originalClassName =
    previousClassRecord?.originalClassName ?? command.fromClassName.trim();
  const nextClassName = command.toClassName.trim();
  if (originalClassName === '' || nextClassName === '') {
    throw new Error('Class overrides require source and replacement classes.');
  }

  if (nextClassName === originalClassName) return;
  if (!element.classList.contains(originalClassName)) {
    throw new Error(
      `The selected element no longer has .${originalClassName}.`,
    );
  }

  const replacementClassPresent = element.classList.contains(nextClassName);
  const id = classOverrideId(originalClassName);
  const publicOverride: ClassOverride = {
    id,
    kind: 'class',
    contextKey: command.contextKey,
    property: command.property,
    value: command.value,
    important: command.important,
    conditions: command.conditions,
    className: nextClassName,
    originalClassName,
  };
  if (command.pseudoElement != null) {
    publicOverride.pseudoElement = command.pseudoElement;
  }
  if (command.sourceEntryKey != null) {
    publicOverride.sourceEntryKey = command.sourceEntryKey;
  }

  element.classList.remove(originalClassName);
  element.classList.add(nextClassName);
  if (
    element.classList.contains(originalClassName) ||
    !element.classList.contains(nextClassName)
  ) {
    throw new Error('The browser rejected the class override.');
  }
  records.set(id, {
    kind: 'class',
    publicOverride,
    originalClassName,
    originalClassPresent: true,
    replacementClassName: nextClassName,
    replacementClassPresent,
  });
}

function snapshotAttribute(element: Element, name: string) {
  return {
    present: element.hasAttribute(name),
    value: element.getAttribute(name) ?? '',
  };
}

function restoreAttribute(
  element: Element,
  name: string,
  snapshot: { present: boolean, value: string },
): void {
  if (snapshot.present) {
    element.setAttribute(name, snapshot.value);
  } else {
    element.removeAttribute(name);
  }
}

function snapshotInlineStyle(element: Element): {
  present: boolean,
  value: string,
} {
  const style = (element as any).style;
  return {
    present: element.hasAttribute('style'),
    value:
      style != null && typeof style.cssText === 'string'
        ? style.cssText
        : (element.getAttribute('style') ?? ''),
  };
}

function restoreInlineStyle(
  element: Element,
  snapshot: { present: boolean, value: string },
): void {
  if (!snapshot.present) {
    element.removeAttribute('style');
    return;
  }
  const style = (element as any).style;
  if (style != null && typeof style.cssText === 'string') {
    style.cssText = snapshot.value;
    if (!element.hasAttribute('style')) element.setAttribute('style', '');
  } else {
    element.setAttribute('style', snapshot.value);
  }
}

function getOriginalRuleTargetAttribute(
  element: Element,
  records: Map<string, OverrideRecord>,
): { present: boolean, value: string } {
  for (const record of records.values()) {
    if (record.kind === 'rule') {
      return {
        present: record.targetAttributeOriginalPresent,
        value: record.targetAttributeOriginalValue,
      };
    }
  }
  return snapshotAttribute(element, RULE_TARGET_ATTRIBUTE);
}

function hasRuleOverride(records: Map<string, OverrideRecord>): boolean {
  for (const record of records.values()) {
    if (record.kind === 'rule') return true;
  }
  return false;
}

function rollbackRuleSheets(
  previousRecords: Map<string, OverrideRecord>,
  nextRecords: Map<string, OverrideRecord>,
): void {
  for (const [id, record] of nextRecords) {
    if (record.kind !== 'rule' || previousRecords.get(id) === record) continue;
    removeRuleOverrideSheet(record.ruleSheet);
  }
  for (const [id, record] of previousRecords) {
    if (record.kind !== 'rule' || nextRecords.get(id) === record) continue;
    restoreRuleOverrideSheet(record.ruleSheet);
  }
}

export type AppliedOverrideMutation =
  | {
      ok: true,
      commit: () => void,
      rollback: () => void,
    }
  | {
      ok: false,
      code: 'mutation-failed' | 'stale-selection',
      message: string,
    };

export function applyOverrideMutation(
  command: OverrideCommand,
  selectedNode: mixed,
): AppliedOverrideMutation {
  const selection = getCurrentSelection();
  if (
    selection == null ||
    selection.id !== command.selectionId ||
    !isElementNode(selectedNode) ||
    !selection.element.isSameNode(selectedNode)
  ) {
    return {
      ok: false,
      code: 'stale-selection',
      message: 'The inspected selection changed. Refresh and try again.',
    };
  }

  const element = selection.element;
  const previousRecords = getOverrideRecordMap(element);
  const records = new Map(previousRecords);
  const classSnapshot = snapshotAttribute(element, 'class');
  const styleSnapshot = snapshotInlineStyle(element);
  const ruleTargetSnapshot = snapshotAttribute(element, RULE_TARGET_ATTRIBUTE);
  const ruleTargetOriginal = getOriginalRuleTargetAttribute(
    element,
    previousRecords,
  );
  let transactionOpen = true;
  const rollback = () => {
    if (!transactionOpen) return;
    transactionOpen = false;
    rollbackRuleSheets(previousRecords, records);
    restoreAttribute(element, 'class', classSnapshot);
    restoreInlineStyle(element, styleSnapshot);
    restoreAttribute(element, RULE_TARGET_ATTRIBUTE, ruleTargetSnapshot);
    setOverrideRecordMap(element, previousRecords);
  };

  try {
    if (command.type === 'remove') {
      const record = records.get(command.overrideId);
      if (record == null) {
        throw new Error('The override no longer exists.');
      }
      restoreRecord(element, record);
      records.delete(command.overrideId);
    } else {
      const existingClassRecord = findExistingClassRecord(records, command);
      const idsToReplace = [...command.replaceOverrideIds];
      if (
        existingClassRecord != null &&
        !idsToReplace.includes(existingClassRecord.publicOverride.id)
      ) {
        idsToReplace.push(existingClassRecord.publicOverride.id);
      }
      removeRecords(element, records, idsToReplace);
      if (command.type === 'set-inline') {
        setInlineOverride(element, records, command);
      } else if (command.type === 'set-rule') {
        setRuleOverride(element, records, command, ruleTargetOriginal);
      } else {
        swapClassOverride(element, records, command, existingClassRecord);
      }
    }

    if (hasRuleOverride(records)) {
      element.setAttribute(RULE_TARGET_ATTRIBUTE, command.selectionId);
    } else {
      restoreAttribute(element, RULE_TARGET_ATTRIBUTE, ruleTargetOriginal);
    }

    setOverrideRecordMap(element, records);
    return {
      ok: true,
      commit() {
        transactionOpen = false;
      },
      rollback,
    };
  } catch (error) {
    rollback();
    return {
      ok: false,
      code: 'mutation-failed',
      message: error instanceof Error ? error.message : 'Override failed.',
    };
  }
}
