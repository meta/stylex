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
  isTokenDimension,
  isTokenEOF,
} from '@csstools/css-tokenizer';

const absoluteLengthUnits = new Set(['px', 'mm', 'in', 'pc', 'pt']);
const isAbsoluteLength: RuleCheck = (
  node: Node,
  _variables?: Variables,
): RuleResponse => {
  if (node.type === 'Literal') {
    const val = node.value;
    if (typeof val === 'string') {
      const [token, eof] = tokenize({ css: val });
      if (
        isTokenDimension(token) &&
        isTokenEOF(eof) &&
        absoluteLengthUnits.has(token[4].unit) &&
        !token[1].includes('\\')
      ) {
        return undefined;
      }
    }
  }
  return {
    message: `a number ending in ${Array.from(absoluteLengthUnits).join(', ')}`,
  };
};

export default isAbsoluteLength;
