/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { CallExpression, Node } from 'estree';
import createImportTracker from './utils/createImportTracker';
import getSourceCode from './utils/getSourceCode';
/*:: import { Rule } from 'eslint'; */

// StyleX APIs that produce static, compiled-away definitions. This rule is
// intentionally stricter than the compiler — which can hoist some of these out
// of nested scopes — because definitions should be declared once at module
// scope.
const ENFORCED_APIS = new Set([
  'create',
  'createTheme',
  'keyframes',
  'defineVars',
  'defineConsts',
  'defineMarker',
  'positionTry',
  'viewTransitionClass',
  'unstable_defineVarsNested',
  'unstable_defineConstsNested',
  'unstable_createThemeNested',
]);

const stylexEnforceModuleScope = {
  meta: {
    type: 'suggestion',
    docs: {
      description:
        'Enforce that statically-compiled StyleX APIs (stylex.create, stylex.keyframes, stylex.defineVars, etc.) are called at module scope',
      category: 'Possible Errors',
      recommended: false,
    },
    schema: [
      {
        type: 'object',
        properties: {
          validImports: {
            type: 'array',
            items: {
              oneOf: [
                { type: 'string' },
                {
                  type: 'object',
                  properties: {
                    from: { type: 'string' },
                    as: { type: 'string' },
                  },
                },
              ],
            },
            default: ['stylex', '@stylexjs/stylex'],
          },
        },
        additionalProperties: false,
      },
    ],
  },
  create(context: Rule.RuleContext): { ... } {
    const { validImports: importsToLookFor = ['stylex', '@stylexjs/stylex'] } =
      context.options[0] || {};

    const importTracker = createImportTracker(importsToLookFor);
    const sourceCode = getSourceCode(context);

    function getStylexApiName(callee: Node): string | null {
      // stylex.create(...) — default/namespace import member access
      if (
        callee.type === 'MemberExpression' &&
        !callee.computed &&
        callee.object.type === 'Identifier' &&
        importTracker.isStylexDefaultImport(callee.object.name) &&
        callee.property.type === 'Identifier' &&
        ENFORCED_APIS.has(callee.property.name)
      ) {
        return callee.property.name;
      }

      // create(...) — named import (optionally aliased)
      if (callee.type === 'Identifier') {
        for (const api of ENFORCED_APIS) {
          if (importTracker.isStylexNamedImport(api, callee.name)) {
            return api;
          }
        }
      }

      return null;
    }

    function isAtModuleScope(node: Node): boolean {
      // variableScope is the nearest enclosing function/module/global scope.
      // A class field initializer and a class static block each report as their
      // own variable scope, so they are correctly treated as non-module.
      // TypeScript namespaces (`tsModule`) can only appear at the top level of
      // a file or another namespace, so they are treated as module scope.
      // $FlowFixMe[prop-missing] Flow libdefs doesn't know SourceCode has `getScope`
      const { type } = sourceCode.getScope(node).variableScope;
      return type === 'module' || type === 'global' || type === 'tsModule';
    }

    return {
      ImportDeclaration: importTracker.ImportDeclaration,

      CallExpression(node: CallExpression) {
        const api = getStylexApiName(node.callee);

        if (api && !isAtModuleScope(node)) {
          context.report({
            node,
            message: `stylex.${api}() must be called at module scope, not inside a function, component, or class.`,
          });
        }
      },

      'Program:exit'() {
        importTracker.clear();
      },
    };
  },
};

export default stylexEnforceModuleScope as typeof stylexEnforceModuleScope;
