/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

/* eslint-disable no-unused-vars */
import type { NodePath } from '@babel/traverse';
import type { FunctionConfig } from '../utils/evaluate-path';

import * as t from '@babel/types';
import StateManager from '../utils/state-manager';
import { evaluate } from '../utils/evaluate-path';
import { create, utils } from '../shared';
import { messages } from '../shared';
import {
  timeUnits,
  lengthUnits,
  getNumberSuffix,
} from '../shared/utils/transform-value';

type TInlineStyles = {
  [string]: {
    +path: $ReadOnlyArray<string>,
    +originalExpression: t.Expression,
    +expression: t.Expression | t.PatternLike,
  },
};

type DynamicFns = {
  [string]: [
    +params: Array<t.Identifier>,
    +inlineStyles: $ReadOnly<TInlineStyles>,
  ],
};

// This
export function evaluateStyleXCreateArg(
  path: NodePath<>,
  traversalState: StateManager,
  functions: FunctionConfig = { identifiers: {}, memberExpressions: {} },
): $ReadOnly<{
  confident: boolean,
  value: any,
  deopt?: null | NodePath<>,
  reason?: string,
  fns?: DynamicFns,
}> {
  if (!path.isObjectExpression()) {
    return evaluate(path, traversalState, functions);
  }

  const value: { [string]: mixed } = {};
  const fns: DynamicFns = {};

  for (const prop of path.get('properties')) {
    if (!prop.isObjectProperty()) {
      return evaluate(path, traversalState, functions);
    }
    const objPropPath: NodePath<t.ObjectProperty> = prop;
    const keyResult = evaluateObjKey(objPropPath, traversalState, functions);
    if (!keyResult.confident) {
      return { confident: false, deopt: keyResult.deopt, value: null };
    }
    const key = keyResult.value;

    const valPath = prop.get('value');
    if (!valPath.isArrowFunctionExpression()) {
      const val = evaluate(valPath, traversalState, functions);
      if (!val.confident) {
        return val;
      }
      value[key] = val.value;
      continue;
    }
    const fnPath: NodePath<t.ArrowFunctionExpression> = valPath;
    const allParams: Array<
      NodePath<t.Identifier | t.SpreadElement | t.Pattern>,
      // $FlowFixMe[incompatible-type]
    > = fnPath.get('params');

    validateDynamicStyleParams(fnPath, allParams);

    const params: Array<t.Identifier> = allParams
      .filter(
        (
          param: NodePath<t.Identifier | t.Pattern | t.SpreadElement>,
        ): param is NodePath<t.Identifier> => param.isIdentifier(),
      )
      .map((param) => param.node);

    const fnBody = fnPath.get('body');
    if (!fnBody.isObjectExpression()) {
      // We only allow arrow functions without block bodies.
      return evaluate(path, traversalState, functions);
    }
    const fnObjectBody: NodePath<t.ObjectExpression> = fnBody;
    const evalResult = evaluatePartialObjectRecursively(
      fnObjectBody,
      traversalState,
      functions,
    );

    if (!evalResult.confident) {
      const { confident, value: v, deopt } = evalResult;
      return { confident, value: v, deopt };
    }
    const { value: v, inlineStyles } = evalResult;
    value[key] = v;
    fns[key] = [params, inlineStyles ?? {}];
  }

  return { value, confident: true, fns };
}

