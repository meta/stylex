/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { Node, CallExpression, ObjectExpression, Property } from 'estree';
import type { ValidImportSource } from './utils/createImportTracker';
import createImportTracker from './utils/createImportTracker';
/*:: import { Rule } from 'eslint'; */

type Schema = {
  validImports: Array<ValidImportSource>,
  allowedMediaQueries: Array<string>,
};

const MEDIA_QUERY_PATTERN: RegExp = /^@media\b/i;

const stylexNoRawMediaQueries = {
  meta: {
    type: 'suggestion',
    docs: {
      description:
        'Disallow raw media query strings within `stylex.create` calls in favor of shared constants defined with `stylex.defineConsts`',
      recommended: false,
      url: 'https://github.com/facebook/stylex/tree/main/packages/@stylexjs/eslint-plugin',
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
          allowedMediaQueries: {
            type: 'array',
            items: { type: 'string' },
            default: [] as Array<string>,
          },
        },
        additionalProperties: false,
      },
    ],
  },
  create(context: Rule.RuleContext): { ... } {
    const {
      validImports: importsToLookFor = ['stylex', '@stylexjs/stylex'],
      allowedMediaQueries = [],
    }: Schema = context.options[0] || {};

    const importTracker = createImportTracker(importsToLookFor);
    const allowedQueries = new Set<string>(allowedMediaQueries);

    function isStylexCreateCallee(node: Node) {
      return (
        (node.type === 'MemberExpression' &&
          node.object.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(node.object.name) &&
          node.property.type === 'Identifier' &&
          node.property.name === 'create') ||
        (node.type === 'Identifier' &&
          importTracker.isStylexNamedImport('create', node.name))
      );
    }

    function isStylexCreateDeclaration(node: Node) {
      return (
        node &&
        node.type === 'CallExpression' &&
        isStylexCreateCallee(node.callee) &&
        node.arguments.length === 1
      );
    }

    // Returns the raw media query string when the key is written inline
    // (a string literal or a template literal without any expressions).
    // Keys computed from identifiers or member expressions are assumed to
    // reference shared constants and return undefined.
    function getRawMediaQueryFromKey(property: Property): string | void {
      const key = property.key;

      if (key.type === 'Literal') {
        const value = key.value;
        if (typeof value === 'string' && MEDIA_QUERY_PATTERN.test(value)) {
          return value;
        }
      }

      if (
        key.type === 'TemplateLiteral' &&
        key.expressions.length === 0 &&
        key.quasis.length === 1
      ) {
        const cooked = key.quasis[0].value.cooked;
        if (typeof cooked === 'string' && MEDIA_QUERY_PATTERN.test(cooked)) {
          return cooked;
        }
      }

      return undefined;
    }

    // Dynamic styles are declared as arrow functions that return the style
    // object (`root: (width) => ({ ... })`). Returns the returned object so
    // the rule can descend into it, mirroring `stylex-valid-styles`.
    function unwrapDynamicStyle(value: Node): Node {
      if (value.type === 'ArrowFunctionExpression') {
        let body = value.body;
        // $FlowFixMe[invalid-compare]
        if (body.type === 'TSAsExpression') {
          body = body.expression;
        }
        if (body.type === 'ObjectExpression') {
          return body;
        }
      }
      return value;
    }

    function checkObjectExpression(node: ObjectExpression): void {
      node.properties.forEach((property) => {
        if (property.type !== 'Property') {
          return;
        }

        const rawMediaQuery = getRawMediaQueryFromKey(property);
        if (rawMediaQuery != null && !allowedQueries.has(rawMediaQuery)) {
          context.report({
            node: property.key,
            message:
              'Raw media query strings are not allowed. Define this query with `stylex.defineConsts` and use the shared constant instead. See https://stylexjs.com/docs/api/javascript/defineConsts/',
          });
        }

        const value = unwrapDynamicStyle(property.value);
        if (value.type === 'ObjectExpression') {
          checkObjectExpression(value);
        }
      });
    }

    return {
      ImportDeclaration: importTracker.ImportDeclaration,

      CallExpression(node: CallExpression): void {
        const firstArg = node.arguments[0];
        if (
          isStylexCreateDeclaration(node) &&
          firstArg.type === 'ObjectExpression'
        ) {
          checkObjectExpression(firstArg);
        }
      },
      'Program:exit'() {
        importTracker.clear();
      },
    };
  },
};

export default stylexNoRawMediaQueries as typeof stylexNoRawMediaQueries;
