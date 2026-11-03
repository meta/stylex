/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

/**
 * This is a extended version of the path evaluation code from Babel.
 *
 * The original can be found at:
 * https://github.com/babel/babel/blob/main/packages/babel-traverse/src/path/evaluation.ts
 *
 * The following extensions were made:
 * - It can accept a mapping from variable names to functions
 *   which when encountered will be evaluated instead of deopting.
 *   - The functions can be configured to accept the raw path instead of
 *     static values to handle dynamic values.
 * - It can handle object spreads when the spread value itself is statically evaluated.
 */

import type { NodePath, Binding } from '@babel/traverse';

import { parseSync } from '@babel/core';
import traverse from '@babel/traverse';
import * as t from '@babel/types';
import StateManager from './state-manager';
import { utils } from '../shared';
import * as errMsgs from './evaluation-errors';
import fs from 'node:fs';

// This file contains Babels metainterpreter that can evaluate static code.

// This evaluator also ships in a browser bundle, where Node's `global` alias
// does not exist.
const globalObject: any = globalThis;

const VALID_CALLEES = ['String', 'Number', 'Math', 'Object', 'Array'];

// The static methods that may be called on the globals in `VALID_CALLEES`.
//
// This is an allowlist rather than a denylist because `Object` exposes
// reflective methods — `getPrototypeOf`, `getOwnPropertyDescriptor`, `create`
// — that hand back objects from the prototype chain. Any one of them is a path
// from a plain object to `Function`, and therefore to arbitrary code execution
// inside the compiler. Only pure methods that return plain data belong here.
//
// A `Map` is used so that a crafted callee name such as `constructor` cannot
// match a property inherited from `Object.prototype`.
const VALID_CALLEE_METHODS: Map<string, Set<string>> = new Map([
  ['String', new Set(['fromCharCode', 'fromCodePoint', 'raw'])],
  [
    'Number',
    new Set([
      'isFinite',
      'isInteger',
      'isNaN',
      'isSafeInteger',
      'parseFloat',
      'parseInt',
    ]),
  ],
  [
    'Math',
    new Set([
      // `random` is deliberately absent: compilation must be deterministic.
      'abs',
      'acos',
      'acosh',
      'asin',
      'asinh',
      'atan',
      'atan2',
      'atanh',
      'cbrt',
      'ceil',
      'clz32',
      'cos',
      'cosh',
      'exp',
      'expm1',
      'f16round',
      'floor',
      'fround',
      'hypot',
      'imul',
      'log',
      'log10',
      'log1p',
      'log2',
      'max',
      'min',
      'pow',
      'round',
      'sign',
      'sin',
      'sinh',
      'sqrt',
      'tan',
      'tanh',
      'trunc',
    ]),
  ],
  [
    'Object',
    // Everything here returns plain data: own values, names or booleans.
    // Excluded are the mutating methods (`assign`, `defineProperty`, `freeze`,
    // `seal`, `preventExtensions`), which were already rejected before, and the
    // reflective ones (`create`, `getPrototypeOf`, `setPrototypeOf`,
    // `getOwnPropertyDescriptor`), which hand back prototype chain objects.
    new Set([
      'entries',
      'fromEntries',
      'getOwnPropertyNames',
      'getOwnPropertySymbols',
      'groupBy',
      'hasOwn',
      'is',
      'isExtensible',
      'isFrozen',
      'isSealed',
      'keys',
      'values',
    ]),
  ],
  ['Array', new Set(['from', 'isArray', 'of'])],
]);

function isValidCallee(val: string): boolean {
  return (VALID_CALLEES as $ReadOnlyArray<string>).includes(val);
}

function isValidCalleeMethod(callee: string, method: string): boolean {
  return VALID_CALLEE_METHODS.get(callee)?.has(method) === true;
}

// Properties that expose the prototype chain. Reading any of these off an
// evaluated value lets an attacker walk from a plain object to `Function`
// (e.g. `({}).constructor.constructor`), which is enough to construct and run
// arbitrary code. Access to these is always blocked during static evaluation.
const BLOCKED_PROPERTIES = new Set(['constructor', '__proto__', 'prototype']);

function isBlockedProperty(property: mixed): boolean {
  return typeof property === 'string' && BLOCKED_PROPERTIES.has(property);
}

