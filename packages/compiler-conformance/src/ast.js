/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { parse } = require('@babel/parser');

// Node properties that describe how source was written rather than what it
// means: positions, comments, the raw text of literals (which encodes quote
// style and numeric formatting), and whether a property used the `{a}`
// shorthand for `{a: a}`. The `__proto__` shorthand is handled separately
// because it has different semantics from an explicit `__proto__` property.
const IGNORED_NODE_KEYS = new Set([
  'comments',
  'end',
  'errors',
  'extra',
  'innerComments',
  'leadingComments',
  'loc',
  'parenStart',
  'parenthesized',
  'range',
  'shorthand',
  'start',
  'tokens',
  'trailingComments',
]);

// Nodes whose `key` names a fixed property when `computed` is false. How that
// name is spelled — `{a: 1}`, `{'a': 1}`, `{1: x}` — is formatting, so the key
// is reduced to the name it denotes.
const KEYED_NODE_TYPES = new Set([
  'ClassMethod',
  'ClassProperty',
  'ObjectMethod',
  'ObjectProperty',
]);

function propertyKeyName(key) {
  if (key == null || typeof key !== 'object') {
    return null;
  }
  if (key.type === 'Identifier') {
    return key.name;
  }
  if (
    key.type === 'StringLiteral' ||
    key.type === 'NumericLiteral' ||
    key.type === 'BigIntLiteral'
  ) {
    return String(key.value);
  }
  return null;
}

function parseProgram(code, syntax, label) {
  try {
    return parse(code, {
      plugins: [...syntax],
      sourceType: 'module',
    }).program;
  } catch (error) {
    error.message = `Failed to parse ${label} as JavaScript: ${error.message}`;
    throw error;
  }
}

function canonicalize(value, parentType = null) {
  if (Array.isArray(value)) {
    return value.map((item) => canonicalize(item, parentType));
  }
  if (value == null || typeof value !== 'object') {
    return value;
  }
  const keyName =
    KEYED_NODE_TYPES.has(value.type) && value.computed === false
      ? propertyKeyName(value.key)
      : null;
  const shorthandIsSemantic =
    parentType === 'ObjectExpression' &&
    value.type === 'ObjectProperty' &&
    value.computed === false &&
    keyName === '__proto__';

  const output = {};
  for (const key of Object.keys(value).sort()) {
    if (
      (IGNORED_NODE_KEYS.has(key) &&
        !(key === 'shorthand' && shorthandIsSemantic)) ||
      value[key] === undefined
    ) {
      continue;
    }
    output[key] =
      key === 'key' && keyName != null
        ? { name: keyName, type: 'PropertyKey' }
        : canonicalize(value[key], value.type ?? parentType);
  }
  return output;
}

/**
 * Parses JavaScript and returns a canonical, JSON-serializable syntax tree with
 * all formatting information removed.
 */
function normalizeAst(code, syntax, label = 'JavaScript') {
  return canonicalize(parseProgram(code, syntax, label));
}

/**
 * Compares two JavaScript sources semantically. Whitespace, indentation, quote
 * style, semicolons and comments are ignored, so implementations only have to
 * agree on the program they emit and not on how they print it.
 */
function isJsEquivalent(expectedJs, actualJs, syntax) {
  if (typeof expectedJs !== 'string' || typeof actualJs !== 'string') {
    return expectedJs === actualJs;
  }
  const expectedAst = normalizeAst(expectedJs, syntax, 'the expected output');
  const actualAst = normalizeAst(actualJs, syntax, 'the actual output');
  return JSON.stringify(expectedAst) === JSON.stringify(actualAst);
}

module.exports = {
  isJsEquivalent,
  normalizeAst,
};
