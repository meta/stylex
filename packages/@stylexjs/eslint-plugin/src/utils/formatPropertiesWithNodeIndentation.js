/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { SourceCode } from 'eslint/eslint-rule';
import type { Node } from 'estree';
import getNodeIndentation from './getNodeIndentation';

export default function formatPropertiesWithNodeIndentation(
  node: $ReadOnly<Node>,
  properties: $ReadOnlyArray<string>,
  sourceCode?: SourceCode,
): string {
  const indentation =
    sourceCode != null
      ? getNodeIndentation(sourceCode, node)
      : node.loc != null
        ? ' '.repeat(node.loc.start.column)
        : '';

  const newLineAndIndent = `\n${indentation}`;

  return properties
    .map((property, index) => `${index > 0 ? newLineAndIndent : ''}${property}`)
    .join(',');
}
