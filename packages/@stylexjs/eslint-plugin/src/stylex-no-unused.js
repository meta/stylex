/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type {
  CallExpression,
  Expression,
  Node,
  Program,
  Property,
  SpreadElement,
  RestElement,
  MemberExpression,
  AssignmentProperty,
  ExportDefaultDeclaration,
  ExportNamedDeclaration,
  ReturnStatement,
  ImportDeclaration,
} from 'estree';
import getSourceCode from './utils/getSourceCode';
import createImportTracker from './utils/createImportTracker';
/*:: import { Rule } from 'eslint'; */

type PropertyValue =
  | Property
  | SpreadElement
  | AssignmentProperty
  | RestElement;

function getPropertiesByName(node: Node | null) {
  const properties = new Map<any, PropertyValue>();
  if (node == null) {
    return properties;
  }
  node.properties
    ?.filter((property) => !property.computed && !property.method)
    .forEach((property) => {
      const { key } = property;
      if (key?.type === 'Identifier') {
        properties.set(key.name, property);
      } else if (key?.type === 'Literal') {
        properties.set(key.value, property);
      }
    });
  return properties;
}

const stylexNoUnused = {
  meta: {
    fixable: 'code',
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
    const stylexProperties = new Map<string, Map<any, PropertyValue>>();

    function isStylexCreate(node: Node) {
      return (
        // const styles = s.create({...})   OR    const styles = stylex.create({...})
        (node.type === 'MemberExpression' &&
          node.object.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(node.object.name) &&
          node.property.type === 'Identifier' &&
          node.property.name === 'create') ||
        // const styles = c({...})   OR   const styles = create({...})
        (node.type === 'Identifier' &&
          importTracker.isStylexNamedImport('create', node.name))
      );
    }

    function isStylexDeclaration(node: Node) {
      return (
        node &&
        node.type === 'CallExpression' &&
        isStylexCreate(node.callee) &&
        node.arguments.length === 1 &&
        node.arguments[0].type === 'ObjectExpression'
      );
    }

    function saveStylexCalls(node: Node) {
      const id = node.id;
      const init = node.init;
      if (id && id.type === 'Identifier' && init && isStylexDeclaration(init)) {
        stylexProperties.set(
          id.name,
          getPropertiesByName(
            init.arguments && init.arguments?.length > 0
              ? init.arguments[0]
              : null,
          ),
        );
      }
    }
    function checkArguments(
      namespaces: Map<any, PropertyValue>,
    ): (argument: Expression | SpreadElement | null) => void {
      return function (argument: Expression | SpreadElement | null): void {
        if (argument) {
          if (argument.type === 'Literal') {
            namespaces.delete(argument.value);
          } else if (argument.type === 'ObjectExpression') {
            argument.properties.forEach((property) => {
              if (property.key) {
                namespaces.delete(property.key.name);
              }
            });
          } else if (argument.type === 'ArrayExpression') {
            argument.elements.forEach((element) => {
              namespaces.delete(element?.value);
            });
          } else if (argument.type === 'ConditionalExpression') {
            const { consequent, alternate } = argument;
            // check for nested expressions
            checkArguments(namespaces)(consequent);
            checkArguments(namespaces)(alternate);
          } else if (
            argument.type === 'LogicalExpression' &&
            argument.operator === '&&'
          ) {
            // check for nested expressions but only on the right
            checkArguments(namespaces)(argument.right);
          }
        }
      };
    }

    return {
      Program(node: Program) {
        // Process imports first
        node.body
          .filter(
            (node): node is ImportDeclaration =>
              node.type === 'ImportDeclaration',
          )
          .forEach(importTracker.ImportDeclaration);

        // Then process stylex.create declarations
        node.body
          .filter(({ type }) => type === 'VariableDeclaration')
          .map(({ declarations }) =>
            declarations && declarations.length === 1 ? declarations[0] : null,
          )
          .filter(Boolean)
          .filter(({ init }) => init && isStylexDeclaration(init))
          .forEach(saveStylexCalls);
      },

      // Exempt used styles: "stylex.__" or "styles[__]"
      MemberExpression(node: MemberExpression) {
        if (
          node.object.type === 'Identifier' &&
          stylexProperties.has(node.object.name)
        ) {
          if (node.computed && node.property.type !== 'Literal') {
            stylexProperties.delete(node.object.name);
          } else if (node.property.type === 'Identifier') {
            stylexProperties.get(node.object.name)?.delete(node.property.name);
          } else if (node.property.type === 'Literal') {
            stylexProperties.get(node.object.name)?.delete(node.property.value);
          }
        }
      },
      // catch function call "functionName(param)"
      CallExpression(node: CallExpression) {
        const functionName = node.callee?.name;
        if (functionName == null || !stylexProperties.has(functionName)) {
          return;
        }
        const namespaces = stylexProperties.get(functionName);
        if (namespaces == null) {
          return;
        }
        node.arguments?.forEach(checkArguments(namespaces));
      },

      // Exempt used styles: export const exportStyles = stylex.create({});
      ExportNamedDeclaration(node: ExportNamedDeclaration) {
        // Exempt used styles: export { exportStyles };
        // Skip re-exports and type-only exports: no local value is exported
        if (node.source == null && (node as $FlowFixMe).exportKind !== 'type') {
          for (const specifier of node.specifiers) {
            // Skip type-only specifier: export { type exportStyles };
            if ((specifier as $FlowFixMe).exportKind === 'type') {
              continue;
            }
            stylexProperties.delete(specifier.local.name);
          }
        }

        const declarations = node.declaration?.declarations;
        if (declarations?.length !== 1) {
          return;
        }
        const exportName = declarations[0].id.name;
        if (exportName == null || !stylexProperties.has(exportName)) {
          return;
        }
        stylexProperties.delete(exportName);
      },

      // Exempt used styles: export default exportStyles;
      ExportDefaultDeclaration(node: ExportDefaultDeclaration) {
        const exportName = node.declaration.name;
        if (exportName == null || !stylexProperties.has(exportName)) {
          return;
        }
        stylexProperties.delete(exportName);
      },

      // Exempt used styles: used as return
      ReturnStatement(node: ReturnStatement) {
        if (node.argument?.type === 'Identifier') {
          const returnName = node.argument.name;
          if (stylexProperties.has(node.argument.name)) {
            stylexProperties.delete(returnName);
          }
        }
      },

      'Program:exit'() {
        const sourceCode = getSourceCode(context);

        stylexProperties.forEach((namespaces, varName) => {
          namespaces.forEach((node, namespaceName) => {
            context.report({
              node,
              message: `Unused style detected: ${varName}.${namespaceName}`,
              fix(fixer) {
                const commaOffset =
                  sourceCode.getTokenAfter(node, {
                    includeComments: false,
                  })?.value === ','
                    ? 1
                    : 0;
                const left = sourceCode.getTokenBefore(node, {
                  includeComments: false,
                });
                if (node.range == null || left?.range == null) {
                  return null;
                }
                return fixer.removeRange([
                  left.range[1],
                  node.range[1] + commaOffset,
                ]);
              },
            });
          });
        });

        stylexProperties.clear();
        importTracker.clear();
      },
    };
  },
};

export default stylexNoUnused as typeof stylexNoUnused;
