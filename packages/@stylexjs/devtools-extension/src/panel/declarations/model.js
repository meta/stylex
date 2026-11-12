/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type {
  AtomicSuggestion,
  MatchedStylexClass,
  RuleCondition,
  StylexDeclaration,
  StylexOverride,
} from '../../types';
import { formatCssValue } from '../../utils/css';

export type PropertyGroup = {
  property: string,
  declarations: Array<StylexDeclaration>,
};

export type PseudoGroup = {
  pseudoElement: string,
  properties: Array<PropertyGroup>,
  variableClasses: Array<{
    className: string,
    properties: Array<PropertyGroup>,
  }>,
};

type PseudoAccumulator = {
  properties: Map<string, Array<StylexDeclaration>>,
  variableClasses: Map<string, Map<string, Array<StylexDeclaration>>>,
};

export function groupDeclarations(
  classes: $ReadOnlyArray<MatchedStylexClass>,
): Array<PseudoGroup> {
  const pseudoOrder = [];
  const groups: Map<string, PseudoAccumulator> = new Map();

  for (const matchedClass of classes) {
    for (const declaration of matchedClass.declarations) {
      const pseudo = declaration.pseudoElement ?? '';
      let pseudoGroup = groups.get(pseudo);
      if (pseudoGroup == null) {
        pseudoGroup = {
          properties: new Map(),
          variableClasses: new Map(),
        };
        groups.set(pseudo, pseudoGroup);
        pseudoOrder.push(pseudo);
      }

      let propertyMap: Map<
        string,
        Array<StylexDeclaration>,
      > = pseudoGroup.properties;
      if (declaration.property.startsWith('--')) {
        let variablePropertyMap = pseudoGroup.variableClasses.get(
          matchedClass.name,
        );
        if (variablePropertyMap == null) {
          variablePropertyMap = new Map();
          pseudoGroup.variableClasses.set(
            matchedClass.name,
            variablePropertyMap,
          );
        }
        propertyMap = variablePropertyMap;
      }
      const declarations = propertyMap.get(declaration.property) ?? [];
      declarations.push(declaration);
      propertyMap.set(declaration.property, declarations);
    }
  }

  pseudoOrder.sort((left, right) => {
    if (left === '') return -1;
    if (right === '') return 1;
    return 0;
  });
  return pseudoOrder.map((pseudoElement) => {
    const group = groups.get(pseudoElement);
    return {
      pseudoElement,
      properties: Array.from(
        group?.properties ?? [],
        ([property, declarations]) => ({ property, declarations }),
      ),
      variableClasses: Array.from(
        group?.variableClasses ?? [],
        ([className, propertyMap]) => ({
          className,
          properties: Array.from(propertyMap, ([property, declarations]) => ({
            property,
            declarations,
          })),
        }),
      ),
    };
  });
}

export function formatConditions(
  conditions: $ReadOnlyArray<RuleCondition>,
): string {
  const visibleConditions = getVisibleConditions(conditions);
  if (visibleConditions.length === 0) {
    return 'default';
  }
  return visibleConditions.map(({ text }) => text).join(' · ');
}

export function conditionsAreActive(
  conditions: $ReadOnlyArray<RuleCondition>,
): boolean | null {
  const visibleConditions = getVisibleConditions(conditions);
  if (visibleConditions.some(({ active }) => active === false)) {
    return false;
  }
  if (visibleConditions.every(({ active }) => active === true)) {
    return true;
  }
  return null;
}

function getVisibleConditions(
  conditions: $ReadOnlyArray<RuleCondition>,
): Array<RuleCondition> {
  return conditions.filter(({ kind, text }) => {
    const normalizedText = text.trim().toLowerCase();
    const isLayer =
      kind === 'at-rule' &&
      (normalizedText === '@layer' || normalizedText.startsWith('@layer '));
    return !isLayer;
  });
}

export function getSuggestionValues(
  suggestions: $ReadOnlyArray<AtomicSuggestion>,
): Array<string> {
  const values: Array<string> = [];
  for (const suggestion of suggestions) {
    const value = formatCssValue(suggestion.value, suggestion.important);
    if (!values.includes(value)) {
      values.push(value);
    }
  }
  return values;
}

export function findSuggestion(
  suggestions: $ReadOnlyArray<AtomicSuggestion>,
  formattedValue: string,
): ?AtomicSuggestion {
  return suggestions.find(
    (suggestion) =>
      formatCssValue(suggestion.value, suggestion.important) === formattedValue,
  );
}

export function findDisplayedOverride(
  declaration: StylexDeclaration,
  overrides: $ReadOnlyArray<StylexOverride>,
): ?StylexOverride {
  return overrides.find(
    (override) =>
      override.property === declaration.property &&
      (override.contextKey === declaration.contextKey ||
        override.sourceEntryKey === declaration.key ||
        (override.kind === 'class' &&
          override.className === declaration.className)),
  );
}

export function getReplacementOverrideIds(
  declaration: StylexDeclaration,
  overrides: $ReadOnlyArray<StylexOverride>,
): Array<string> {
  return overrides
    .filter(
      (override) =>
        (override.kind === 'inline' &&
          override.property === declaration.property) ||
        (override.kind === 'rule' &&
          override.property === declaration.property &&
          override.pseudoElement === (declaration.pseudoElement ?? '')) ||
        (override.kind === 'class' &&
          (override.className === declaration.className ||
            override.originalClassName === declaration.className ||
            override.sourceEntryKey === declaration.key)),
    )
    .map(({ id }) => id);
}
