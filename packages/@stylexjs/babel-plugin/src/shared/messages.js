/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

// This file contains constants to be used within Error messages.
// The URLs within will eventually be replaced by links to the documentation website for Stylex.

export const illegalArgumentLength = (fn: string, argLength: number): string =>
  `${fn}() should have ${argLength} argument${argLength === 1 ? '' : 's'}.`;
export const nonStaticValue = (fn: string): string =>
  `Only static values are allowed inside of a ${fn}() call.`;
export const nonStyleObject = (fn: string): string =>
  `${fn}() can only accept an object.`;
export const nonExportNamedDeclaration = (fn: string): string =>
  `The return value of ${fn}() must be bound to a named export.`;
export const unboundCallValue = (fn: string): string =>
  `${fn}() calls must be bound to a bare variable.`;
export const cannotGenerateHash = (fn: string): string =>
  `Unable to generate hash for ${fn}(). Check that the file has a valid extension and that unstable_moduleResolution is configured.`;
export const invalidDefineVarsFunctionValue = (): string =>
  'Function values in defineVars() must be zero-argument and return a static value supported by defineVars().';
export const cyclicDefineVarsReference = (cycle: string): string =>
  `Cyclic same-group references in defineVars() are not allowed: ${cycle}.`;
export const unknownDefineVarsReference = (
  key: string,
  dependency: string,
): string =>
  `Unknown same-group reference "${dependency}" found while resolving "${key}" in defineVars().`;

export const DUPLICATE_CONDITIONAL =
  'The same pseudo selector or at-rule cannot be used more than once.';
export const ESCAPED_STYLEX_VALUE = 'Escaping a create() value is not allowed.';
export const ILLEGAL_NESTED_PSEUDO =
  "Pseudo objects can't be nested more than one level deep.";
export const ILLEGAL_PROP_VALUE =
  'A style value can only contain an array, string or number.';
export const ILLEGAL_PROP_ARRAY_VALUE =
  'A style array value can only contain strings or numbers.';
export const ILLEGAL_NAMESPACE_VALUE = 'A StyleX namespace must be an object.';
export const INVALID_PSEUDO = 'Invalid pseudo selector, not on the whitelist.';
export const INVALID_PSEUDO_OR_AT_RULE = 'Invalid pseudo or at-rule.';
export const INVALID_MEDIA_QUERY_SYNTAX = 'Invalid media query syntax.';
export const LINT_UNCLOSED_FUNCTION = 'Rule contains an unclosed function';
export const LINT_UNCLOSED_STRING = 'Rule contains an unclosed string';
export const LOCAL_ONLY =
  'The return value of create() should not be exported.';
export const NON_OBJECT_KEYFRAME =
  'Every frame within a keyframes() call must be an object.';
export const NON_CONTIGUOUS_VARS =
  'All variables passed to firstThatWorks() must be contiguous.';
export const NO_OBJECT_SPREADS =
  'Object spreads are not allowed in create() calls.';
export const ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS =
  'Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.';
export const ONLY_TOP_LEVEL =
  'create() is only allowed at the root of a program.';
export const UNKNOWN_PROP_KEY = 'Unknown property key';

export const POSITION_TRY_INVALID_PROPERTY =
  'Invalid property in `positionTry()` call. It may only contain, positionAnchor, positionArea, inset properties (top, left, insetInline etc.), margin properties, size properties (height, inlineSize, etc.), and self-alignment properties (alignSelf, justifySelf, placeSelf)';

export const VIEW_TRANSITION_CLASS_INVALID_PROPERTY =
  'Invalid property in `viewTransitionClass()` call. It may only contain group, imagePair, old, and new properties';