// Functions that turn data into executable code. Reaching one of these through
// a gadget this file does not yet know about would mean arbitrary code
// execution at build time, so invoking them is refused outright. This is the
// backstop behind `BLOCKED_PROPERTIES` and `VALID_CALLEE_METHODS`.
function isBlockedFunction(fn: mixed): boolean {
  if (typeof fn !== 'function') {
    return false;
  }
  const callable: $FlowFixMe = fn;
  return (
    // `Function` itself, plus the async, generator and async-generator function
    // constructors, which are the only functions that inherit directly from
    // `Function` rather than from `Function.prototype`.
    callable === Function ||
    Object.getPrototypeOf(callable) === Function ||
    // Referenced, never called: this is a value the evaluator refuses to run.
    // eslint-disable-next-line no-eval
    callable === globalObject.eval
  );
}

// `state.functions.identifiers` and `state.functions.memberExpressions` are
// plain objects, so a bare `config[name]` lookup also finds inherited members:
// `memberExpressions['constructor']` resolves to `Object`, from which
// `constructor.constructor` resolves to `Function`. Only ever read own keys.
function getOwnProperty(obj: $FlowFixMe, key: string): $FlowFixMe {
  if (obj == null) {
    return undefined;
  }
  // $FlowFixMe[method-unbinding]
  return Object.prototype.hasOwnProperty.call(obj, key) ? obj[key] : undefined;
}

const MUTATING_ARRAY_METHODS = new Set([
  'push',
  'pop',
  'shift',
  'unshift',
  'splice',
  'sort',
  'reverse',
  'fill',
  'copyWithin',
]);

function isMutated(binding: Binding): boolean {
  for (const path of binding.referencePaths) {
    const parentPath = path.parentPath;
    if (!parentPath) continue;

    if (
      parentPath.isMemberExpression() &&
      parentPath.node.object === path.node
    ) {
      const memberExpr = parentPath;
      const parent = memberExpr.parentPath;
      if (!parent) continue;

      if (
        parent.isAssignmentExpression() &&
        parent.node.left === memberExpr.node
      ) {
        return true;
      }
      if (parent.isUpdateExpression()) {
        return true;
      }
      if (parent.isUnaryExpression({ operator: 'delete' })) {
        return true;
      }
      if (parent.isCallExpression() && parent.node.callee === memberExpr.node) {
        // $FlowFixMe[prop-missing]
        const property = memberExpr.node.property;
        if (
          t.isIdentifier(property) &&
          MUTATING_ARRAY_METHODS.has(property.name)
        ) {
          return true;
        }
      }
    }

    if (
      parentPath?.isCallExpression() &&
      path.listKey === 'arguments' &&
      path.key === 0
    ) {
      // TODO: There seems to be a Flow bug with `parentPath` here.
      const callExpr: NodePath<t.CallExpression> = parentPath as $FlowFixMe;
      const callee = callExpr.get('callee');
      if (
        callee.matchesPattern('Object.assign') ||
        callee.matchesPattern('Object.defineProperty') ||
        callee.matchesPattern('Object.defineProperties') ||
        callee.matchesPattern('Object.setPrototypeOf')
      ) {
        return true;
      }
    }
  }
  return false;
}

export type FunctionConfig = {
  identifiers: {
    [fnName: string]: $FlowFixMe,
  },
  memberExpressions: {
    [key: string]: {
      [memberName: string]: {
        fn: (...args: $FlowFixMe[]) => $FlowFixMe,
        takesPath?: boolean,
      },
    },
  },
  disableImports?: boolean,
};

type State = {
  confident: boolean,
  deoptPath: NodePath<> | null,
  deoptReason?: string,
  seen: Map<t.Node, Result>,
  addedImports: Set<string>,
  functions: FunctionConfig,
  traversalState: StateManager,
};

type Result =
  | {
      resolved: true,
      value: any,
    }
  | {
      resolved: false,
      reason: string,
    };

type VarGroupProxyOptions = {
  fileName: string,
  exportName: string,
  traversalState: StateManager,
  onAccess?: (key: string) => void,
};

function getVarGroupHash(
  fileName: string,
  exportName: string,
  traversalState: StateManager,
): string {
  return (
    traversalState.options.classNamePrefix +
    utils.hash(utils.genFileBasedIdentifier({ fileName, exportName }))
  );
}

function resolveVarGroupKey(
  key: string,
  fileName: string,
  exportName: string,
  traversalState: StateManager,
): string {
  if (key.startsWith('--')) {
    return `var(${key})`;
  }

  const strToHash = utils.genFileBasedIdentifier({ fileName, exportName, key });
  const { classNamePrefix } = traversalState.options;
  const varName = classNamePrefix + utils.hash(strToHash);

  return `var(--${varName})`;
}

