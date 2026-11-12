/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const t = require('@babel/types');

const ATOMS_SOURCE = '@stylexjs/atoms';

/**
 * Creates a visitor that transforms utility style expressions into compiled style objects.
 *
 * - css.display.flex -> { $$css: true, display: 'xabc123' }
 * - css.color(myVar) -> _styles.color(myVar)
 *
 * @param {object} state - StateManager from babel-plugin
 * @param {object} compile - Compilation utilities from babel-plugin
 * @param {function} compile.styleXCreateSet - Compiles raw styles to class names + CSS
 * @param {function} compile.convertObjectToAST - Converts JS object to babel AST
 * @param {function} compile.hoistExpression - Hoists an expression to module scope
 * @param {function} compile.injectDevClassNames - Adds dev class names
 * @returns {object} Babel visitor
 */
function createUtilityStylesVisitor(state, compile) {
  return {
    MemberExpression(path) {
      if (path.node._utilityStyleProcessed) {
        return;
      }

      const staticStyle = getStaticStyleFromPath(path, state);
      if (staticStyle != null) {
        path.node._utilityStyleProcessed = true;
        compileStaticStyle(path, staticStyle, state, compile);
      }
    },

    CallExpression(path) {
      if (path.node._utilityStyleProcessed) {
        return;
      }

      const dynamicStyle = getDynamicStyleFromPath(path, state);
      if (dynamicStyle != null) {
        path.node._utilityStyleProcessed = true;
        compileDynamicStyle(path, dynamicStyle, state, compile);
      }
    },
  };
}

function isUtilityStylesIdentifier(ident, state, path) {
  if (state.atomImports && state.atomImports.has(ident.name)) {
    return true;
  }

  const binding = path.scope?.getBinding(ident.name);
  if (!binding) {
    return false;
  }

  if (
    binding.path.isImportSpecifier() &&
    binding.path.parent.type === 'ImportDeclaration' &&
    binding.path.parent.source.value === ATOMS_SOURCE
  ) {
    return true;
  }

  if (
    binding.path.isImportNamespaceSpecifier() &&
    binding.path.parent.type === 'ImportDeclaration' &&
    binding.path.parent.source.value === ATOMS_SOURCE
  ) {
    return true;
  }

  if (
    binding.path.isImportDefaultSpecifier() &&
    binding.path.parent.type === 'ImportDeclaration' &&
    binding.path.parent.source.value === ATOMS_SOURCE
  ) {
    return true;
  }

  return false;
}

function getPropKey(prop, computed) {
  if (!computed && t.isIdentifier(prop)) {
    return prop.name;
  }
  if (computed && t.isStringLiteral(prop)) {
    return prop.value;
  }
  if (computed && t.isNumericLiteral(prop)) {
    return String(prop.value);
  }
  return null;
}

/**
 * Strips a leading underscore from CSS values. This allows using underscore-
 * prefixed identifiers for CSS values that conflict with JS reserved words
 * or start with a digit (e.g., css.display._flex or css.zIndex._1).
 */
function normalizeValue(value) {
  if (typeof value === 'string' && value.startsWith('_')) {
    return value.slice(1);
  }
  return value;
}

function getStaticStyleFromPath(path, state) {
  const node = path.node;
  if (!t.isMemberExpression(node)) {
    return null;
  }

  if (
    path.parentPath?.isCallExpression() &&
    path.parentPath.node.callee === node
  ) {
    return null;
  }

  const valueKey = getPropKey(node.property, node.computed);
  if (valueKey == null) {
    return null;
  }

  const parent = node.object;

  if (t.isMemberExpression(parent)) {
    const propName = getPropKey(parent.property, parent.computed);
    const base = parent.object;
    if (
      propName != null &&
      t.isIdentifier(base) &&
      isUtilityStylesIdentifier(base, state, path)
    ) {
      return { property: propName, value: normalizeValue(valueKey) };
    }
  }

  if (
    t.isIdentifier(parent) &&
    isUtilityStylesIdentifier(parent, state, path)
  ) {
    const importedName = state.atomImports?.get(parent.name);
    if (importedName == null) {
      return null;
    }
    const property = importedName === '*' ? valueKey : importedName;
    return { property, value: normalizeValue(valueKey) };
  }

  return null;
}