function evaluatePartialObjectRecursively(
  path: NodePath<t.ObjectExpression>,
  traversalState: StateManager,
  functions: FunctionConfig = { identifiers: {}, memberExpressions: {} },
  keyPath: $ReadOnlyArray<string> = [],
): $ReadOnly<{
  confident: boolean,
  value: any,
  deopt?: null | NodePath<>,
  reason?: string,
  inlineStyles?: $ReadOnly<TInlineStyles>,
}> {
  const obj: { [string]: mixed } = {};
  const inlineStyles: TInlineStyles = {};
  const props: $ReadOnlyArray<
    NodePath<t.ObjectMethod | t.ObjectProperty | t.SpreadElement>,
  > = path.get('properties');
  for (const prop of props) {
    if (prop.isObjectMethod()) {
      return { value: null, confident: false };
    }
    if (prop.isSpreadElement()) {
      const result = evaluate(prop.get('argument'), traversalState, functions);
      if (!result.confident) {
        return result;
      }
      // $FlowFixMe[unsafe-object-assign]
      Object.assign(obj, result.value);
      continue;
    }
    if (prop.isObjectProperty()) {
      const keyResult = evaluateObjKey(prop, traversalState, functions);
      if (!keyResult.confident) {
        return { confident: false, deopt: keyResult.deopt, value: null };
      }
      let key = keyResult.value;

      const valuePath: NodePath<t.Expression | t.PatternLike> =
        prop.get('value');

      if (key.startsWith('var(') && key.endsWith(')')) {
        const inner = key.slice(4, -1);
        // When the `keyPath` is not empty, the var(--hash) is a `defineConsts` at-rule placeholder and must be kept intact.
        if (keyPath.length === 0) {
          key = inner;
        }
      }

      if (valuePath.isObjectExpression()) {
        const result = evaluatePartialObjectRecursively(
          valuePath,
          traversalState,
          functions,
          [...keyPath, key],
        );
        if (!result.confident) {
          return { confident: false, deopt: result.deopt, value: null };
        }
        obj[key] = result.value;
        // $FlowFixMe[unsafe-object-assign]
        Object.assign(inlineStyles, result.inlineStyles);
      } else {
        const result = evaluate(valuePath, traversalState, functions);
        if (!result.confident) {
          const fullKeyPath = [...keyPath, key];
          const varName =
            '--x-' +
            (keyPath.length > 0
              ? utils.hash([...keyPath, key].join('_'))
              : key);
          obj[key] = `var(${varName})`;
          const node = valuePath.node;
          if (!t.isExpression(node)) {
            throw valuePath.buildCodeFrameError(
              'Expected expression as style value',
              SyntaxError,
            );
          }
          const expression: t.Expression = node as $FlowFixMe;

          const propName =
            fullKeyPath.find(
              (k) =>
                !k.startsWith(':') && !k.startsWith('@') && k !== 'default',
            ) ?? key;

          const unit =
            timeUnits.has(propName) || lengthUnits.has(propName)
              ? getNumberSuffix(propName)
              : '';

          const inlineStyleExpression =
            unit !== ''
              ? t.callExpression(
                  t.arrowFunctionExpression(
                    [t.identifier('val')],
                    t.conditionalExpression(
                      t.binaryExpression(
                        '===',
                        t.unaryExpression('typeof', t.identifier('val')),
                        t.stringLiteral('number'),
                      ),
                      t.binaryExpression(
                        '+',
                        t.identifier('val'),
                        t.stringLiteral(unit),
                      ),
                      t.conditionalExpression(
                        t.binaryExpression(
                          '!=',
                          t.identifier('val'),
                          t.nullLiteral(),
                        ),
                        t.identifier('val'),
                        t.identifier('undefined'),
                      ),
                    ),
                  ),
                  [expression as t.Expression],
                )
              : t.conditionalExpression(
                  t.binaryExpression('!=', expression, t.nullLiteral()),
                  expression,
                  t.identifier('undefined'),
                );
          inlineStyles[varName] = {
            path: [...keyPath, key],
            originalExpression: expression,
            expression: inlineStyleExpression,
          };
        } else {
          obj[key] = result.value;
        }
      }
    }
  }
  return { value: obj, confident: true, inlineStyles };
}

type KeyResult =
  | { confident: true, value: string }
  | { confident: false, deopt?: null | NodePath<> };

function evaluateObjKey(
  prop: NodePath<t.ObjectProperty>,
  traversalState: StateManager,
  functions: FunctionConfig,
): KeyResult {
  const keyPath: NodePath<t.ObjectProperty['key']> = prop.get('key');
  let key: string;
  if ((prop.node as t.ObjectProperty).computed) {
    const result = evaluate(keyPath, traversalState, functions);
    if (!result.confident) {
      return { confident: false, deopt: result.deopt };
    }
    key = result.value;
  } else if (keyPath.isIdentifier()) {
    key = keyPath.node.name;
  } else {
    // TODO: This isn't handling all possible types that `keyPath` could be
    key = (keyPath.node as $FlowFixMe).value;
  }
  return {
    confident: true,
    value: String(key),
  };
}

function validateDynamicStyleParams(
  path: NodePath<t.ArrowFunctionExpression>,
  params: Array<NodePath<t.Identifier | t.SpreadElement | t.Pattern>>,
) {
  if (params.some((param) => !param.isIdentifier())) {
    throw path.buildCodeFrameError(
      messages.ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS,
      SyntaxError,
    );
  }
}