export function createVarGroupProxy({
  fileName,
  exportName,
  traversalState,
  onAccess,
}: VarGroupProxyOptions): { [string]: string } {
  const varGroupHash = getVarGroupHash(fileName, exportName, traversalState);

  return new Proxy(
    {},
    {
      get(_, key: string | symbol) {
        if (typeof key !== 'string') {
          return undefined;
        }
        if (key === '__IS_PROXY') {
          return true;
        }
        if (key === 'toString') {
          return () => varGroupHash;
        }
        if (key === '__varGroupHash__') {
          return varGroupHash;
        }
        onAccess?.(key);
        return resolveVarGroupKey(key, fileName, exportName, traversalState);
      },
      set(_, key: string, value: string) {
        throw new Error(
          `Cannot set value ${value} to key ${key} in theme ${fileName}`,
        );
      },
    },
  );
}
/**
 * Deopts the evaluation
 */
function deopt(path: NodePath<>, state: State, reason: string): void {
  if (!state.confident) return;
  state.deoptPath = path;
  state.confident = false;
  state.deoptReason = reason;
}

function evaluateImportedFile(
  filePath: string,
  namedExport: string,
  state: State,
  bindingPath: NodePath<>,
): any {
  const fileContents = fs.readFileSync(filePath, 'utf8');
  // It's safe to use `.babelrc` here because we're only
  // interested in the JS runtime, and not the CSS.
  // TODO: in environments where `.babelrc` is not available,
  // we need to find a way to decide whether to use Flow or TS syntax extensions.
  const ast: null | t.File | { +errors: mixed } = parseSync(fileContents, {
    babelrc: true,
  });
  if (!ast || ast.errors || !t.isNode(ast)) {
    deopt(bindingPath, state, errMsgs.IMPORT_FILE_PARSING_ERROR);
    return;
  }

  const astNode: t.Node = ast as $FlowFixMe;

  let result: any;

  traverse(astNode, {
    ExportNamedDeclaration(path: NodePath<t.ExportNamedDeclaration>) {
      const declaration = path.get('declaration');

      if (declaration.isVariableDeclaration()) {
        const decls = declaration.get('declarations');

        const finder = (decl: NodePath<t.Node>) => {
          if (decl.isVariableDeclarator()) {
            const id = decl.get('id');
            const init: ?NodePath<t.Expression> = (
              decl as NodePath<t.VariableDeclarator>
            ).get('init');
            if (
              id.isIdentifier() &&
              id.node.name === namedExport &&
              init != null &&
              init.isExpression()
            ) {
              result = evaluateCached(init, state);
            }
          }
        };
        if (Array.isArray(decls)) {
          decls.forEach(finder);
        } else {
          finder(decls);
        }
      }
    },
  });

  if (state.confident) {
    return result;
  } else {
    deopt(bindingPath, state, errMsgs.IMPORT_FILE_EVAL_ERROR);
    return;
  }
}

function evaluateThemeRef(
  fileName: string,
  exportName: string,
  state: State,
): { [key: string]: string } {
  return createVarGroupProxy({
    fileName,
    exportName,
    traversalState: state.traversalState,
  });
}

/**
 * We wrap the _evaluate method so we can track `seen` nodes, we push an item
 * to the map before we actually evaluate it so we can deopt on self recursive
 * nodes such as:
 *
 *   var g = a ? 1 : 2,
 *       a = g * this.foo
 */
function evaluateCached(path: NodePath<>, state: State): any {
  const { node } = path;
  const { seen } = state;

  const existing: ?Result = seen.get(node);
  if (existing != null) {
    if (existing.resolved) {
      return existing.value;
    } else {
      deopt(path, state, existing.reason);
      return;
    }
  } else {
    const item: Result = { resolved: false, reason: 'Currently evaluating' };
    seen.set(node, item);

    if (node == null) {
      deopt(path, state, errMsgs.PATH_WITHOUT_NODE);
      return;
    }

    const val = _evaluate(path, state);
    if (state.confident) {
      seen.set(node, {
        resolved: true,
        value: val,
      });
    }

    return val;
  }
}

