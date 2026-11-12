/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Which files and calls belong to StyleX.

const { t } = require('./babel-deps');
const { syntaxOf } = require('./grammar');

const BUILT_IN_SOURCES = ['@stylexjs/stylex', 'stylex', '@stylexjs/atoms'];

function importSources(options) {
  const sources = new Map(BUILT_IN_SOURCES.map((source) => [source, null]));
  for (const entry of Array.isArray(options.importSources)
    ? options.importSources
    : []) {
    if (typeof entry === 'string') sources.set(entry, null);
    else if (entry != null && typeof entry.from === 'string')
      sources.set(entry.from, typeof entry.as === 'string' ? entry.as : null);
  }
  return sources;
}

function sxPropName(options) {
  return options.sxPropName === false
    ? null
    : typeof options.sxPropName === 'string'
      ? options.sxPropName
      : 'sx';
}

function isRequireOf(node, sources) {
  return (
    node != null &&
    node.type === 'CallExpression' &&
    node.callee.type === 'Identifier' &&
    node.callee.name === 'require' &&
    node.arguments.length === 1 &&
    node.arguments[0].type === 'StringLiteral' &&
    sources.has(node.arguments[0].value)
  );
}

// Whether source text provably spells no StyleX import source or sx attribute.
// An escape that could stand for a character of one makes it give up.
const CONTROL_ESCAPES = 'nrtbfv';
function cannotSpellStyleX(code, s) {
  if (s.sx != null && code.includes(s.sx)) return false;
  const needles = [...s.sources.keys()];
  // `rewriteAliases` rewrites theme imports even in a file without StyleX.
  if (s.rewriteAliases) needles.push('.stylex');
  if (needles.some((needle) => code.includes(needle))) return false;
  const controls = needles.some((needle) => /[\n\r\t\b\f\v]/.test(needle));
  for (let i = code.indexOf('\\'); i !== -1; i = code.indexOf('\\', i + 2)) {
    const next = code[i + 1];
    if (next === undefined || /[ux0-9\r\n\u2028\u2029]/.test(next))
      return false;
    if (CONTROL_ESCAPES.includes(next)) {
      if (controls) return false;
    } else if (needles.some((needle) => needle.includes(next))) return false;
  }
  return true;
}

// Also collects the program's syntax and, optionally, its local declarations.
function mayUseStyleX(
  program,
  options,
  syntax,
  declarations = null,
  visit = null,
) {
  const sources = importSources(options);
  const sx = sxPropName(options);
  let relevant = false;
  t.traverseFast(program, (node) => {
    syntaxOf(node, syntax);
    if (declarations != null) indexDeclaration(node, declarations);
    if (visit != null) visit(node);
    if (relevant) return;
    if (
      node.type === 'ImportDeclaration' &&
      node.importKind !== 'type' &&
      node.importKind !== 'typeof' &&
      sources.has(node.source.value)
    )
      relevant = true;
    if (isRequireOf(node, sources)) relevant = true;
    // `rewriteAliases` rewrites theme imports even in a file without StyleX.
    if (
      options?.rewriteAliases === true &&
      node.type === 'ImportDeclaration' &&
      typeof node.source.value === 'string' &&
      node.source.value.includes('.stylex')
    )
      relevant = true;
    if (
      sx != null &&
      node.type === 'JSXOpeningElement' &&
      node.name.type === 'JSXIdentifier' &&
      node.name.name[0] === node.name.name[0].toLowerCase() &&
      node.attributes.some(
        (attr) =>
          attr.type === 'JSXAttribute' &&
          attr.name.type === 'JSXIdentifier' &&
          attr.name.name === sx,
      )
    )
      relevant = true;
  });
  return relevant;
}

function indexDeclaration(node, declarations) {
  let names = null;
  if (node.type === 'VariableDeclarator')
    names =
      node.id.type === 'Identifier'
        ? [node.id.name]
        : Object.keys(t.getBindingIdentifiers(node.id));
  else if (
    (node.type === 'FunctionDeclaration' || node.type === 'ClassDeclaration') &&
    node.id != null
  )
    names = [node.id.name];
  if (names == null) return;
  for (const name of names) {
    const list = declarations.get(name);
    if (list == null) declarations.set(name, [node]);
    else list.push(node);
  }
}

