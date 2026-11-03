/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

export type StylexSource = {
  raw: string,
  file: string,
  line: number | null,
};

export type RuleCondition = {
  kind: 'at-rule' | 'selector',
  text: string,
  active: boolean | null,
};

export type StylexDeclaration = {
  key: string,
  contextKey: string,
  property: string,
  value: string,
  important: boolean,
  conditions: Array<RuleCondition>,
  pseudoElement?: string,
  className: string,
};

export type MatchedStylexClass = {
  name: string,
  declarations: Array<StylexDeclaration>,
};

export type AtomicSuggestion = {
  className: string,
  property: string,
  value: string,
  important: boolean,
  pseudoElement?: string,
};

export type InlineOverride = {
  id: string,
  kind: 'inline',
  contextKey: string,
  property: string,
  value: string,
  important: boolean,
  conditions: Array<RuleCondition>,
  sourceEntryKey?: string,
};

export type RuleOverride = {
  id: string,
  kind: 'rule',
  contextKey: string,
  property: string,
  value: string,
  important: boolean,
  conditions: Array<RuleCondition>,
  pseudoElement: string,
  sourceEntryKey?: string,
};

export type ClassOverride = {
  id: string,
  kind: 'class',
  contextKey: string,
  property: string,
  value: string,
  important: boolean,
  conditions: Array<RuleCondition>,
  pseudoElement?: string,
  className: string,
  originalClassName: string,
  sourceEntryKey?: string,
};

export type StylexOverride = InlineOverride | RuleOverride | ClassOverride;

type OverrideCommandBase = {
  selectionId: string,
  contextKey: string,
  property: string,
  value: string,
  important: boolean,
  conditions: Array<RuleCondition>,
  sourceEntryKey?: string,
  replaceOverrideIds: Array<string>,
};

export type SetInlineOverrideCommand = {
  ...OverrideCommandBase,
  type: 'set-inline',
};

export type SwapClassOverrideCommand = {
  ...OverrideCommandBase,
  type: 'swap-class',
  fromClassName: string,
  toClassName: string,
  pseudoElement?: string,
};

export type SetRuleOverrideCommand = {
  ...OverrideCommandBase,
  type: 'set-rule',
  pseudoElement: string,
};

export type RemoveOverrideCommand = {
  type: 'remove',
  selectionId: string,
  overrideId: string,
};

export type OverrideCommand =
  | SetInlineOverrideCommand
  | SetRuleOverrideCommand
  | SwapClassOverrideCommand
  | RemoveOverrideCommand;

export type DebugWarning = {
  code: string,
  message: string,
  count?: number,
};

export type StylexDebugData = Readonly<{
  selectionId: string,
  selectionState: 'element' | 'none' | 'non-element',
  element: {
    tagName: string,
  },
  sources: Array<StylexSource>,
  computed: { [string]: { [string]: string, ... }, ... },
  resolvedVariables: { [string]: string, ... },
  suggestions: { [string]: Array<AtomicSuggestion>, ... },
  overrides: Array<StylexOverride>,
  matched: {
    classes: Array<MatchedStylexClass>,
  },
  warnings: Array<DebugWarning>,
}>;

export type OverrideMutationResult =
  | { ok: true, data: StylexDebugData }
  | {
      ok: false,
      code: 'mutation-failed' | 'stale-selection',
      message: string,
    };

export type SourcePreview = {
  url: string,
  snippet: string,
};