function _evaluate(path: NodePath<>, state: State): any {
  if (!state.confident) return;

  if (path.isArrowFunctionExpression()) {
    const body = path.get('body');
    const params: $ReadOnlyArray<
      NodePath<t.Identifier | t.Pattern | t.RestElement>,
    > = path.get('params');
    const identParams = params
      .filter(
        (
          param: NodePath<t.Identifier | t.Pattern | t.RestElement>,
        ): param is NodePath<t.Identifier> => param.isIdentifier(),
      )
      .map((paramPath) => paramPath.node.name);

    if (body.isExpression() && identParams.length === params.length) {
      const evaluatedExpr: NodePath<t.Expression> = body;
      const evaluatedFn: any = (...args: Array<any>) => {
        const identifierEntries = identParams.map(
          (ident, index): [string, any] => [ident, args[index]],
        );
        const identifiersObj = Object.fromEntries(identifierEntries);
        const result = evaluate(evaluatedExpr, state.traversalState, {
          ...state.functions,
          identifiers: { ...state.functions.identifiers, ...identifiersObj },
        });
        if (!result.confident) {
          throw new Error(result.reason ?? errMsgs.NON_CONSTANT);
        }
        return result.value;
      };
      Object.defineProperty(evaluatedFn, '__stylexParamCount', {
        value: identParams.length,
      });
      return evaluatedFn;
    }
  }

  if (path.isIdentifier()) {
    const name: string = path.node.name;
    if (Object.keys(state.functions?.identifiers ?? {}).includes(name)) {
      return state.functions.identifiers[name];
    }
  }

  if (path.isTSAsExpression()) {
    const expr: NodePath<t.Expression> = path.get('expression');
    return evaluateCached(expr, state);
  }

  if (path.node.type === 'TSSatisfiesExpression') {
    const expr: NodePath<t.Expression> = (path as $FlowFixMe).get('expression');
    return evaluateCached(expr, state);
  }

  if (path.isSequenceExpression()) {
    const exprs = path.get('expressions');
    return evaluateCached(exprs[exprs.length - 1], state);
  }

  if (
    path.isStringLiteral() ||
    path.isNumericLiteral() ||
    path.isBooleanLiteral()
  ) {
    return path.node.value;
  }

  if (path.isNullLiteral()) {
    return null;
  }

  if (path.isTemplateLiteral()) {
    return evaluateQuasis(path, path.node.quasis, state);
  }

  const maybeTag = path.isTaggedTemplateExpression() && path.get('tag');
  if (
    path.isTaggedTemplateExpression() &&
    maybeTag &&
    maybeTag.isMemberExpression()
  ) {
    const tag: NodePath<t.MemberExpression> = maybeTag;
    const object: NodePath<t.Expression | t.Super> = tag.get('object');

    if (object.isIdentifier()) {
      const name = object.node.name;
      const property: NodePath<> = tag.get('property');

      if (
        name === 'String' &&
        !path.scope.hasBinding(name) &&
        property.isIdentifier() &&
        property.node.name === 'raw'
      ) {
        return evaluateQuasis(path, path.node.quasi.quasis, state, true);
      }
    }
  }

  if (path.isConditionalExpression()) {
    const testResult = evaluateCached(path.get('test'), state);
    if (!state.confident) return;
    if (testResult) {
      return evaluateCached(path.get('consequent'), state);
    } else {
      return evaluateCached(path.get('alternate'), state);
    }
  }

  if (path.isExpressionWrapper()) {
    // TypeCastExpression, ExpressionStatement etc
    return evaluateCached(path.get('expression'), state);
  }

  /**
   * Collects the full member expression chain for cross-file nested token resolution.
   *
   * When tokens are imported from another .stylex.js file, the plugin creates a
   * themeNameRef proxy that only handles single-level access. Multi-level access like
   * tokens.button.primary.background would fail because proxy['button'] returns a
   * string, and "var(--hash)"['primary'] is undefined.
   *
   * This function walks the MemberExpression AST chain from outermost to innermost,
   * collecting all property names. The caller can then resolve the full dotted key
   * against the proxy in one shot: proxy['button.primary.background'].
   *
   * Example:
   *   AST for tokens.button.primary.background
   *   → { basePath: <Identifier:tokens>, parts: ['button', 'primary', 'background'] }
   *
   * Returns null for:
   *   - Single-level access (no benefit from path collection)
   *   - Dynamic computed properties that can't be resolved statically
   *
   * @param memberPath - The outermost MemberExpression NodePath
   * @returns { basePath, parts } or null
   */
  function getFullMemberPath(
    memberPath: NodePath<t.MemberExpression>,
  ): ?{ basePath: NodePath<>, parts: Array<string> } {
    const parts: Array<string> = [];
    let current: NodePath<> = memberPath;

    while (current.isMemberExpression()) {
      const propPath = current.get('property');
      if (current.node.computed) {
        // Only handle static computed properties (string/number literals)
        if (propPath.isStringLiteral()) {
          parts.unshift(propPath.node.value);
        } else if (propPath.isNumericLiteral()) {
          parts.unshift(String(propPath.node.value));
        } else {
          return null; // Dynamic computed property — can't collect statically
        }
      } else if (propPath.isIdentifier()) {
        parts.unshift(propPath.node.name);
      } else {
        return null;
      }
      current = current.get('object');
    }

    if (parts.length < 2) {
      return null; // Single-level access — no benefit from collecting path
    }

    return { basePath: current, parts };
  }

  // "foo".length
  if (
    path.isMemberExpression() &&
    !path.parentPath.isCallExpression({ callee: path.node })
  ) {
    // Cross-file nested token resolution:
    // When tokens are imported from another .stylex.js file, the evaluator creates
    // a themeNameRef proxy. For flat tokens (tokens.color), single-level proxy access
    // works fine. For nested tokens (tokens.button.primary.background), multi-level
    // access fails because proxy['button'] returns "var(--hash)" (a string) and
    // "var(--hash)"['primary'] is undefined.
    //
    // Fix: collect the full member chain ['button', 'primary', 'background'] and
    // resolve it as a single dotted key: proxy['button.primary.background'].
    // The dotted key produces the same hash as defineVarsNested compilation
    // (which internally flattens to the same dotted key).
    const fullPath = getFullMemberPath(path);
    if (fullPath != null) {
      const { basePath, parts } = fullPath;
      const baseObject = evaluateCached(basePath, state);
      if (!state.confident) {
        return;
      }
      if (
        baseObject != null &&
        typeof baseObject === 'object' &&
        baseObject.__IS_PROXY === true
      ) {
        // Resolve the full dotted path at once against the proxy
        return baseObject[parts.join('.')];
      }
      // Not a proxy — fall through to standard recursive evaluation
    }

    const object = evaluateCached(path.get('object'), state);
    if (!state.confident) {
      return;
    }

    const propPath = path.get('property');

    let property;
    if (path.node.computed) {
      const computedKey = evaluateCached(propPath, state);
      if (!state.confident) {
        return;
      }
      if (typeof computedKey === 'symbol') {
        return deopt(propPath, state, errMsgs.UNEXPECTED_MEMBER_LOOKUP);
      }
      // Normalize the key the way the runtime would before checking it against
      // the blocklist, and then look the property up with the normalized key.
      // Without this a boxed value such as `Object('constructor')` slips past
      // the check as a non-string yet still resolves to `constructor`.
      property = String(computedKey);
    } else if (propPath.isIdentifier()) {
      property = propPath.node.name;
    } else if (propPath.isStringLiteral()) {
      property = propPath.node.value;
    } else {
      return deopt(propPath, state, errMsgs.UNEXPECTED_MEMBER_LOOKUP);
    }

    if (isBlockedProperty(property)) {
      return deopt(propPath, state, errMsgs.BLOCKED_PROPERTY_ACCESS);
    }

    return object[property];
  }

  if (path.isReferencedIdentifier()) {
    const binding: ?Binding = path.scope?.getBinding(path.node.name);

    const bindingPath = binding?.path;
    if (
      binding &&
      bindingPath &&
      !bindingPath.isImportDefaultSpecifier() &&
      !bindingPath.isImportNamespaceSpecifier() &&
      bindingPath.isImportSpecifier()
    ) {
      const importSpecifierPath: NodePath<t.ImportSpecifier> = bindingPath;
      const importSpecifierNode: t.ImportSpecifier = importSpecifierPath.node;
      // const localName = binding.path.node.local.name;
      const imported: t.Identifier | t.StringLiteral =
        importSpecifierNode.imported;
      const importedName =
        imported.type === 'Identifier' ? imported.name : imported.value;
      const importPath = binding.path.parentPath;
      if (
        importPath &&
        importPath.isImportDeclaration() &&
        !state.functions.disableImports
      ) {
        const absPath = state.traversalState.importPathResolver(
          importPath.node.source.value,
        );
        if (!absPath) {
          return deopt(
            binding.path,
            state,
            errMsgs.IMPORT_PATH_RESOLUTION_ERROR,
          );
        }
        const [type, value] = absPath;

        const returnValue =
          type === 'themeNameRef'
            ? evaluateThemeRef(value, importedName, state)
            : evaluateImportedFile(value, importedName, state, bindingPath);
        if (state.confident) {
          if (
            !state.addedImports.has(importPath.node.source.value) &&
            state.traversalState.treeshakeCompensation
          ) {
            importPath.insertBefore(
              t.importDeclaration([], importPath.node.source),
            );
            state.addedImports.add(importPath.node.source.value);
          }
          return returnValue;
        } else {
          deopt(binding.path, state, errMsgs.IMPORT_FILE_EVAL_ERROR);
        }
      }
    }

    if (binding && bindingPath && bindingPath.isImportDefaultSpecifier()) {
      deopt(binding.path, state, errMsgs.IMPORT_FILE_EVAL_ERROR);
    }

    if (binding && binding.constantViolations.length > 0) {
      return deopt(binding.path, state, errMsgs.NON_CONSTANT);
    }

    if (binding && isMutated(binding)) {
      return deopt(binding.path, state, errMsgs.NON_CONSTANT);
    }

    if (binding && path.node.start < binding.path.node.end) {
      return deopt(binding.path, state, errMsgs.USED_BEFORE_DECLARATION);
    }

    if (binding && binding.hasValue) {
      return binding.value;
    } else {
      if (path.node.name === 'undefined') {
        return binding
          ? deopt(binding.path, state, errMsgs.UNINITIALIZED_CONST)
          : undefined;
      } else if (path.node.name === 'Infinity') {
        return binding
          ? deopt(binding.path, state, errMsgs.UNINITIALIZED_CONST)
          : Infinity;
      } else if (path.node.name === 'NaN') {
        return binding
          ? deopt(binding.path, state, errMsgs.UNINITIALIZED_CONST)
          : NaN;
      }

      const resolved = (path as $FlowFixMe).resolve();
      if (resolved === path) {
        return deopt(path, state, errMsgs.UNDEFINED_CONST);
      } else {
        return evaluateCached(resolved, state);
      }
    }
  }

  if (path.isUnaryExpression({ prefix: true })) {
    if (path.node.operator === 'void') {
      // we don't need to evaluate the argument to know what this will return
      return undefined;
    }

    const argument = path.get('argument');
    if (
      path.node.operator === 'typeof' &&
      (argument.isFunction() || argument.isClass())
    ) {
      return 'function';
    }

    const arg = evaluateCached(argument, state);
    if (!state.confident) return;
    switch (path.node.operator) {
      case '!':
        return !arg;
      case '+':
        return +arg;
      case '-':
        return -arg;
      case '~':
        return ~arg;
      case 'typeof':
        return typeof arg;
      case 'void':
        return undefined;
      default:
        return deopt(
          path,
          state,
          errMsgs.UNSUPPORTED_OPERATOR(path.node.operator),
        );
    }
  }

  if (path.isArrayExpression()) {
    const arrPath: NodePath<t.ArrayExpression> = path;
    const arr = [];
    const elems: $ReadOnlyArray<NodePath<>> = arrPath.get('elements');
    for (const elem of elems) {
      const elemValue = evaluate(elem, state.traversalState, state.functions);

      if (elemValue.confident) {
        arr.push(elemValue.value);
      } else {
        elemValue.deopt &&
          deopt(elemValue.deopt, state, elemValue.reason ?? 'unknown error');
        return;
      }
    }
    return arr;
  }

  if (path.isObjectExpression()) {
    const obj: { [string]: mixed } = {};
    const props: $ReadOnlyArray<
      NodePath<t.ObjectMethod | t.ObjectProperty | t.SpreadElement>,
    > = path.get('properties');
    for (const prop of props) {
      if (prop.isObjectMethod()) {
        return deopt(prop, state, errMsgs.OBJECT_METHOD);
      }
      if (prop.isSpreadElement()) {
        const spreadExpression = evaluateCached(prop.get('argument'), state);
        if (!state.confident) {
          return deopt(prop, state, state.deoptReason ?? 'unknown error');
        }
        // $FlowFixMe[unsafe-object-assign]
        Object.assign(obj, spreadExpression);
        continue;
      }
      if (prop.isObjectProperty()) {
        const keyPath: NodePath<t.ObjectProperty['key']> = prop.get('key');
        let key: string | number | boolean;
        if (prop.node.computed) {
          const {
            confident,
            deopt: resultDeopt,
            reason: deoptReason,
            value,
          } = evaluate(
            keyPath,
            state.traversalState,
            state.functions,
            state.seen,
          );

          if (!confident) {
            resultDeopt &&
              deopt(resultDeopt, state, deoptReason ?? 'unknown error');
            return;
          }
          key = value;
        } else if (keyPath.isIdentifier()) {
          key = keyPath.node.name;
        } else {
          // TODO: This isn't handling all possible types that `keyPath` could be
          key = (keyPath.node as $FlowFixMe).value;
        }

        const valuePath: NodePath<> = prop.get('value');
        let value = evaluate(
          valuePath,
          state.traversalState,
          state.functions,
          state.seen,
        );
        if (!value.confident) {
          value.deopt &&
            deopt(value.deopt, state, value.reason ?? 'unknown error');
          return;
        }
        value = value.value;
        obj[key] = value;
      }
    }
    return obj;
  }

  if (path.isLogicalExpression()) {
    // If we are confident that the left side of an && is false, or the left
    // side of an || is true, we can be confident about the entire expression
    const stateForLeft = {
      ...state,
      deoptPath: null,
      confident: true,
    } as const;
    const leftPath = path.get('left');
    const left = evaluateCached(leftPath, stateForLeft as $FlowFixMe);
    const leftConfident: boolean = stateForLeft.confident as $FlowFixMe;

    const stateForRight = { ...state, deoptPath: null, confident: true };
    const rightPath = path.get('right');
    const right = evaluateCached(rightPath, stateForRight as $FlowFixMe);
    const rightConfident: boolean = stateForRight.confident as $FlowFixMe;

    switch (path.node.operator) {
      case '||': {
        // TODO consider having a "truthy type" that doesn't bail on
        // left uncertainty but can still evaluate to truthy.
        if (leftConfident && (!!left || rightConfident)) {
          return left || right;
        }
        if (!leftConfident) {
          deopt(leftPath, state, stateForLeft.deoptReason ?? 'unknown error');
          return;
        }
        if (!rightConfident) {
          deopt(rightPath, state, stateForRight.deoptReason ?? 'unknown error');
          return;
        }

        deopt(path, state, 'unknown error');
        return;
      }
      case '&&': {
        if (leftConfident && (!left || rightConfident)) {
          return left && right;
        }
        if (!leftConfident) {
          deopt(leftPath, state, stateForLeft.deoptReason ?? 'unknown error');
          return;
        }
        if (!rightConfident) {
          deopt(rightPath, state, stateForRight.deoptReason ?? 'unknown error');
          return;
        }

        deopt(path, state, 'unknown error');
        return;
      }
      case '??': {
        if (leftConfident && !!(left ?? rightConfident)) {
          return left ?? right;
        }
        if (!leftConfident) {
          deopt(leftPath, state, stateForLeft.deoptReason ?? 'unknown error');
          return;
        }
        if (!rightConfident) {
          deopt(rightPath, state, stateForRight.deoptReason ?? 'unknown error');
          return;
        }

        deopt(path, state, 'unknown error');
        return;
      }
      default:
        path.node.operator as empty;
    }
  }

  if (path.isBinaryExpression()) {
    const left = evaluateCached(path.get('left'), state);
    if (!state.confident) return;
    const right = evaluateCached(path.get('right'), state);
    if (!state.confident) return;

    switch (path.node.operator) {
      case '-':
        return left - right;
      case '+':
        return left + right;
      case '/':
        return left / right;
      case '*':
        return left * right;
      case '%':
        return left % right;
      case '**':
        return left ** right;
      case '<':
        return left < right;
      case '>':
        return left > right;
      case '<=':
        return left <= right;
      case '>=':
        return left >= right;
      case '==':
        return left == right; // eslint-disable-line eqeqeq
      case '!=':
        return left !== right;
      case '===':
        return left === right;
      case '!==':
        return left !== right;
      case '|':
        return left | right;
      case '&':
        return left & right;
      case '^':
        return left ^ right;
      case '<<':
        return left << right;
      case '>>':
        return left >> right;
      case '>>>':
        return left >>> right;
      case 'in':
        return left in right;
      case 'instanceof':
        return left instanceof right;
      default:
        return;
    }
  }

  if (path.isCallExpression()) {
    const callee = path.get('callee');
    let context;
    let func;

    // Number(1);
    if (
      callee.isIdentifier() &&
      !path.scope.getBinding(callee.node.name) &&
      isValidCallee(callee.node.name)
    ) {
      func = globalObject[callee.node.name];
    } else if (
      callee.isIdentifier() &&
      getOwnProperty(state.functions.identifiers, callee.node.name)
    ) {
      func = getOwnProperty(state.functions.identifiers, callee.node.name);
    } else if (callee.isIdentifier()) {
      const maybeFunction = evaluateCached(callee, state);
      if (state.confident) {
        func = maybeFunction;
      } else {
        deopt(callee, state, errMsgs.NON_CONSTANT);
      }
    }

    if (callee.isMemberExpression()) {
      const object = callee.get('object');
      const property = callee.get('property');

      // Math.min(1, 2)
      if (object.isIdentifier() && property.isIdentifier()) {
        if (
          isValidCallee(object.node.name) &&
          isValidCalleeMethod(object.node.name, property.node.name)
        ) {
          context = globalObject[object.node.name];
          // @ts-expect-error property may not exist in context object
          func = context[property.node.name];
        } else {
          const memberFns = getOwnProperty(
            state.functions.memberExpressions,
            object.node.name,
          );
          const memberFn = getOwnProperty(memberFns, property.node.name);
          if (memberFn) {
            context = memberFns;
            func = memberFn;
          }
        }
      }

      if (object.isIdentifier() && property.isStringLiteral()) {
        const memberFns = getOwnProperty(
          state.functions.memberExpressions,
          object.node.name,
        );
        const memberFn = getOwnProperty(memberFns, property.node.value);
        if (memberFn) {
          context = memberFns;
          func = memberFn;
        }
      }

      // "abc".charCodeAt(4)
      if (
        (object.isStringLiteral() || object.isNumericLiteral()) &&
        property.isIdentifier()
      ) {
        if (isBlockedProperty(property.node.name)) {
          return deopt(property, state, errMsgs.BLOCKED_PROPERTY_ACCESS);
        }
        const val: number | string = object.node.value;
        func = (val as $FlowFixMe)[property.node.name];
        if (typeof val === 'string') {
          context = object.node.value;
        }
      }

      if (func == null) {
        const parsedObj = evaluate(
          object,
          state.traversalState,
          state.functions,
        );
        if (parsedObj.confident && property.isIdentifier()) {
          if (isBlockedProperty(property.node.name)) {
            return deopt(property, state, errMsgs.BLOCKED_PROPERTY_ACCESS);
          }
          func = parsedObj.value[property.node.name];
          context = parsedObj.value;
        }
        if (parsedObj.confident && property.isStringLiteral()) {
          if (isBlockedProperty(property.node.value)) {
            return deopt(property, state, errMsgs.BLOCKED_PROPERTY_ACCESS);
          }
          func = parsedObj.value[property.node.value];
          context = parsedObj.value;
        }
      }
    }

    if (func) {
      if (isBlockedFunction(func) || isBlockedFunction(func.fn)) {
        return deopt(path, state, errMsgs.BLOCKED_FUNCTION_CALL);
      }

      if (func.takesPath) {
        const args = path.get('arguments');
        return func.fn(...args);
      } else {
        const args = path
          .get('arguments')
          .map((arg: NodePath<t.CallExpression['arguments'][number]>) =>
            evaluateCached(arg, state),
          );
        if (!state.confident) return;

        if (func.fn) {
          return func.fn.apply(context, args);
        } else {
          return func.apply(context, args);
        }
      }
    }
  }

  deopt(path, state, errMsgs.UNSUPPORTED_EXPRESSION(path.node.type));
}

