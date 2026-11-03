/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

// $FlowFixMe[cannot-resolve-module]
import selectorParser from 'postcss-selector-parser';
import type { RuleCondition } from '../types.js';

export type SelectorCandidate = {
  className: string,
  selectorConditions: Array<RuleCondition>,
  contextSelector: string,
  isRootFallback: boolean,
  matchesClass: boolean,
  pseudoElement?: string,
};

const DYNAMIC_STATE_PSEUDOS = new Set([
  ':active',
  ':any-link',
  ':checked',
  ':disabled',
  ':enabled',
  ':focus',
  ':focus-visible',
  ':focus-within',
  ':hover',
  ':indeterminate',
  ':invalid',
  ':link',
  ':optional',
  ':placeholder-shown',
  ':read-only',
  ':read-write',
  ':required',
  ':target',
  ':user-invalid',
  ':valid',
  ':visited',
]);

const LEGACY_PSEUDO_ELEMENTS = new Set([
  ':after',
  ':before',
  ':first-letter',
  ':first-line',
]);

function isPseudoElement(node: any): boolean {
  return (
    node?.type === 'pseudo' &&
    (String(node.value).startsWith('::') ||
      LEGACY_PSEUDO_ELEMENTS.has(String(node.value)))
  );
}

function removePseudoElements(selector: any): any {
  const clone = selector.clone();
  clone.walkPseudos((node) => {
    if (isPseudoElement(node)) node.remove();
  });
  return clone;
}

function removeDynamicStatePseudos(selector: any): any {
  const clone = removePseudoElements(selector);
  clone.walkPseudos((node) => {
    if (DYNAMIC_STATE_PSEUDOS.has(String(node.value))) node.remove();
  });
  return clone;
}

function safelyMatches(element: Element, selector: string): boolean {
  if (selector.trim() === '') return false;
  try {
    return element.matches(selector);
  } catch {
    return false;
  }
}

function selectorMatchesElement(element: Element, selector: any): boolean {
  const withoutPseudoElement = removePseudoElements(selector).toString();
  if (safelyMatches(element, withoutPseudoElement)) return true;
  return safelyMatches(element, removeDynamicStatePseudos(selector).toString());
}

function selectorIsActive(element: Element, selector: any): boolean {
  return safelyMatches(element, removePseudoElements(selector).toString());
}

function getPseudoElement(selector: any): string | null {
  let value = null;
  selector.walkPseudos((node) => {
    if (value == null && isPseudoElement(node)) {
      value = node.toString();
    }
  });
  return value;
}

function getClassNodeAt(selector: any, targetIndex: number): any {
  let index = 0;
  let target = null;
  selector.walkClasses((node) => {
    if (index === targetIndex) target = node;
    index += 1;
  });
  return target;
}

function isInSameCompound(left: any, right: any): boolean {
  const parent = left?.parent;
  if (parent == null || parent !== right?.parent) return false;
  const nodes = parent.nodes;
  if (!Array.isArray(nodes)) return false;

  const leftIndex = nodes.indexOf(left);
  const rightIndex = nodes.indexOf(right);
  if (leftIndex === -1 || rightIndex === -1) return false;
  const start = Math.min(leftIndex, rightIndex) + 1;
  const end = Math.max(leftIndex, rightIndex);
  return !nodes.slice(start, end).some((node) => node?.type === 'combinator');
}

function removeRepeatedSpecificityClasses(selector: any, target: any): void {
  const className = String(target.value);
  const repeatedNodes = [];
  selector.walkClasses((node) => {
    if (
      node !== target &&
      String(node.value) === className &&
      isInSameCompound(node, target)
    ) {
      repeatedNodes.push(node);
    }
  });
  repeatedNodes.forEach((node) => node.remove());
}

function buildSelectorContext(selector: any, classIndex: number): string {
  const clone = removePseudoElements(selector);
  const target = getClassNodeAt(clone, classIndex);
  if (target == null) return '';
  removeRepeatedSpecificityClasses(clone, target);
  target.value = '__stylex_target__';
  const withPlaceholder = clone
    .toString()
    .replace(/\.__stylex_target__/g, '&')
    .trim();
  return withPlaceholder.startsWith('&')
    ? withPlaceholder.slice(1) || 'default'
    : withPlaceholder;
}

export function parseSelectorCandidates(
  selectorText: string,
  element: Element,
  elementClassSet: $ReadOnlySet<string>,
): Array<SelectorCandidate> {
  const root = selectorParser().astSync(selectorText);
  const candidates: Array<SelectorCandidate> = [];
  const isRootFallback = root.some(
    (selector) => selector.toString().trim() === ':root',
  );

  root.each((selector) => {
    const pseudoElement = getPseudoElement(selector);
    const selectorMatches = selectorMatchesElement(element, selector);
    const selectorActive = selectorIsActive(element, selector);
    const seenCandidates: Set<string> = new Set();
    let classIndex = 0;

    selector.walkClasses((classNode) => {
      const className = String(classNode.value);
      const contextSelector = buildSelectorContext(selector, classIndex);
      const candidateKey = `${className}\u0000${contextSelector}`;
      classIndex += 1;
      if (seenCandidates.has(candidateKey)) return;
      seenCandidates.add(candidateKey);
      const selectorConditions: Array<RuleCondition> = [];
      if (contextSelector !== '' && contextSelector !== 'default') {
        selectorConditions.push({
          kind: 'selector',
          text: contextSelector,
          active: selectorActive,
        });
      }
      const candidate: SelectorCandidate = {
        className,
        selectorConditions,
        contextSelector,
        isRootFallback,
        matchesClass: elementClassSet.has(className) && selectorMatches,
      };
      if (pseudoElement != null) candidate.pseudoElement = pseudoElement;
      candidates.push(candidate);
    });
  });

  return candidates;
}
