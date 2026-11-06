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
  RuleCheck,
  RuleResponse,
  Variables,
} from '../stylex-valid-styles';
import type { Node, Property } from 'estree';
/*:: import { Rule } from 'eslint'; */
import makeVariableCheckingRule from '../utils/makeVariableCheckingRule';
import getSourceCode from '../utils/getSourceCode';

const numericOperators = new Set(['+', '-', '*', '/']);
const mathFunctions = new Set(['abs', 'ceil', 'floor', 'round']);

export const isNumber: RuleCheck = makeVariableCheckingRule(
  (
    node: Node,
    variables?: Variables,
    prop?: $ReadOnly<Property>,
    context?: Rule.RuleContext,
  ): RuleResponse => {
    if (node.type === 'Literal' && typeof node.value === 'number') {
      return undefined;
    }

    if (node.type === 'Identifier' && context) {
      // $FlowFixMe[prop-missing] Flow libdefs doesn't know SourceCode has `getScope`
      const scope = getSourceCode(context).getScope(node);
      const variable = scope.set.get(node.name);
      const def = variable?.defs?.[0];

      const isLocalConst =
        def?.node?.type === 'VariableDeclarator' &&
        def.parent?.kind === 'const';

      return isLocalConst || variables?.get(node.name)?.type === 'number'
        ? undefined
        : { message: 'a number literal or math expression' };
    }

    if (node.type === 'UnaryExpression' && node.operator === '-') {
      return isNumber(node.argument, variables, prop, context);
    }

    if (
      node.type === 'BinaryExpression' &&
      numericOperators.has(node.operator)
    ) {
      const left = isNumber(node.left, variables, prop, context);
      const right = isNumber(node.right, variables, prop, context);
      return left === undefined && right === undefined
        ? undefined
        : { message: 'a number literal or math expression' };
    }

    if (node.type === 'MemberExpression' && node.object.type === 'Identifier') {
      return undefined;
    }

    if (node.type === 'CallExpression') {
      return undefined;
    }

    if (isMathCall(node)) {
      return undefined;
    }

    return { message: 'a number literal or math expression' };
  },
);

export function isMathCall(node: Node): boolean {
  return (
    node.type === 'CallExpression' &&
    node.callee.type === 'MemberExpression' &&
    node.callee.object.type === 'Identifier' &&
    node.callee.object.name === 'Math' &&
    node.callee.property.type === 'Identifier' &&
    mathFunctions.has(node.callee.property.name) &&
    node.arguments.length === 1
  );
}

export default isNumber;