function evaluateQuasis(
  path: NodePath<t.TaggedTemplateExpression | t.TemplateLiteral>,
  quasis: Array<any>,
  state: State,
  raw: boolean = false,
) {
  let str = '';

  let i = 0;
  const exprs: $ReadOnlyArray<NodePath<>> = path.isTemplateLiteral()
    ? path.get('expressions')
    : path.isTaggedTemplateExpression()
      ? path.get('quasi').get('expressions')
      : [];

  // const exprs: Array<NodePath<t.Node>> = path.isTemplateLiteral()
  //   ? path.get('expressions')
  //   : (path as NodePath<t.TaggedTemplateExpression>)
  //       .get('quasi')
  //       .get('expressions');

  for (const elem of quasis) {
    // not confident, evaluated an expression we don't like
    if (!state.confident) break;

    // add on element
    str += raw ? elem.value.raw : elem.value.cooked;

    // add on interpolated expression if it's present
    const expr = exprs[i++];
    if (expr) str += String(evaluateCached(expr, state));
  }

  if (!state.confident) return;
  return str;
}

/**
 * Walk the input `node` and statically evaluate it.
 *
 * Returns an object in the form `{ confident, value, deopt }`. `confident`
 * indicates whether or not we had to drop out of evaluating the expression
 * because of hitting an unknown node that we couldn't confidently find the
 * value of, in which case `deopt` is the path of said node.
 *
 * Example:
 *
 *   evaluate(parse("5 + 5")) // { confident: true, value: 10 }
 *   evaluate(parse("!true")) // { confident: true, value: false }
 *   evaluate(parse("foo + foo")) // { confident: false, value: undefined, deopt: NodePath }
 *
 */

// Track all the imports added to the file, so we don't add them multiple times
// Instead of polluting StateManager with this, we use a WeakMap
// so the logic can be localized this file.
const importsForState = new WeakMap<StateManager, Set<string>>();

export function evaluate(
  path: NodePath<>,
  traversalState: StateManager,
  functions: FunctionConfig = {
    identifiers: {},
    memberExpressions: {},
    disableImports: false,
  },
  seen: Map<t.Node, Result> = new Map(),
): $ReadOnly<{
  confident: boolean,
  value: any,
  deopt?: null | NodePath<>,
  reason?: string,
}> {
  const addedImports = importsForState.get(traversalState) ?? new Set();
  importsForState.set(traversalState, addedImports);

  const state: State = {
    confident: true,
    deoptPath: null,
    seen,
    addedImports,
    functions,
    traversalState,
  };
  let value = evaluateCached(path, state);
  if (!state.confident) value = undefined;

  return {
    confident: state.confident,
    deopt: state.deoptPath,
    reason: state.deoptReason,
    value: value,
  };
}
