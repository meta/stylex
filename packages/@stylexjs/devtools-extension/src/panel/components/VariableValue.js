/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as React from 'react';
import { useState } from 'react';
import * as stylex from '@stylexjs/stylex';

import { findCssVariableReferences } from '../../utils/cssVariables';
import { formatCssValueForDisplay } from '../declarations/formatValue';
import { colors } from '../theme.stylex';

type ResolvedVariables = $ReadOnly<{ [string]: string, ... }>;

export function VariableValue({
  resolvedVariables,
  value,
}: {
  resolvedVariables: ResolvedVariables,
  value: string,
}): React.Node {
  const references = findCssVariableReferences(value);
  if (references.length === 0) return value;

  const parts: Array<React.Node> = [];
  let cursor = 0;
  for (const { end, name, start } of references) {
    if (start < cursor) continue;
    if (start > cursor) parts.push(value.slice(cursor, start));
    parts.push(
      <VariableReference
        key={`${start}:${name}`}
        name={name}
        resolvedValue={resolvedVariables[name]}
      />,
    );
    cursor = end;
  }
  if (cursor < value.length) parts.push(value.slice(cursor));
  return parts;
}

function VariableReference({
  name,
  resolvedValue,
}: {
  name: string,
  resolvedValue?: string,
}): React.Node {
  const [hovered, setHovered] = useState(false);
  const trimmedValue = resolvedValue?.trim() ?? '';
  const tooltip =
    trimmedValue === ''
      ? 'Not defined on this element.'
      : formatCssValueForDisplay(trimmedValue);

  return (
    <span
      {...stylex.props(styles.reference)}
      data-css-variable={name}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {name}
      {hovered ? (
        <span aria-hidden="true" {...stylex.props(styles.tooltip)}>
          {tooltip}
        </span>
      ) : null}
    </span>
  );
}

const styles = stylex.create({
  reference: {
    position: 'relative',
    textDecorationColor: colors.textMuted,
    textDecorationLine: 'underline',
    textDecorationStyle: 'dotted',
    textUnderlineOffset: 2,
  },
  tooltip: {
    backgroundColor: colors.bgRaised,
    borderColor: colors.border,
    borderRadius: 4,
    borderStyle: 'solid',
    borderWidth: 1,
    boxShadow: '0 4px 12px rgb(0 0 0 / 0.3)',
    color: colors.textPrimary,
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    fontSize: 12,
    insetBlockStart: '100%',
    insetInlineStart: 0,
    lineHeight: 1.4,
    maxWidth: 'min(420px, 75vw)',
    overflowWrap: 'anywhere',
    paddingBlock: 6,
    paddingInline: 8,
    position: 'absolute',
    whiteSpace: 'pre-wrap',
    width: 'max-content',
    zIndex: 10,
  },
});
