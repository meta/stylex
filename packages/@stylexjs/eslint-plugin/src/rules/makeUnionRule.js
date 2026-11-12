/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type {
  RuleResponse,
  Variables,
  RuleCheck,
} from '../stylex-valid-styles';
import type { Expression, Pattern, Property } from 'estree';
/*:: import { Rule } from 'eslint'; */

import makeLiteralRule from './makeLiteralRule';

export default function makeUnionRule(
  ...rules: $ReadOnlyArray<number | string | RuleCheck>
): RuleCheck {
  return (
    node: Expression | Pattern,
    variables?: Variables,
    prop?: Property,
    context?: Rule.RuleContext,
  ): RuleResponse => {
    const failedRules = [];
    for (const _rule of rules) {
      const rule =
        typeof _rule === 'string'
          ? makeLiteralRule(_rule)
          : typeof _rule === 'number'
            ? makeLiteralRule(_rule)
            : _rule;

      const check = rule(node, variables, prop, context);
      if (check === undefined) {
        // passes, that means we pass.
        return undefined;
      }
      failedRules.push(check);
    }
    const fixable = failedRules.filter(
      (a) => a.suggest != null || a.fix != null,
    );
    fixable.sort((a, b) => (a.distance || Infinity) - (b.distance || Infinity));

    return {
      message: failedRules.map((a) => a.message).join('\n'),
      fix: fixable[0] != null ? fixable[0].fix : undefined,
      suggest: fixable[0] != null ? fixable[0].suggest : undefined,
    };
  };
}
