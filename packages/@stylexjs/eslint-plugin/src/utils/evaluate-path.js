/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

type EvaluationResult =
  | { confident: true, value: mixed }
  | { confident: false, value: void };

const FAILED: EvaluationResult = { confident: false, value: undefined };

function confident(value: mixed): EvaluationResult {
  return { confident: true, value };
}

type Scope = interface {
  getBinding(name: string): ?{
    path: NodePath,
    ...
  },
  ...
};

type NodePath = interface {
  node: ESNode,
  scope: Scope,
  parentPath: ?NodePath,
  ...
};

type ESNode = {
  type: string,
  [string]: mixed,
  ...
};

function evaluatePath(path: NodePath): EvaluationResult {
  return evaluateNode(path.node, path.scope, new Set());
}

function evaluateNode(
  node: ESNode,
  scope: Scope,
  seen: Set<ESNode>,
): EvaluationResult {
  if (seen.has(node)) {
    return FAILED;
  }

  switch (node.type) {
    case 'StringLiteral':
    case 'NumericLiteral':
    case 'BooleanLiteral':
      return confident(node.value);

    case 'NullLiteral':
      return confident(null);

    case 'Literal': {
      // ESTree-style literal (used by ESLint)
      const litNode = node;
      if (
        typeof litNode.value === 'string' ||
        typeof litNode.value === 'number' ||
        typeof litNode.value === 'boolean'
      ) {
        return confident(litNode.value);
      }
      if (litNode.value === null) {
        return confident(null);
      }
      return FAILED;
    }

    case 'TemplateLiteral': {
      const quasis: $ReadOnlyArray<ESNode> = (node.quasis: $FlowFixMe);
      const expressions: $ReadOnlyArray<ESNode> = (node.expressions: $FlowFixMe);
      let result = '';
      for (let i = 0; i < quasis.length; i++) {
        const quasi = quasis[i];
        const cooked: string = (quasi.value: $FlowFixMe).cooked;
        if (cooked == null) {
          return FAILED;
        }
        result += cooked;
        if (i < expressions.length) {
          const exprResult = evaluateNode(
            (expressions[i]: $FlowFixMe),
            scope,
            seen,
          );
          if (!exprResult.confident) {
            return FAILED;
          }
          result += String(exprResult.value);
        }
      }
      return confident(result);
    }

    case 'BinaryExpression': {
      const operator: string = (node.operator: $FlowFixMe);
      const leftResult = evaluateNode((node.left: $FlowFixMe), scope, seen);
      if (!leftResult.confident) {
        return FAILED;
      }
      const rightResult = evaluateNode((node.right: $FlowFixMe), scope, seen);
      if (!rightResult.confident) {
        return FAILED;
      }
      const l = leftResult.value;
      const r = rightResult.value;
      switch (operator) {
        case '+':
          // $FlowFixMe
          return confident((l: $FlowFixMe) + (r: $FlowFixMe));
        case '-':
          // $FlowFixMe
          return confident((l: $FlowFixMe) - (r: $FlowFixMe));
        case '*':
          // $FlowFixMe
          return confident((l: $FlowFixMe) * (r: $FlowFixMe));
        case '/':
          // $FlowFixMe
          return confident((l: $FlowFixMe) / (r: $FlowFixMe));
        case '%':
          // $FlowFixMe
          return confident((l: $FlowFixMe) % (r: $FlowFixMe));
        case '**':
          // $FlowFixMe
          return confident((l: $FlowFixMe) ** (r: $FlowFixMe));
        case '===':
          return confident(l === r);
        case '!==':
          return confident(l !== r);
        case '==':
          // $FlowFixMe
          return confident(l == r); // eslint-disable-line eqeqeq
        case '!=':
          // $FlowFixMe
          return confident(l != r); // eslint-disable-line eqeqeq
        case '<':
          // $FlowFixMe
          return confident((l: $FlowFixMe) < (r: $FlowFixMe));
        case '<=':
          // $FlowFixMe
          return confident((l: $FlowFixMe) <= (r: $FlowFixMe));
        case '>':
          // $FlowFixMe
          return confident((l: $FlowFixMe) > (r: $FlowFixMe));
        case '>=':
          // $FlowFixMe
          return confident((l: $FlowFixMe) >= (r: $FlowFixMe));
        case '|':
          // $FlowFixMe
          return confident((l: $FlowFixMe) | (r: $FlowFixMe));
        case '&':
          // $FlowFixMe
          return confident((l: $FlowFixMe) & (r: $FlowFixMe));
        case '^':
          // $FlowFixMe
          return confident((l: $FlowFixMe) ^ (r: $FlowFixMe));
        case '>>':
          // $FlowFixMe
          return confident((l: $FlowFixMe) >> (r: $FlowFixMe));
        case '<<':
          // $FlowFixMe
          return confident((l: $FlowFixMe) << (r: $FlowFixMe));
        case '>>>':
          // $FlowFixMe
          return confident((l: $FlowFixMe) >>> (r: $FlowFixMe));
        default:
          return FAILED;
      }
    }

    case 'UnaryExpression': {
      const operator: string = (node.operator: $FlowFixMe);
      const argResult = evaluateNode((node.argument: $FlowFixMe), scope, seen);
      if (!argResult.confident) {
        return FAILED;
      }
      const v = argResult.value;
      switch (operator) {
        case '!':
          return confident(!v);
        case '+':
          // $FlowFixMe
          return confident(+(v: $FlowFixMe));
        case '-':
          // $FlowFixMe
          return confident(-(v: $FlowFixMe));
        case '~':
          // $FlowFixMe
          return confident(~(v: $FlowFixMe));
        case 'typeof':
          return confident(typeof v);
        case 'void':
          return confident(undefined);
        default:
          return FAILED;
      }
    }

    case 'LogicalExpression': {
      const operator: string = (node.operator: $FlowFixMe);
      const leftResult = evaluateNode((node.left: $FlowFixMe), scope, seen);
      if (!leftResult.confident) {
        return FAILED;
      }
      switch (operator) {
        case '&&':
          if (!leftResult.value) {
            return confident(leftResult.value);
          }
          return evaluateNode((node.right: $FlowFixMe), scope, seen);
        case '||':
          if (leftResult.value) {
            return confident(leftResult.value);
          }
          return evaluateNode((node.right: $FlowFixMe), scope, seen);
        case '??':
          if (leftResult.value != null) {
            return confident(leftResult.value);
          }
          return evaluateNode((node.right: $FlowFixMe), scope, seen);
        default:
          return FAILED;
      }
    }

    case 'ConditionalExpression': {
      const testResult = evaluateNode((node.test: $FlowFixMe), scope, seen);
      if (!testResult.confident) {
        return FAILED;
      }
      if (testResult.value) {
        return evaluateNode((node.consequent: $FlowFixMe), scope, seen);
      } else {
        return evaluateNode((node.alternate: $FlowFixMe), scope, seen);
      }
    }

    case 'MemberExpression': {
      const objectResult = evaluateNode((node.object: $FlowFixMe), scope, seen);
      if (!objectResult.confident) {
        return FAILED;
      }
      const obj = objectResult.value;
      if (obj == null || typeof obj !== 'object') {
        return FAILED;
      }
      let key: mixed;
      if ((node.computed: $FlowFixMe)) {
        const keyResult = evaluateNode(
          (node.property: $FlowFixMe),
          scope,
          seen,
        );
        if (!keyResult.confident) {
          return FAILED;
        }
        key = keyResult.value;
      } else {
        const prop: ESNode = (node.property: $FlowFixMe);
        if (prop.type === 'Identifier') {
          key = prop.name;
        } else {
          return FAILED;
        }
      }
      if (typeof key !== 'string' && typeof key !== 'number') {
        return FAILED;
      }
      // $FlowFixMe
      const memberValue = (obj: $FlowFixMe)[key];
      return confident(memberValue);
    }

    case 'ObjectExpression': {
      const properties: $ReadOnlyArray<ESNode> = (node.properties: $FlowFixMe);
      const result: { [string]: mixed } = {};
      for (const prop of properties) {
        if (prop.type === 'SpreadElement' || prop.type === 'SpreadProperty') {
          return FAILED;
        }
        if ((prop: $FlowFixMe).computed) {
          const keyResult = evaluateNode(
            (prop: $FlowFixMe).key,
            scope,
            seen,
          );
          if (!keyResult.confident) {
            return FAILED;
          }
          const valResult = evaluateNode(
            (prop: $FlowFixMe).value,
            scope,
            seen,
          );
          if (!valResult.confident) {
            return FAILED;
          }
          result[String(keyResult.value)] = valResult.value;
        } else {
          const keyNode: ESNode = (prop: $FlowFixMe).key;
          let keyName: string;
          if (keyNode.type === 'Identifier') {
            keyName = (keyNode.name: $FlowFixMe);
          } else if (
            keyNode.type === 'StringLiteral' ||
            keyNode.type === 'Literal'
          ) {
            keyName = String((keyNode.value: $FlowFixMe));
          } else {
            return FAILED;
          }
          const valResult = evaluateNode(
            (prop: $FlowFixMe).value,
            scope,
            seen,
          );
          if (!valResult.confident) {
            return FAILED;
          }
          result[keyName] = valResult.value;
        }
      }
      return confident(result);
    }

    case 'ArrayExpression': {
      const elements: $ReadOnlyArray<?ESNode> = (node.elements: $FlowFixMe);
      const result: Array<mixed> = [];
      for (const element of elements) {
        if (element == null) {
          result.push(undefined);
          continue;
        }
        if (element.type === 'SpreadElement') {
          return FAILED;
        }
        const elemResult = evaluateNode(element, scope, seen);
        if (!elemResult.confident) {
          return FAILED;
        }
        result.push(elemResult.value);
      }
      return confident(result);
    }

    case 'Identifier': {
      const name: string = (node.name: $FlowFixMe);
      if (name === 'undefined') {
        return confident(undefined);
      }
      if (name === 'Infinity') {
        return confident(Infinity);
      }
      if (name === 'NaN') {
        return confident(NaN);
      }
      const binding = scope.getBinding(name);
      if (binding == null) {
        return FAILED;
      }
      const bindingPath: NodePath = binding.path;
      const bindingNode: ESNode = bindingPath.node;

      // Handle import specifiers
      if (bindingNode.type === 'ImportDefaultSpecifier') {
        return evaluateImportBinding(bindingPath, scope, seen);
      }
      if (bindingNode.type === 'ImportNamespaceSpecifier') {
        return evaluateImportBinding(bindingPath, scope, seen);
      }
      if (bindingNode.type === 'ImportSpecifier') {
        return evaluateImportBinding(bindingPath, scope, seen);
      }

      // Variable declarator
      if (bindingNode.type === 'VariableDeclarator') {
        const init: ?ESNode = (bindingNode.init: $FlowFixMe);
        if (init == null) {
          return FAILED;
        }
        seen.add(node);
        const result = evaluateNode(init, bindingPath.scope, seen);
        seen.delete(node);
        return result;
      }

      return FAILED;
    }

    case 'ArrowFunctionExpression': {
      // Only handle simple pure arrow functions: (args) => expression
      // with no body block and no rest/default params
      const params: $ReadOnlyArray<ESNode> = (node.params: $FlowFixMe);
      const body: ESNode = (node.body: $FlowFixMe);
      if (body.type === 'BlockStatement') {
        return FAILED;
      }
      for (const param of params) {
        if (param.type !== 'Identifier') {
          return FAILED;
        }
      }
      // Return the function as a callable that evaluates the body
      const fn = (...args: $ReadOnlyArray<mixed>): mixed => {
        const paramNames: $ReadOnlyArray<string> = params.map(
          (p) => (p.name: $FlowFixMe),
        );
        const fakeScope = createFakeScope(scope, paramNames, args);
        const result = evaluateNode(body, fakeScope, new Set());
        if (!result.confident) {
          throw new Error('Could not evaluate arrow function body');
        }
        return result.value;
      };
      return confident(fn);
    }

    default:
      return FAILED;
  }
}

