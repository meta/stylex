/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Parsing with the file's own grammar and parser.

const { t } = require('./babel-deps');

// Parser overrides (notably Hermes) produce the file's AST.
function parserOverrides(state) {
  return (state.file.opts.plugins || []).filter(
    (plugin) => typeof plugin.parserOverride === 'function',
  );
}

// Node types that need the flow or typescript parser plugin to reparse.
const FLOW_TYPES = new Set(t.FLIPPED_ALIAS_KEYS.Flow);
const TYPESCRIPT_TYPES = new Set(t.FLIPPED_ALIAS_KEYS.TypeScript);

function syntaxOf(node, syntax) {
  const type = node.type;
  if (type.startsWith('JSX')) syntax.jsx = true;
  if (FLOW_TYPES.has(type)) syntax.flow = true;
  if (TYPESCRIPT_TYPES.has(type)) syntax.typescript = true;
}

// `known` is the syntax of the current program when the caller collected it.
function grammarOptions(state, known = null) {
  const parserOpts = { ...state.file.opts.parserOpts };
  delete parserOpts.sourceFileName;
  const plugins = [...(parserOpts.plugins || [])];
  const syntax = known ?? { jsx: false, flow: false, typescript: false };
  if (known == null)
    t.traverseFast(state.file.ast.program, (node) => syntaxOf(node, syntax));
  if (parserOverrides(state).length > 0 && !syntax.typescript) {
    syntax.jsx = true;
    syntax.flow = true;
  }
  const has = (name) =>
    plugins.some(
      (plugin) => (Array.isArray(plugin) ? plugin[0] : plugin) === name,
    );
  if (syntax.typescript && !has('typescript')) plugins.push('typescript');
  if (syntax.flow && !syntax.typescript && !has('flow'))
    plugins.push(['flow', { all: true }]);
  if (syntax.jsx && !has('jsx')) plugins.push('jsx');
  parserOpts.plugins = plugins;
  return parserOpts;
}

module.exports = {
  grammarOptions,
  parserOverrides,
  syntaxOf,
};
