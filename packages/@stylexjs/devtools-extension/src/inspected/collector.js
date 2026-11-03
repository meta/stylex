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
  DebugWarning,
  MatchedStylexClass,
  RuleCondition,
  StylexDebugData,
  StylexDeclaration,
  StylexOverride,
  StylexSource,
} from '../types';

import { readAuthoredDeclarations } from './declarationAnalysis';
import { parseSelectorCandidates } from './selectorAnalysis';
import { findCssVariableReferences } from '../utils/cssVariables';
import { normalizeCssProperty } from '../utils/css';
import {
  clearSelection,
  getPublicOverrides,
  isElementNode,
  registerSelection,
} from './runtimeState';

type CollectionState = {
  classDeclarations: Map<string, Map<string, StylexDeclaration>>,
  suggestions: Map<string, Map<string, AtomicSuggestion>>,
  propertiesByPseudo: Map<string, Set<string>>,
  rootFallbackClasses: Set<string>,
  skippedStylesheets: number,
  unsupportedSelectors: number,
};

function createEmptyData(
  selectionState: 'none' | 'non-element',
): StylexDebugData {
  clearSelection();
  return {
    selectionId: '',
    selectionState,
    element: { tagName: '—' },
    sources: [],
    computed: {},
    resolvedVariables: {},
    suggestions: {},
    overrides: [],
    matched: { classes: [] },
    warnings: [],
  };
}

function resolveTarget(target: mixed): mixed {
  if (typeof target === 'string') {
    try {
      return document.querySelector(target);
    } catch {
      return null;
    }
  }
  return target;
}

function parseSourceMetadata(element: Element): Array<StylexSource> {
  const value = element.getAttribute('data-style-src');
  if (value == null || value.trim() === '') {
    return [];
  }

  return value
    .split(';')
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const match = entry.match(/^(.*):(\d+)$/);
      if (match == null) {
        return { raw: entry, file: entry, line: null };
      }
      return { raw: entry, file: match[1], line: Number(match[2]) };
    });
}

function getAtRuleCondition(rule: CSSRule): ?RuleCondition {
  if (typeof CSSMediaRule !== 'undefined' && rule instanceof CSSMediaRule) {
    const text = `@media ${rule.conditionText}`;
    let active = null;
    try {
      active = window.matchMedia(rule.conditionText).matches;
    } catch {}
    return { kind: 'at-rule', text, active };
  }

  if (
    typeof CSSSupportsRule !== 'undefined' &&
    rule instanceof CSSSupportsRule
  ) {
    const text = `@supports ${rule.conditionText}`;
    let active = null;
    try {
      const cssApi = (globalThis as any).CSS;
      active = cssApi?.supports(rule.conditionText) ?? null;
    } catch {}
    return { kind: 'at-rule', text, active };
  }

  const candidate = rule as any;
  if (typeof candidate.name === 'string') {
    return { kind: 'at-rule', text: `@layer ${candidate.name}`, active: true };
  }
  if (typeof candidate.conditionText === 'string') {
    return {
      kind: 'at-rule',
      text: `@unknown ${candidate.conditionText}`,
      active: null,
    };
  }

  const cssText =
    typeof candidate.cssText === 'string' ? candidate.cssText : '';
  const openingBrace = cssText.indexOf('{');
  const prelude =
    openingBrace === -1 ? cssText : cssText.slice(0, openingBrace);
  const text = prelude.trim();
  return text.startsWith('@') ? { kind: 'at-rule', text, active: null } : null;
}

function getContextKey(
  property: string,
  pseudoElement: ?string,
  conditions: $ReadOnlyArray<RuleCondition>,
): string {
  return JSON.stringify([
    normalizeCssProperty(property),
    pseudoElement ?? '',
    ...conditions.map(({ kind, text }) => `${kind}:${text}`),
  ]);
}

