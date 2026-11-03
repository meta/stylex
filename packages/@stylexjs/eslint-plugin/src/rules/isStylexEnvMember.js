/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { Expression, Pattern } from 'estree';

type ASTNode = {
  +type: string,
  +name?: string,
  +computed?: boolean,
  +object?: ASTNode,
  +property?: ASTNode,
  ...
};

/**
 * True when `node` is a compile time `stylex.env` member chain, e.g.
 * `stylex.env.responsive.belowSmall` or a named import `env.responsive.belowSmall`.
 * Those values are interpolated as literals by the compiler, so they are valid
 * object keys (typically media queries).
 */
export default function isStylexEnvMember(
  node: Expression | Pattern,
  styleXDefaultImports: Set<string>,
  styleXEnvImports: Set<string>,
): boolean {
  if (node == null || node.type !== 'MemberExpression') {
    return false;
  }

  const properties: Array<string> = [];
  let current: ASTNode = node;
  while (current.type === 'MemberExpression') {
    if (current.computed === true || current.property?.type !== 'Identifier') {
      return false;
    }
    if (typeof current.property.name !== 'string') {
      return false;
    }
    properties.unshift(current.property.name);
    if (current.object == null) {
      return false;
    }
    current = current.object;
  }

  if (current.type !== 'Identifier' || typeof current.name !== 'string') {
    return false;
  }

  if (styleXEnvImports.has(current.name)) {
    return properties.length > 0;
  }

  return (
    styleXDefaultImports.has(current.name) &&
    properties[0] === 'env' &&
    properties.length >= 1
  );
}
