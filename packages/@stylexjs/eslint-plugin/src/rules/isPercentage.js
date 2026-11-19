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
import type { Node } from 'estree';
import {
  tokenize,
  isTokenPercentage,
  isTokenEOF,
} from '@csstools/css-tokenizer';

import makeVariableCheckingRule from '../utils/makeVariableCheckingRule';

function isPercentage(node: Node, _variables?: Variables): RuleResponse {
  if (node.type === 'Literal') {
    const val = node.value;
    if (typeof val === 'string') {
      const [token, eof] = tokenize({ css: val });
      if (isTokenPercentage(token) && isTokenEOF(eof)) {
        return undefined;
      }
    }
  }
  return {
    message: 'A string literal representing a percentage (e.g. 100%)',
  };
}
export default makeVariableCheckingRule(isPercentage) as RuleCheck;
