/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { RuleCondition } from '../types.js';

export const RULE_TARGET_ATTRIBUTE = 'data-stylex-devtools-target';

export type RuleOverrideSheet = {
  root: any,
  sheet: CSSStyleSheet,
};

function getAdoptedStyleSheets(root: any): Array<CSSStyleSheet> {
  const sheets = root?.adoptedStyleSheets;
  if (sheets == null || typeof sheets[Symbol.iterator] !== 'function') {
    throw new Error(
      'The selected element does not support constructed stylesheets.',
    );
  }
  return Array.from(sheets);
}

function setAdoptedStyleSheets(
  root: any,
  sheets: $ReadOnlyArray<CSSStyleSheet>,
): void {
  root.adoptedStyleSheets = [...sheets];
}

function getStyleRoot(element: Element): any {
  const root = element.getRootNode();
  getAdoptedStyleSheets(root);
  return root;
}

function getStyleSheetConstructor(element: Element): any {
  const constructor = element.ownerDocument.defaultView?.CSSStyleSheet;
  if (typeof constructor !== 'function') {
    throw new Error('Constructed stylesheets are not supported.');
  }
  return constructor;
}

function buildTargetSelector(
  element: Element,
  selectionId: string,
  pseudoElement: string,
  conditions: $ReadOnlyArray<RuleCondition>,
): string {
  let specificityId = `__stylex_devtools_specificity_${selectionId}`;
  while (element.id === specificityId) specificityId += '_';

  let selector = `[${RULE_TARGET_ATTRIBUTE}="${selectionId}"]:not(#${specificityId})`;
  for (const condition of conditions) {
    if (condition.kind !== 'selector') continue;
    const context = condition.text.trim();
    if (context === '' || context === 'default') continue;
    selector = context.includes('&')
      ? context.split('&').join(selector)
      : `${selector}${context}`;
  }
  return `${selector}${pseudoElement}`;
}

function buildRuleText(
  selector: string,
  conditions: $ReadOnlyArray<RuleCondition>,
): string {
  let ruleText = `${selector} {}`;
  for (let index = conditions.length - 1; index >= 0; index -= 1) {
    const condition = conditions[index];
    if (condition.kind !== 'at-rule') continue;
    const prelude = condition.text.trim();
    if (
      prelude === '' ||
      !prelude.startsWith('@') ||
      prelude.startsWith('@unknown')
    ) {
      continue;
    }
    ruleText = `${prelude} { ${ruleText} }`;
  }
  return ruleText;
}

function findStyleDeclaration(rules: CSSRuleList): CSSStyleDeclaration | null {
  for (const rule of Array.from(rules)) {
    const candidate = rule as any;
    if (candidate.style != null) return candidate.style;
    if (candidate.cssRules != null) {
      const nested = findStyleDeclaration(candidate.cssRules);
      if (nested != null) return nested;
    }
  }
  return null;
}

export function createRuleOverrideSheet(
  element: Element,
  selectionId: string,
  pseudoElement: string,
  property: string,
  value: string,
  important: boolean,
  conditions: $ReadOnlyArray<RuleCondition>,
): RuleOverrideSheet {
  if (!pseudoElement.startsWith(':')) {
    throw new Error(`Invalid pseudo-element selector: ${pseudoElement}`);
  }

  const root = getStyleRoot(element);
  const StyleSheet = getStyleSheetConstructor(element);
  const sheet: CSSStyleSheet = new StyleSheet();
  const selector = buildTargetSelector(
    element,
    selectionId,
    pseudoElement,
    conditions,
  );

  try {
    sheet.insertRule(buildRuleText(selector, conditions), 0);
    const declaration = findStyleDeclaration(sheet.cssRules);
    if (declaration == null) {
      throw new Error(`Could not create a rule for ${pseudoElement}.`);
    }
    declaration.setProperty(property, value, important ? 'important' : '');
    if (declaration.getPropertyValue(property).trim() === '') {
      throw new Error(`The browser rejected the ${property} override.`);
    }
    setAdoptedStyleSheets(root, [...getAdoptedStyleSheets(root), sheet]);
    return { root, sheet };
  } catch (error) {
    const sheets = getAdoptedStyleSheets(root);
    if (sheets.includes(sheet)) {
      setAdoptedStyleSheets(
        root,
        sheets.filter((candidate) => candidate !== sheet),
      );
    }
    throw error;
  }
}

export function removeRuleOverrideSheet({
  root,
  sheet,
}: RuleOverrideSheet): void {
  const sheets = getAdoptedStyleSheets(root);
  if (!sheets.includes(sheet)) return;
  setAdoptedStyleSheets(
    root,
    sheets.filter((candidate) => candidate !== sheet),
  );
}

export function restoreRuleOverrideSheet({
  root,
  sheet,
}: RuleOverrideSheet): void {
  const sheets = getAdoptedStyleSheets(root);
  if (sheets.includes(sheet)) return;
  setAdoptedStyleSheets(root, [...sheets, sheet]);
}
