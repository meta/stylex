/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as React from 'react';
import { useMemo, useState } from 'react';
import * as stylex from '@stylexjs/stylex';

import type {
  AtomicSuggestion,
  OverrideCommand,
  StylexDebugData,
  StylexOverride,
} from '../../types';
import {
  formatCssValue,
  normalizeCssProperty,
  parseCssValue,
} from '../../utils/css';
import { colors } from '../theme.stylex';
import { ComboBox } from './ComboBox';

export function OverridesSection({
  data,
  onMutate,
}: {
  data: StylexDebugData,
  onMutate: (command: OverrideCommand) => Promise<void>,
}): React.Node {
  const [property, setProperty] = useState('');
  const [value, setValue] = useState('');
  const [pending, setPending] = useState(false);
  const suggestions = useMemo(
    () => getPropertySuggestions(data, property),
    [data, property],
  );

  const addOverride = async (nextValue: string) => {
    const normalizedProperty = normalizeCssProperty(property);
    const parsed = parseCssValue(nextValue);
    if (normalizedProperty === '' || parsed.value === '' || pending) return;
    setPending(true);
    try {
      await onMutate({
        type: 'set-inline',
        selectionId: data.selectionId,
        contextKey: `manual:${normalizedProperty}`,
        property: normalizedProperty,
        value: parsed.value,
        important: parsed.important,
        conditions: [],
        replaceOverrideIds: data.overrides
          .filter(
            (override) =>
              normalizeCssProperty(override.property) === normalizedProperty,
          )
          .map(({ id }) => id),
      });
      setValue('');
    } finally {
      setPending(false);
    }
  };

  return (
    <div {...stylex.props(styles.root)}>
      <div {...stylex.props(styles.title)}>Overrides</div>
      {data.overrides.length > 0 ? (
        <div {...stylex.props(styles.list)}>
          {data.overrides.map((override) => (
            <OverrideRow
              key={override.id}
              onRemove={() =>
                onMutate({
                  type: 'remove',
                  selectionId: data.selectionId,
                  overrideId: override.id,
                })
              }
              override={override}
            />
          ))}
        </div>
      ) : (
        <div {...stylex.props(styles.muted)}>No overrides yet.</div>
      )}
      <div {...stylex.props(styles.composer)}>
        <input
          {...stylex.props(styles.input)}
          onChange={(event) => setProperty(event.currentTarget.value)}
          placeholder="property"
          spellCheck={false}
          value={property}
        />
        <ComboBox
          commitOnBlur={false}
          disabled={pending}
          onChange={setValue}
          onCommit={addOverride}
          placeholder="value"
          suggestions={suggestions}
          value={value}
        />
        <button
          {...stylex.props(styles.button)}
          disabled={pending || property.trim() === '' || value.trim() === ''}
          onClick={() => {
            addOverride(value).catch(() => {});
          }}
          type="button"
        >
          Add
        </button>
      </div>
    </div>
  );
}

function getPropertySuggestions(
  data: StylexDebugData,
  property: string,
): Array<string> {
  const normalized = normalizeCssProperty(property);
  if (normalized === '') return [];
  const suggestions: Array<AtomicSuggestion> = Object.values(
    data.suggestions,
  ).flat();
  const values: Array<string> = [];
  for (const suggestion of suggestions) {
    if (normalizeCssProperty(suggestion.property) !== normalized) continue;
    const value = formatCssValue(suggestion.value, suggestion.important);
    if (!values.includes(value)) values.push(value);
  }
  return values;
}

function OverrideRow({
  onRemove,
  override,
}: {
  onRemove: () => Promise<void>,
  override: StylexOverride,
}): React.Node {
  const [pending, setPending] = useState(false);
  return (
    <div {...stylex.props(styles.row)}>
      <span {...stylex.props(styles.declaration)}>
        <span {...stylex.props(styles.property)}>{override.property}</span>
        {`: ${formatCssValue(override.value, override.important)}`}
      </span>
      <button
        {...stylex.props(styles.button)}
        disabled={pending}
        onClick={async () => {
          setPending(true);
          try {
            await onRemove();
          } catch {
            // The shared mutation hook surfaces the error in the panel.
          } finally {
            setPending(false);
          }
        }}
        type="button"
      >
        Remove
      </button>
    </div>
  );
}

const control = {
  backgroundColor: colors.bgRaised,
  borderColor: colors.border,
  borderRadius: 4,
  borderStyle: 'solid',
  borderWidth: 1,
  color: colors.textPrimary,
  fontFamily:
    'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
  fontSize: 12,
  lineHeight: '1.4',
};

const styles = stylex.create({
  root: {
    borderTopColor: colors.border,
    borderTopStyle: 'solid',
    borderTopWidth: 1,
    display: 'grid',
    gap: 7,
    paddingTop: 12,
  },
  title: { fontSize: 12, fontWeight: 600 },
  muted: { color: colors.textMuted },
  list: { display: 'grid', gap: 5 },
  row: {
    alignItems: 'center',
    display: 'flex',
    gap: 8,
    justifyContent: 'space-between',
    minWidth: 0,
  },
  declaration: {
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    minWidth: 0,
    overflowWrap: 'anywhere',
  },
  property: { color: colors.textAccent },
  composer: { display: 'flex', flexWrap: 'wrap', gap: 7 },
  input: {
    ...control,
    flex: 1,
    minWidth: 80,
    paddingBlock: 2,
    paddingInline: 5,
  },
  button: {
    ...control,
    cursor: { default: 'pointer', ':disabled': 'default' },
    flexShrink: 0,
    paddingBlock: 2,
    paddingInline: 7,
    ':disabled': { opacity: 0.55 },
  },
});
