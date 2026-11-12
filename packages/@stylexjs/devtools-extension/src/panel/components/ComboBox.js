/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as React from 'react';
import { useEffect, useId, useMemo, useRef, useState } from 'react';
import * as stylex from '@stylexjs/stylex';

import { colors } from '../theme.stylex';

export function ComboBox({
  autoFocus = false,
  commitOnBlur = true,
  disabled = false,
  onCancel,
  onChange,
  onCommit,
  placeholder,
  suggestions,
  value,
}: {
  autoFocus?: boolean,
  commitOnBlur?: boolean,
  disabled?: boolean,
  onCancel?: () => void,
  onChange: (value: string) => void,
  onCommit: (value: string) => mixed,
  placeholder?: string,
  suggestions: ReadonlyArray<string>,
  value: string,
}): React.Node {
  const listId = useId();
  const committing = useRef(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [focused, setFocused] = useState(false);
  const filtered = useMemo(() => {
    const query = value.trim().toLowerCase();
    return suggestions
      .filter((suggestion) =>
        query === '' ? true : suggestion.toLowerCase().includes(query),
      )
      .slice(0, 8);
  }, [suggestions, value]);

  useEffect(() => {
    setActiveIndex((current) =>
      current >= filtered.length ? filtered.length - 1 : current,
    );
  }, [filtered.length]);

  const commit = (nextValue: string) => {
    if (committing.current) return;
    if (nextValue.trim() !== '') {
      committing.current = true;
      try {
        Promise.resolve(onCommit(nextValue)).then(
          () => {
            committing.current = false;
          },
          () => {
            committing.current = false;
          },
        );
      } catch (error) {
        committing.current = false;
        throw error;
      }
    } else {
      onCancel?.();
    }
  };

  return (
    <div {...stylex.props(styles.root)}>
      <input
        {...stylex.props(styles.input)}
        aria-activedescendant={
          activeIndex >= 0 ? `${listId}-${activeIndex}` : undefined
        }
        aria-autocomplete="list"
        aria-controls={focused && filtered.length > 0 ? listId : undefined}
        aria-expanded={focused && filtered.length > 0}
        autoFocus={autoFocus}
        disabled={disabled}
        onBlur={() => {
          setFocused(false);
          if (commitOnBlur && !disabled) commit(value);
        }}
        onChange={(event) => onChange(event.currentTarget.value)}
        onFocus={() => setFocused(true)}
        onKeyDown={(event) => {
          if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            if (filtered.length === 0) return;
            event.preventDefault();
            const delta = event.key === 'ArrowDown' ? 1 : -1;
            setActiveIndex((current) => {
              if (current === -1) return delta > 0 ? 0 : filtered.length - 1;
              return (current + delta + filtered.length) % filtered.length;
            });
          } else if (event.key === 'Enter') {
            event.preventDefault();
            commit(filtered[activeIndex] ?? value);
          } else if (event.key === 'Escape') {
            event.preventDefault();
            onCancel?.();
          }
        }}
        placeholder={placeholder}
        role="combobox"
        spellCheck={false}
        value={value}
      />
      {focused && filtered.length > 0 ? (
        <div {...stylex.props(styles.list)} id={listId} role="listbox">
          {filtered.map((suggestion, index) => (
            <div
              {...stylex.props(
                styles.option,
                index === activeIndex && styles.optionActive,
              )}
              aria-selected={index === activeIndex}
              id={`${listId}-${index}`}
              key={suggestion}
              onMouseDown={(event) => {
                event.preventDefault();
                commit(suggestion);
              }}
              onMouseEnter={() => setActiveIndex(index)}
              role="option"
            >
              {suggestion}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

const styles = stylex.create({
  root: {
    display: 'flex',
    flex: 1,
    minWidth: 0,
    position: 'relative',
  },
  input: {
    backgroundColor: colors.bgRaised,
    borderColor: colors.border,
    borderRadius: 4,
    borderStyle: 'solid',
    borderWidth: 1,
    color: colors.textPrimary,
    flex: 1,
    fontFamily: 'inherit',
    fontSize: 'inherit',
    lineHeight: 'inherit',
    minWidth: 0,
    paddingBlock: 2,
    paddingInline: 5,
  },
  list: {
    backgroundColor: colors.bgRaised,
    borderColor: colors.border,
    borderRadius: 6,
    borderStyle: 'solid',
    borderWidth: 1,
    boxShadow: '0 6px 16px rgba(0, 0, 0, 0.18)',
    left: 0,
    maxHeight: 160,
    minWidth: '100%',
    overflowY: 'auto',
    paddingBlock: 4,
    position: 'absolute',
    top: '100%',
    zIndex: 3,
  },
  option: {
    color: { default: colors.textPrimary, ':hover': colors.textAccent },
    cursor: 'pointer',
    paddingBlock: 4,
    paddingInline: 8,
  },
  optionActive: {
    backgroundColor: colors.bg,
    color: colors.textAccent,
  },
});