function getDeclarationKey(declaration: StylexDeclaration): string {
  return JSON.stringify([
    declaration.className,
    declaration.property,
    declaration.value,
    declaration.important,
    declaration.pseudoElement ?? '',
    ...declaration.conditions.map(({ kind, text }) => `${kind}:${text}`),
  ]);
}

function addSuggestion(
  state: CollectionState,
  contextKey: string,
  suggestion: AtomicSuggestion,
): void {
  let contextSuggestions = state.suggestions.get(contextKey);
  if (contextSuggestions == null) {
    contextSuggestions = new Map();
    state.suggestions.set(contextKey, contextSuggestions);
  }
  const key = JSON.stringify([
    suggestion.className,
    suggestion.value,
    suggestion.important,
  ]);
  contextSuggestions.set(key, suggestion);
}

function addMatchedDeclaration(
  state: CollectionState,
  declaration: StylexDeclaration,
): void {
  let classDeclarations = state.classDeclarations.get(declaration.className);
  if (classDeclarations == null) {
    classDeclarations = new Map();
    state.classDeclarations.set(declaration.className, classDeclarations);
  }
  classDeclarations.set(getDeclarationKey(declaration), declaration);

  const pseudo = declaration.pseudoElement ?? '';
  let properties = state.propertiesByPseudo.get(pseudo);
  if (properties == null) {
    properties = new Set();
    state.propertiesByPseudo.set(pseudo, properties);
  }
  properties.add(declaration.property);
}

function collectStyleRule(
  rule: CSSStyleRule,
  element: Element,
  elementClasses: Set<string>,
  atRuleConditions: $ReadOnlyArray<RuleCondition>,
  state: CollectionState,
): void {
  const declarations = readAuthoredDeclarations(rule.style);
  if (declarations.length === 0) {
    return;
  }

  let candidates;
  try {
    candidates = parseSelectorCandidates(
      rule.selectorText,
      element,
      elementClasses,
    );
  } catch {
    state.unsupportedSelectors += 1;
    return;
  }

  for (const candidate of candidates) {
    for (const cssDeclaration of declarations) {
      if (
        cssDeclaration.property.startsWith('--') &&
        candidate.contextSelector.trim().toLowerCase() === ':root'
      ) {
        continue;
      }
      const conditions = [...atRuleConditions, ...candidate.selectorConditions];
      const contextKey = getContextKey(
        cssDeclaration.property,
        candidate.pseudoElement,
        conditions,
      );

      if (declarations.length === 1) {
        addSuggestion(state, contextKey, {
          className: candidate.className,
          property: cssDeclaration.property,
          value: cssDeclaration.value,
          important: cssDeclaration.important,
          pseudoElement: candidate.pseudoElement,
        });
      }

      if (!candidate.matchesClass) {
        continue;
      }
      if (candidate.isRootFallback) {
        state.rootFallbackClasses.add(candidate.className);
      }

      addMatchedDeclaration(state, {
        key: getDeclarationKey({
          key: '',
          contextKey,
          className: candidate.className,
          property: cssDeclaration.property,
          value: cssDeclaration.value,
          important: cssDeclaration.important,
          conditions,
          pseudoElement: candidate.pseudoElement,
        }),
        contextKey,
        className: candidate.className,
        property: cssDeclaration.property,
        value: cssDeclaration.value,
        important: cssDeclaration.important,
        conditions,
        pseudoElement: candidate.pseudoElement,
      });
    }
  }
}

function walkRules(
  rules: CSSRuleList,
  element: Element,
  elementClasses: Set<string>,
  conditions: $ReadOnlyArray<RuleCondition>,
  state: CollectionState,
): void {
  for (const rule of Array.from(rules)) {
    const candidate = rule as any;
    if (typeof candidate.selectorText === 'string' && candidate.style) {
      collectStyleRule(candidate, element, elementClasses, conditions, state);
      continue;
    }

    const nestedRules = candidate.cssRules;
    if (nestedRules == null) {
      continue;
    }
    const condition = getAtRuleCondition(rule);
    walkRules(
      nestedRules,
      element,
      elementClasses,
      condition == null ? conditions : [...conditions, condition],
      state,
    );
  }
}

