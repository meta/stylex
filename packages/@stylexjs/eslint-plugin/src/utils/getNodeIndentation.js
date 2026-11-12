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
import type { SourceCode } from 'eslint/eslint-rule';
import type { Comment, Node } from 'estree';

function isSameLine(
  aNode: Node | Comment | Token,
  bNode: Node | Comment | Token,
): boolean {
  return Boolean(
    aNode.loc && bNode.loc && aNode.loc?.start.line === bNode.loc?.start.line,
  );
}

export default function getNodeIndentation(
  sourceCode: SourceCode,
  node: $ReadOnly<Node | Comment>,
): string {
  const tokenBefore = sourceCode.getTokenBefore(node, {
    includeComments: false,
  });

  const isTokenBeforeSameLineAsNode =
    !!tokenBefore && isSameLine(tokenBefore, node);

  const sliceStart =
    isTokenBeforeSameLineAsNode && tokenBefore?.loc
      ? tokenBefore.loc.end.column
      : 0;

  return node?.loc
    ? sourceCode.lines[node.loc.start.line - 1].slice(
        sliceStart,
        node.loc.start.column,
      )
    : '';
}