function getDynamicStyleFromPath(path, state) {
  const callee = path.get('callee');
  if (!callee.isMemberExpression()) {
    return null;
  }

  const valueKey = getPropKey(callee.node.property, callee.node.computed);
  if (valueKey == null) {
    return null;
  }

  if (path.node.arguments.length !== 1) {
    return null;
  }

  const argPath = path.get('arguments')[0];
  if (!argPath || !argPath.node || !argPath.isExpression()) {
    return null;
  }

  const parent = callee.node.object;

  if (
    t.isIdentifier(parent) &&
    isUtilityStylesIdentifier(parent, state, path)
  ) {
    return {
      property: valueKey,
      value: argPath.node,
    };
  }

  if (t.isMemberExpression(parent)) {
    const propName = getPropKey(parent.property, parent.computed);
    const base = parent.object;
    if (
      propName != null &&
      t.isIdentifier(base) &&
      isUtilityStylesIdentifier(base, state, path)
    ) {
      return {
        property: propName,
        value: argPath.node,
      };
    }
  }

  return null;
}

/**
 * Compile static utility style directly to a compiled style object.
 * css.display.flex -> { $$css: true, display: 'xabc123' }
 */
function compileStaticStyle(path, styleInfo, state, compile) {
  const { property, value } = styleInfo;
  const rawStyle = { [property]: value };
  const styleInput = { __inline__: rawStyle };

  // eslint-disable-next-line prefer-const
  let [compiledStyles, injectedStyles] = compile.styleXCreateSet(
    styleInput,
    state.options,
  );

  if (state.isDev && state.options.enableDevClassNames) {
    compiledStyles = {
      ...compile.injectDevClassNames(compiledStyles, null, state),
    };
  }

  const listOfStyles = Object.entries(injectedStyles).map(
    ([key, { priority, ...rest }]) => [key, rest, priority],
  );
  state.registerStyles(listOfStyles, path);

  const compiledStyle = compiledStyles.__inline__;
  if (compiledStyle == null) {
    return;
  }

  const compiledAst = compile.convertObjectToAST(compiledStyle);
  path.replaceWith(compiledAst);
}

/**
 * Compile dynamic utility style to a hoisted function pattern.
 * css.color(myVar) -> _hoisted.color(myVar)
 */
function compileDynamicStyle(path, styleInfo, state, compile) {
  const { property, value } = styleInfo;
  const varName = `--x-${property}`;
  const rawStyle = { [property]: `var(${varName})` };
  const styleInput = { __inline__: rawStyle };

  // eslint-disable-next-line prefer-const
  let [compiledStyles, injectedStyles] = compile.styleXCreateSet(
    styleInput,
    state.options,
  );

  if (state.isDev && state.options.enableDevClassNames) {
    compiledStyles = {
      ...compile.injectDevClassNames(compiledStyles, null, state),
    };
  }

  injectedStyles[varName] = {
    priority: 0,
    ltr: `@property ${varName} { syntax: "*"; inherits: false;}`,
    rtl: null,
  };

  const listOfStyles = Object.entries(injectedStyles).map(
    ([key, { priority, ...rest }]) => [key, rest, priority],
  );
  state.registerStyles(listOfStyles, path);

  const compiledStyle = compiledStyles.__inline__;
  if (compiledStyle == null) {
    return;
  }

  // compiledStyle has: { $$css: true, "devClassName": "devClassName", "prop-hash": "xAbc123" }
  // We need the prop-hash key (not $$css, not the dev class name where key === value).
  const propKey = Object.keys(compiledStyle).find(
    (k) => k !== '$$css' && compiledStyle[k] !== k,
  );
  const className = propKey != null ? compiledStyle[propKey] : null;
  if (className == null || propKey == null || typeof className !== 'string') {
    return;
  }

  const param = t.identifier('_v');
  const nullCheck = t.binaryExpression('!=', param, t.nullLiteral());

  const compiledObj = t.objectExpression([
    t.objectProperty(
      t.stringLiteral(propKey),
      t.conditionalExpression(nullCheck, t.stringLiteral(className), param),
    ),
    t.objectProperty(t.stringLiteral('$$css'), t.booleanLiteral(true)),
  ]);

  const inlineVarsObj = t.objectExpression([
    t.objectProperty(
      t.stringLiteral(varName),
      t.conditionalExpression(
        t.binaryExpression('!=', param, t.nullLiteral()),
        param,
        t.identifier('undefined'),
      ),
    ),
  ]);

  const fnBody = t.arrayExpression([compiledObj, inlineVarsObj]);
  const arrowFn = t.arrowFunctionExpression([param], fnBody);

  const stylesObj = t.objectExpression([
    t.objectProperty(t.identifier(property), arrowFn),
  ]);

  const hoistedId = compile.hoistExpression(path, stylesObj);

  const callExpr = t.callExpression(
    t.memberExpression(hoistedId, t.identifier(property)),
    [value],
  );
  path.replaceWith(callExpr);
}

module.exports = {
  createUtilityStylesVisitor,
  isUtilityStylesIdentifier,
  getStaticStyleFromPath,
  getDynamicStyleFromPath,
};