function collectRules(element: Element): CollectionState {
  const state: CollectionState = {
    classDeclarations: new Map(),
    suggestions: new Map(),
    propertiesByPseudo: new Map(),
    rootFallbackClasses: new Set(),
    skippedStylesheets: 0,
    unsupportedSelectors: 0,
  };
  const elementClasses = new Set(Array.from(element.classList));

  for (const stylesheet of Array.from(document.styleSheets)) {
    let rules;
    try {
      rules = (stylesheet as any).cssRules;
    } catch {
      state.skippedStylesheets += 1;
      continue;
    }
    if (rules != null) {
      walkRules(rules, element, elementClasses, [], state);
    }
  }
  return state;
}

function collectReferencedVariableNames(
  classes: $ReadOnlyArray<MatchedStylexClass>,
  overrides: $ReadOnlyArray<StylexOverride>,
): Set<string> {
  const names: Set<string> = new Set();
  const values = [
    ...classes.flatMap(({ declarations }) =>
      declarations.map(({ value }) => value),
    ),
    ...overrides.map(({ value }) => value),
  ];

  for (const value of values) {
    for (const { name } of findCssVariableReferences(value)) {
      names.add(name);
    }
  }
  return names;
}

type ComputedData = {
  properties: { [string]: { [string]: string } },
  variables: { [string]: string },
};

function collectComputedData(
  element: Element,
  propertiesByPseudo: Map<string, Set<string>>,
  variableNames: Set<string>,
): ComputedData {
  const propertiesByContext: { [string]: { [string]: string } } = {};
  const variables: { [string]: string } = {};
  const pseudos = new Set(propertiesByPseudo.keys());
  if (variableNames.size > 0) pseudos.add('');

  for (const pseudo of pseudos) {
    const properties = propertiesByPseudo.get(pseudo) ?? new Set();
    try {
      const declaration = window.getComputedStyle(element, pseudo || null);
      const values: { [string]: string } = {};
      for (const property of properties) {
        values[property] = declaration.getPropertyValue(property);
      }
      propertiesByContext[pseudo] = values;
      if (pseudo === '') {
        for (const name of variableNames) {
          variables[name] = declaration.getPropertyValue(name).trim();
        }
      }
    } catch {
      propertiesByContext[pseudo] = {};
      if (pseudo === '') {
        for (const name of variableNames) variables[name] = '';
      }
    }
  }
  return { properties: propertiesByContext, variables };
}

function variableKey(property: string, pseudoElement?: string): string {
  return JSON.stringify([normalizeCssProperty(property), pseudoElement ?? '']);
}

function getThemedVariableKeys(
  classDeclarations: Map<string, Map<string, StylexDeclaration>>,
  rootFallbackClasses: Set<string>,
): Set<string> {
  const themedVariableKeys: Set<string> = new Set();
  for (const [className, declarations] of classDeclarations) {
    if (rootFallbackClasses.has(className)) continue;
    for (const declaration of declarations.values()) {
      if (!declaration.property.startsWith('--')) continue;
      themedVariableKeys.add(
        variableKey(declaration.property, declaration.pseudoElement),
      );
    }
  }
  return themedVariableKeys;
}

function formatMatchedClasses(
  element: Element,
  classDeclarations: Map<string, Map<string, StylexDeclaration>>,
  rootFallbackClasses: Set<string>,
  themedVariableKeys: Set<string>,
): Array<MatchedStylexClass> {
  const classes = [];
  for (const className of Array.from(element.classList)) {
    const declarations = classDeclarations.get(className);
    if (declarations != null && declarations.size > 0) {
      const visibleDeclarations = Array.from(declarations.values()).filter(
        (declaration) =>
          !(
            rootFallbackClasses.has(className) &&
            declaration.property.startsWith('--') &&
            themedVariableKeys.has(
              variableKey(declaration.property, declaration.pseudoElement),
            )
          ),
      );
      if (visibleDeclarations.length === 0) continue;
      classes.push({
        name: className,
        declarations: visibleDeclarations,
      });
    }
  }
  return classes;
}

