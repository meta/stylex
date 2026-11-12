/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { Node } from 'estree';
import createImportTracker from './utils/createImportTracker';
/*:: import { Rule } from 'eslint'; */

type JSXIdentifier = {
  +type: 'JSXIdentifier',
  +name: string,
};

type JSXExpressionContainer = {
  +type: 'JSXExpressionContainer',
  +expression: Node | { +type: 'JSXEmptyExpression' },
};

type JSXAttribute = {
  +type: 'JSXAttribute',
  +name: JSXIdentifier | { +type: string },
  +value?: Node | JSXExpressionContainer | null,
};

type JSXSpreadAttribute = {
  +type: 'JSXSpreadAttribute',
  +argument: Node,
};

type JSXOpeningElement = {
  +type: 'JSXOpeningElement',
  +name: JSXIdentifier | { +type: string },
  +attributes: $ReadOnlyArray<JSXAttribute | JSXSpreadAttribute>,
};

const STYLEX_PROPS_CONFLICTING_PROPS_MESSAGE =
  'The `{{propName}}` prop should not be used when spreading `stylex.props()` to avoid conflicts.';

const STYLEX_SHORTHAND_CONFLICTING_PROPS_MESSAGE =
  'The `{{propName}}` prop should not be used with the `{{sxPropName}}` StyleX prop to avoid conflicts.';

const stylexNoConflictingProps = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Disallow using `className` or `style` props on elements that spread `stylex.props()` or use StyleX JSX shorthand',
      category: 'Best Practices',
      recommended: true,
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
          sxPropName: {
            oneOf: [{ type: 'string' }, { enum: [false] }],
            default: 'sx',
          },
        },
        additionalProperties: false,
      },
    ],
  },
  create(context: Rule.RuleContext): { ... } {
    const {
      validImports: importsToLookFor = ['stylex', '@stylexjs/stylex'],
      sxPropName = 'sx',
    } = context.options[0] || {};

    const importTracker = createImportTracker(importsToLookFor);

    function isStylexPropsCallee(node: Node) {
      return (
        (node.type === 'MemberExpression' &&
          node.object.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(node.object.name) &&
          node.property.type === 'Identifier' &&
          node.property.name === 'props') ||
        (node.type === 'Identifier' &&
          importTracker.isStylexNamedImport('props', node.name))
      );
    }

    function isLowercaseHostElement(node: JSXOpeningElement): boolean {
      return (
        node.name.type === 'JSXIdentifier' &&
        typeof node.name.name === 'string' &&
        node.name.name[0] === node.name.name[0].toLowerCase()
      );
    }

    function hasStylexShorthandProp(node: JSXOpeningElement): boolean {
      return (
        typeof sxPropName === 'string' &&
        isLowercaseHostElement(node) &&
        node.attributes.some(
          (attr) =>
            attr.type === 'JSXAttribute' &&
            attr.name.type === 'JSXIdentifier' &&
            attr.name.name === sxPropName &&
            attr.value != null &&
            attr.value.type === 'JSXExpressionContainer' &&
            attr.value.expression.type !== 'JSXEmptyExpression',
        )
      );
    }

    return {
      ImportDeclaration: importTracker.ImportDeclaration,

      JSXOpeningElement(node: JSXOpeningElement) {
        const hasStylexPropsSpread = node.attributes.some(
          (attr) =>
            attr.type === 'JSXSpreadAttribute' &&
            attr.argument.type === 'CallExpression' &&
            isStylexPropsCallee(attr.argument.callee),
        );

        const hasStylexShorthand = hasStylexShorthandProp(node);

        if (!hasStylexPropsSpread && !hasStylexShorthand) {
          return;
        }

        const message = hasStylexShorthand
          ? STYLEX_SHORTHAND_CONFLICTING_PROPS_MESSAGE
          : STYLEX_PROPS_CONFLICTING_PROPS_MESSAGE;

        for (const attr of node.attributes) {
          if (
            attr.type === 'JSXAttribute' &&
            attr.name.type === 'JSXIdentifier' &&
            (attr.name.name === 'className' || attr.name.name === 'style')
          ) {
            context.report({
              // $FlowFixMe[incompatible-type]
              node: attr,
              message,
              data: { propName: attr.name.name, sxPropName },
            });
          } else if (
            attr.type === 'JSXSpreadAttribute' &&
            attr.argument.type === 'ObjectExpression'
          ) {
            for (const prop of attr.argument.properties) {
              if (
                prop.type === 'Property' &&
                !prop.computed &&
                prop.key.type === 'Identifier' &&
                (prop.key.name === 'className' || prop.key.name === 'style')
              ) {
                context.report({
                  // $FlowFixMe[incompatible-type]
                  node: prop,
                  message,
                  data: { propName: prop.key.name, sxPropName },
                });
              }
            }
          }
        }
      },

      'Program:exit'() {
        importTracker.clear();
      },
    };
  },
};

export default stylexNoConflictingProps;