// Local declarations a subtree may transitively read, ignoring shadowing.
function dependencies(root, declarations) {
  const found = new Set();
  const seen = new Set();
  const pending = [root];
  while (pending.length > 0) {
    t.traverseFast(pending.pop(), (node) => {
      if (node.type !== 'Identifier' || seen.has(node.name)) return;
      seen.add(node.name);
      for (const declaration of declarations.get(node.name) || [])
        if (!found.has(declaration)) {
          found.add(declaration);
          pending.push(declaration);
        }
    });
  }
  found.delete(root);
  return [...found];
}

// Local names through which top-level imports and requires reach StyleX.
// `namespaces` hold the module object, `named` map a local to its export.
function stylexNames(program, sources) {
  const namespaces = new Set();
  const named = new Map();
  for (const statement of program.body) {
    if (
      statement.type === 'ImportDeclaration' &&
      statement.importKind !== 'type' &&
      statement.importKind !== 'typeof' &&
      sources.has(statement.source.value)
    ) {
      const as = sources.get(statement.source.value);
      for (const specifier of statement.specifiers) {
        if (specifier.type === 'ImportSpecifier') {
          if (specifier.importKind === 'type') continue;
          const imported =
            specifier.imported.type === 'Identifier'
              ? specifier.imported.name
              : specifier.imported.value;
          if (as != null && imported === as)
            namespaces.add(specifier.local.name);
          else named.set(specifier.local.name, imported);
        } else namespaces.add(specifier.local.name);
      }
    }
    if (statement.type === 'VariableDeclaration') {
      for (const declarator of statement.declarations) {
        let init = declarator.init;
        // `_interopRequireWildcard(require(...))` and similar wrappers.
        while (
          init != null &&
          init.type === 'CallExpression' &&
          !isRequireOf(init, sources) &&
          init.arguments.length > 0
        )
          init = init.arguments[0];
        if (!isRequireOf(init, sources)) continue;
        if (declarator.id.type === 'Identifier')
          namespaces.add(declarator.id.name);
        else if (declarator.id.type === 'ObjectPattern') {
          for (const property of declarator.id.properties) {
            if (
              property.type === 'ObjectProperty' &&
              property.key.type === 'Identifier' &&
              property.value.type === 'Identifier'
            )
              named.set(property.value.name, property.key.name);
          }
        }
      }
    }
  }
  return {
    namespaces,
    named,
    empty: namespaces.size === 0 && named.size === 0,
  };
}

// The StyleX API a call uses (`create`, `props`, `default` for `stylex(...)`),
// or null for any other call.
function stylexCall(node, names) {
  let callee = node.callee;
  if (callee.type === 'SequenceExpression')
    callee = callee.expressions[callee.expressions.length - 1];
  if (callee.type === 'Identifier') {
    if (names.named.has(callee.name)) return names.named.get(callee.name);
    return names.namespaces.has(callee.name) ? 'default' : null;
  }
  let property = null;
  while (callee.type === 'MemberExpression') {
    property = callee.property;
    callee = callee.object;
  }
  if (callee.type !== 'Identifier' || property == null) return null;
  const name =
    property.type === 'Identifier'
      ? property.name
      : property.type === 'StringLiteral'
        ? property.value
        : '';
  if (names.namespaces.has(callee.name)) return name;
  return names.named.has(callee.name) ? names.named.get(callee.name) : null;
}

function sxAttribute(node, sx) {
  return (
    sx != null &&
    node.type === 'JSXOpeningElement' &&
    node.name.type === 'JSXIdentifier' &&
    node.name.name[0] === node.name.name[0].toLowerCase() &&
    node.attributes.some(
      (attr) =>
        attr.type === 'JSXAttribute' &&
        attr.name.type === 'JSXIdentifier' &&
        attr.name.name === sx,
    )
  );
}

module.exports = {
  cannotSpellStyleX,
  dependencies,
  importSources,
  mayUseStyleX,
  stylexCall,
  stylexNames,
  sxAttribute,
  sxPropName,
};
