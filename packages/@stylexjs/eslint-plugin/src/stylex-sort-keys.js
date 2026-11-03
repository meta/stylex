/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { Token } from 'eslint/eslint-ast';
import type { RuleFixer, SourceCode } from 'eslint/eslint-rule';
import type {
  CallExpression,
  Node,
  Property,
  SpreadElement,
  ObjectExpression,
  Comment,
} from 'estree';
import getSourceCode from './utils/getSourceCode';
import getPropertyName from './utils/getPropertyName';
import getPropertyPriorityAndType from './utils/getPropertyPriorityAndType';
import createImportTracker from './utils/createImportTracker';
/*:: import { Rule } from 'eslint'; */

type Schema = {
  validImports: Array<
    | string
    | {
        from: string,
        as: string,
      },
  >,
  order: 'default' | 'clean' | 'recess',
  minKeys: number,
  allowLineSeparatedGroups: boolean,
  enableMediaQueryOrder: boolean,
};

type Stack = null | {
  upper: Stack,
  prevNode: $ReadOnly<{ ...Property, ... }> | null,
  prevName: string | null,
  prevBlankLine: boolean,
  numKeys: number,
};

type SortableProperty = {
  node: Property,
  name: string,
  text: string,
  rangeStart: number,
  rangeEnd: number,
};

function isMediaQuery(name: string): boolean {
  return typeof name === 'string' && name.startsWith('@media');
}

function isValidOrder(
  prevName: string,
  currName: string,
  order: Schema['order'],
  enableMediaQueryOrder: boolean,
): boolean {
  // When `enableMediaQueryOrder` is set, StyleX resolves overlapping media
  // queries by source order ("last media query wins"). Sorting `@media`
  // condition keys alphabetically inverts that cascade, so we treat any
  // authored order between two media queries as valid and leave it untouched.
  if (
    enableMediaQueryOrder &&
    isMediaQuery(prevName) &&
    isMediaQuery(currName)
  ) {
    return true;
  }

  const prev = getPropertyPriorityAndType(prevName, order);
  const curr = getPropertyPriorityAndType(currName, order);

  if (prev.type !== 'string' || curr.type !== 'string') {
    if (prev.priority === curr.priority) return prevName <= currName;
    return prev.priority <= curr.priority;
  }

  return prevName <= currName;
}

function comparePropertyNames(
  aName: string,
  bName: string,
  order: Schema['order'],
  enableMediaQueryOrder: boolean,
): number {
  if (aName === bName) {
    return 0;
  }

  // Preserve the authored relative order of media queries so the autofix does
  // not reshuffle them (the sort below is stable and falls back to the
  // original index when this returns 0).
  if (enableMediaQueryOrder && isMediaQuery(aName) && isMediaQuery(bName)) {
    return 0;
  }

  return isValidOrder(aName, bName, order, enableMediaQueryOrder) ? -1 : 1;
}