function createFakeScope(
  parentScope: Scope,
  paramNames: $ReadOnlyArray<string>,
  args: $ReadOnlyArray<mixed>,
): Scope {
  return {
    getBinding(name: string) {
      const idx = paramNames.indexOf(name);
      if (idx !== -1) {
        const value = args[idx];
        // Create a synthetic binding path for a literal value
        const syntheticNode: ESNode = {
          type: 'Literal',
          value,
        };
        const syntheticPath: NodePath = {
          node: syntheticNode,
          scope: parentScope,
          parentPath: null,
        };
        return { path: syntheticPath };
      }
      return parentScope.getBinding(name);
    },
  };
}

function evaluateImportBinding(
  bindingPath: NodePath,
  _scope: Scope,
  _seen: Set<ESNode>,
): EvaluationResult {
  const specifierNode: ESNode = bindingPath.node;
  const importDeclaration: ?NodePath = bindingPath.parentPath;
  if (importDeclaration == null) {
    return FAILED;
  }
  const importNode: ESNode = importDeclaration.node;
  if (importNode.type !== 'ImportDeclaration') {
    return FAILED;
  }

  const source: string = (importNode.source: $FlowFixMe).value;

  // Only handle imports from .stylex.js / .stylex.ts files
  if (!isStyleXFile(source)) {
    return FAILED;
  }

  if (specifierNode.type === 'ImportDefaultSpecifier') {
    // default import from a stylex file — return a namespace-like object
    // We can't resolve the actual file at lint time without the filesystem,
    // so we return a sentinel object keyed by source
    return confident({ __stylexImport: source, __default: true });
  }

  if (specifierNode.type === 'ImportNamespaceSpecifier') {
    return confident({ __stylexImport: source, __namespace: true });
  }

  if (specifierNode.type === 'ImportSpecifier') {
    const importedNode: ESNode = (specifierNode.imported: $FlowFixMe);
    const importedName: string =
      importedNode.type === 'Identifier'
        ? (importedNode.name: $FlowFixMe)
        : (importedNode.value: $FlowFixMe);
    return confident({
      __stylexImport: source,
      __exportName: importedName,
    });
  }

  return FAILED;
}

function isStyleXFile(source: string): boolean {
  return (
    source.endsWith('.stylex') ||
    source.endsWith('.stylex.js') ||
    source.endsWith('.stylex.ts') ||
    source.endsWith('.stylex.jsx') ||
    source.endsWith('.stylex.tsx')
  );
}

module.exports = { evaluatePath, evaluateNode, isStyleXFile };