function formatSuggestions(
  suggestions: Map<string, Map<string, AtomicSuggestion>>,
  matchedContextKeys: Set<string>,
  rootFallbackClasses: Set<string>,
  themedVariableKeys: Set<string>,
): { [string]: Array<AtomicSuggestion> } {
  const result: { [string]: Array<AtomicSuggestion> } = {};
  for (const contextKey of matchedContextKeys) {
    const entries = suggestions.get(contextKey);
    if (entries == null) continue;
    const visibleEntries = Array.from(entries.values()).filter(
      (suggestion) =>
        !(
          rootFallbackClasses.has(suggestion.className) &&
          suggestion.property.startsWith('--') &&
          themedVariableKeys.has(
            variableKey(suggestion.property, suggestion.pseudoElement),
          )
        ),
    );
    if (visibleEntries.length > 0) result[contextKey] = visibleEntries;
  }
  return result;
}

function formatWarnings(state: CollectionState): Array<DebugWarning> {
  const warnings: Array<DebugWarning> = [];
  if (state.skippedStylesheets > 0) {
    warnings.push({
      code: 'stylesheet-inaccessible',
      message: `${state.skippedStylesheets} stylesheet${
        state.skippedStylesheets === 1 ? '' : 's'
      } could not be inspected.`,
      count: state.skippedStylesheets,
    });
  }
  if (state.unsupportedSelectors > 0) {
    warnings.push({
      code: 'selector-unsupported',
      message: `${state.unsupportedSelectors} selector${
        state.unsupportedSelectors === 1 ? '' : 's'
      } could not be parsed.`,
      count: state.unsupportedSelectors,
    });
  }
  return warnings;
}

export function collectStylexDebugData(target: mixed): StylexDebugData {
  const selected = resolveTarget(target);
  if (selected == null) {
    return createEmptyData('none');
  }
  if (!isElementNode(selected)) {
    return createEmptyData('non-element');
  }

  const element = selected;
  const selectionId = registerSelection(element);
  const state = collectRules(element);
  const overrides = getPublicOverrides(element);
  for (const override of overrides) {
    const pseudoElement = override.pseudoElement ?? '';
    let properties = state.propertiesByPseudo.get(pseudoElement);
    if (properties == null) {
      properties = new Set();
      state.propertiesByPseudo.set(pseudoElement, properties);
    }
    properties.add(override.property);
  }
  const themedVariableKeys = getThemedVariableKeys(
    state.classDeclarations,
    state.rootFallbackClasses,
  );
  const matchedClasses = formatMatchedClasses(
    element,
    state.classDeclarations,
    state.rootFallbackClasses,
    themedVariableKeys,
  );
  const matchedContextKeys = new Set(
    matchedClasses.flatMap(({ declarations }) =>
      declarations.map(({ contextKey }) => contextKey),
    ),
  );
  const computedData = collectComputedData(
    element,
    state.propertiesByPseudo,
    collectReferencedVariableNames(matchedClasses, overrides),
  );

  return {
    selectionId,
    selectionState: 'element',
    element: { tagName: element.tagName.toLowerCase() },
    sources: parseSourceMetadata(element),
    computed: computedData.properties,
    resolvedVariables: computedData.variables,
    suggestions: formatSuggestions(
      state.suggestions,
      matchedContextKeys,
      state.rootFallbackClasses,
      themedVariableKeys,
    ),
    overrides,
    matched: {
      classes: matchedClasses,
    },
    warnings: formatWarnings(state),
  };
}