const stylexSortKeys = {
  meta: {
    type: 'suggestion',
    docs: {
      description: 'Require style properties to be sorted by key',
      recommended: false,
      url: 'https://github.com/facebook/stylex/tree/main/packages/@stylexjs/eslint-plugin',
    },
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
          order: {
            enum: ['default', 'clean', 'recess'],
            default: 'default',
          },
          minKeys: {
            type: 'integer',
            minimum: 2,
            default: 2,
          },
          allowLineSeparatedGroups: {
            type: 'boolean',
            default: false,
          },
          enableMediaQueryOrder: {
            type: 'boolean',
            default: false,
          },
        },
        additionalProperties: false,
      },
    ],
  },
  create(context: Rule.RuleContext): { ... } {
    const {
      validImports: importsToLookFor = ['stylex', '@stylexjs/stylex'],
      order = 'default',
      minKeys = 2,
      allowLineSeparatedGroups = false,
      enableMediaQueryOrder = false,
    }: Schema = context.options[0] || {};

    const importTracker = createImportTracker(importsToLookFor);

    function isStylexCallee(node: Node) {
      return (
        (node.type === 'MemberExpression' &&
          node.object.type === 'Identifier' &&
          importTracker.isStylexDefaultImport(node.object.name) &&
          node.property.type === 'Identifier' &&
          (node.property.name === 'create' ||
            node.property.name === 'keyframes')) ||
        (node.type === 'Identifier' &&
          (importTracker.isStylexNamedImport('create', node.name) ||
            importTracker.isStylexNamedImport('keyframes', node.name)))
      );
    }

    function isStylexDeclaration(node: $ReadOnly<{ ...Node, ... }>) {
      return (
        node &&
        node.type === 'CallExpression' &&
        isStylexCallee(node.callee) &&
        node.arguments.length === 1
      );
    }

    let stack: Stack = null;
    let isInsideStyleXCreateCall = false;
    let objectExpressionNestingLevel = -1;

    return {
      ImportDeclaration: importTracker.ImportDeclaration,
      CallExpression(
        node: $ReadOnly<{ ...CallExpression, ...Rule.NodeParentExtension }>,
      ) {
        if (
          !isStylexDeclaration(node) ||
          !node.arguments[0].properties ||
          node.arguments[0].properties.length === 0
        ) {
          return;
        }

        isInsideStyleXCreateCall = true;
      },
      ObjectExpression(node: ObjectExpression) {
        if (isInsideStyleXCreateCall) {
          objectExpressionNestingLevel++;
        }

        if (objectExpressionNestingLevel > 0) {
          stack = {
            upper: stack,
            prevNode: null,
            prevName: null,
            prevBlankLine: false,
            numKeys: node.properties.length,
          };
        }
      },
      'ObjectExpression:exit'() {
        if (
          isInsideStyleXCreateCall &&
          objectExpressionNestingLevel > 0 &&
          stack
        ) {
          stack = stack.upper;
        }

        if (isInsideStyleXCreateCall) {
          objectExpressionNestingLevel--;
        }
      },
      SpreadElement(
        node: $ReadOnly<{ ...SpreadElement, ...Rule.NodeParentExtension }>,
      ) {
        if (
          isInsideStyleXCreateCall &&
          objectExpressionNestingLevel > 0 &&
          (node.parent.type as $FlowFixMe) === 'ObjectExpression' &&
          stack
        ) {
          stack.prevName = null;
        }
      },
      Property(node: $ReadOnly<{ ...Property, ...Rule.NodeParentExtension }>) {
        if (
          !isInsideStyleXCreateCall ||
          objectExpressionNestingLevel < 1 ||
          (node.parent.type as $FlowFixMe) === 'ObjectPattern' ||
          stack === null
        ) {
          return;
        }

        const prevName = stack.prevName;
        const numKeys = stack.numKeys;
        const currName = getPropertyName(node);
        let isBlankLineBetweenNodes = stack?.prevBlankLine;

        const sourceCode = getSourceCode(context);

        const tokens =
          stack?.prevNode &&
          sourceCode.getTokensBetween(stack.prevNode, node, {
            includeComments: true,
          });

        if (tokens && tokens.length > 0) {
          tokens.forEach((token, index) => {
            const previousToken = tokens[index - 1];

            if (
              previousToken &&
              token.loc &&
              previousToken.loc &&
              token.loc.start.line - previousToken.loc.end.line > 1
            ) {
              isBlankLineBetweenNodes = true;
            }
          });

          if (
            !isBlankLineBetweenNodes &&
            (node.loc?.start?.line ?? 0) - (tokens.at(-1)?.loc?.end.line ?? 0) >
              1
          ) {
            isBlankLineBetweenNodes = true;
          }

          if (
            !isBlankLineBetweenNodes &&
            tokens[0].loc &&
            stack?.prevNode?.loc &&
            tokens[0].loc.start.line - stack?.prevNode?.loc?.end.line > 1
          ) {
            isBlankLineBetweenNodes = true;
          }
        }

        if (stack) {
          stack.prevNode = node;
        }

        if (currName !== null && stack) {
          stack.prevName = currName;
        }

        if (allowLineSeparatedGroups && isBlankLineBetweenNodes && stack) {
          stack.prevBlankLine = currName === null;
          return;
        }

        if (prevName === null || currName === null || numKeys < minKeys) {
          return;
        }

        if (!isValidOrder(prevName, currName, order, enableMediaQueryOrder)) {
          context.report({
            // $FlowFixMe[incompatible-type]
            node,
            loc: node.key.loc,
            message: `StyleX property key "${currName}" should be above "${prevName}"`,
            // $FlowFixMe[incompatible-type]
            fix: createFix({
              currNode: node as $FlowFixMe,
              sourceCode,
              order,
              allowLineSeparatedGroups,
              enableMediaQueryOrder,
            }),
          });
        }
      },
      'CallExpression:exit'(
        node: $ReadOnly<{ ...CallExpression, ...Rule.NodeParentExtension }>,
      ) {
        if (isInsideStyleXCreateCall && isStylexDeclaration(node)) {
          isInsideStyleXCreateCall = false;
        }
      },
      'Program:exit'() {
        importTracker.clear();
      },
    };
  },
};

function createFix({
  currNode,
  sourceCode,
  order,
  allowLineSeparatedGroups,
  enableMediaQueryOrder,
}: {
  currNode: Property,
  sourceCode: SourceCode,
  order: Schema['order'],
  allowLineSeparatedGroups: boolean,
  enableMediaQueryOrder: boolean,
}) {
  return function (fixer: RuleFixer) {
    const group = getSortableGroup(currNode);

    if (group.length < 2) {
      return [];
    }

    const sortedGroup = [...group].sort((a, b) => {
      const result = comparePropertyNames(
        a.name,
        b.name,
        order,
        enableMediaQueryOrder,
      );

      return result === 0 ? group.indexOf(a) - group.indexOf(b) : result;
    });

    if (sortedGroup.every((item, index) => item === group[index])) {
      return [];
    }

    const firstItem = group[0];
    const lastItem = group[group.length - 1];
    const replacementText = isInlineGroup(group)
      ? getInlineReplacementText(sortedGroup, firstItem.text)
      : sortedGroup.map((item) => item.text).join('\n');

    return fixer.replaceTextRange(
      [firstItem.rangeStart, lastItem.rangeEnd],
      replacementText,
    );
  };

  function getSortableGroup(node: Property): SortableProperty[] {
    const parent = (node as $FlowFixMe).parent;

    if (!parent || parent.type !== 'ObjectExpression') {
      return [];
    }

    const groups: Array<SortableProperty[]> = [];
    let group: SortableProperty[] = [];

    for (const property of parent.properties as $FlowFixMe) {
      if (property.type !== 'Property') {
        if (group.length > 0) {
          groups.push(group);
          group = [];
        }
        continue;
      }

      const name = getPropertyName(property);
      if (name === null) {
        if (group.length > 0) {
          groups.push(group);
          group = [];
        }
        continue;
      }

      const sortableProperty = getSortableProperty(property, name);
      if (sortableProperty === null) {
        if (group.length > 0) {
          groups.push(group);
          group = [];
        }
        continue;
      }

      if (
        group.length > 0 &&
        isGroupBoundary(group[group.length - 1], sortableProperty)
      ) {
        groups.push(group);
        group = [];
      }

      group.push(sortableProperty);
    }

    if (group.length > 0) {
      groups.push(group);
    }

    return (
      groups.find((sortableGroup) =>
        sortableGroup.some((item) => item.node === node),
      ) ?? []
    );
  }

  function getSortableProperty(
    node: Property,
    name: string,
  ): SortableProperty | null {
    const commentsBefore = getPropertyCommentsBefore(node);
    const contextStartNode =
      commentsBefore.length > 0 ? commentsBefore[0] : node;
    const { indentation } = getNodeIndentation(contextStartNode);
    const sameLineComment = getPropertySameLineComment(node);
    const tokenAfterNode = sourceCode.getTokenAfter(node, {
      includeComments: false,
    });
    const hasCommaAfterNode = tokenAfterNode && isCommaToken(tokenAfterNode);
    const contextEndNode =
      sameLineComment ?? (hasCommaAfterNode ? tokenAfterNode : node);
    const contextStartRange = contextStartNode.range;
    const contextEndRange = contextEndNode?.range;
    const nodeRange = node.range;

    if (!contextStartRange || !contextEndRange || !nodeRange) {
      return null;
    }

    const rangeStart = contextStartRange[0] - indentation.length;
    const rangeEnd = contextEndRange[1];
    const sourceText = sourceCode.getText();
    const commaInsertIndex = nodeRange[1] - rangeStart;
    let text = sourceText.slice(rangeStart, rangeEnd);

    if (!hasCommaAfterNode) {
      text =
        text.slice(0, commaInsertIndex) + ',' + text.slice(commaInsertIndex);
    }

    return {
      node,
      name,
      text,
      rangeStart,
      rangeEnd,
    };
  }

  function isGroupBoundary(
    prevProperty: SortableProperty,
    currProperty: SortableProperty,
  ): boolean {
    const textBetweenProperties = sourceCode
      .getText()
      .slice(prevProperty.rangeEnd, currProperty.rangeStart);

    if (/[^ \t\r\n]/.test(textBetweenProperties)) {
      return true;
    }

    return (
      allowLineSeparatedGroups &&
      /(?:\r?\n)[ \t]*(?:\r?\n)/.test(textBetweenProperties)
    );
  }

  function isInlineGroup(group: SortableProperty[]): boolean {
    const firstItem = group[0];
    const lastItem = group[group.length - 1];

    return !/[\r\n]/.test(
      sourceCode.getText().slice(firstItem.rangeStart, lastItem.rangeEnd),
    );
  }

  function getInlineReplacementText(
    sortedGroup: SortableProperty[],
    firstText: string,
  ): string {
    const leadingWhitespace = firstText.match(/^[ \t]*/)?.[0] ?? '';

    return (
      leadingWhitespace +
      sortedGroup.map((item) => item.text.trimStart()).join(' ')
    );
  }

  function getEmptyLineCountBetweenNodes(
    aNode: Property | Comment,
    bNode: Property | Comment,
  ): number {
    const [upperNode, lowerNode] = [aNode, bNode].sort(
      (a, b) => (a.loc?.start.line ?? 0) - (b.loc?.start.line ?? 0),
    );

    const upperNodeLine = upperNode.loc?.start.line;
    const lowerNodeLine = lowerNode.loc?.start.line;

    if (upperNodeLine === undefined || lowerNodeLine === undefined) {
      throw new Error('Invalid node location');
    }

    return sourceCode.lines
      .slice(upperNodeLine, lowerNodeLine - 1)
      .filter((line) => /^[ \t]*$/.test(line)).length;
  }

  function getPropertyCommentsBefore(node: Property): Comment[] {
    return sourceCode.getCommentsBefore(node).filter((comment) => {
      const tokenBefore = sourceCode.getTokenBefore(comment, {
        includeComments: false,
      });

      if (tokenBefore === null) {
        return true;
      }

      // Only comments that have no other tokens on the same line are considered
      // Also, comments that have at least one empty line between node and comment will be ignored
      // For example:
      //
      //  create({
      //    foo: { // comment above a <- this comment does not belong to property below
      //      // comment above b <- this comment does not belong to property below
      //
      //      // comment above c <- this comment belongs to property below
      //      display: 'red'
      //    }
      //  })
      //
      return (
        !isSameLine(tokenBefore, comment) &&
        getEmptyLineCountBetweenNodes(node, comment) === 0
      );
    });
  }

  function getPropertySameLineComment(node: Property): Comment | void {
    const tokenAfter = sourceCode.getTokenAfter(node, {
      includeComments: false,
    });

    const comments = sourceCode
      .getCommentsAfter(
        tokenAfter && isCommaToken(tokenAfter) ? tokenAfter : node,
      )
      .filter((comment) => isSameLine(node, comment));

    return comments[0];
  }

  function getNodeIndentation(node: Property | Comment): {
    indentation: string,
    isTokenBeforeSameLineAsNode: boolean,
  } {
    const tokenBefore = sourceCode.getTokenBefore(node, {
      includeComments: false,
    });

    const isTokenBeforeSameLineAsNode =
      !!tokenBefore && isSameLine(tokenBefore, node);

    const sliceStart =
      isTokenBeforeSameLineAsNode && tokenBefore?.loc
        ? tokenBefore.loc.end.column
        : 0;

    return {
      isTokenBeforeSameLineAsNode,
      indentation: node?.loc
        ? sourceCode.lines[node.loc.start.line - 1].slice(
            sliceStart,
            node.loc.start.column,
          )
        : '',
    };
  }
}

function isSameLine(
  aNode: Property | Comment | Token,
  bNode: Property | Comment | Token,
): boolean {
  return Boolean(
    aNode.loc && bNode.loc && aNode.loc?.start.line === bNode.loc?.start.line,
  );
}

function isCommaToken(token: Token): boolean {
  return token.type === 'Punctuator' && token.value === ',';
}

export default stylexSortKeys as typeof stylexSortKeys;
