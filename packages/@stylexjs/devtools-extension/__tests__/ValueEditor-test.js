/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

import * as React from 'react';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { ValueEditor } from '../src/panel/components/ValueEditor';

jest.mock('@stylexjs/stylex', () => ({
  create: (styles) => styles,
  defineConsts: (values) => values,
  props: () => ({}),
}));

global.IS_REACT_ACT_ENVIRONMENT = true;

function renderEditor(props = {}) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  const onCommit = jest.fn(() => Promise.resolve());
  act(() => {
    root.render(
      <ValueEditor
        onCommit={onCommit}
        suggestions={['red', 'blue']}
        value="red"
        {...props}
      />,
    );
  });
  return { container, onCommit, root };
}

afterEach(() => {
  jest.restoreAllMocks();
  document.body.innerHTML = '';
});

test('prefills the editor with the displayed value', () => {
  const { container } = renderEditor();
  act(() => container.querySelector('button').click());

  expect(container.querySelector('input').value).toBe('red');
});

test('pretty prints a long function without changing the editable value', () => {
  const value = `clamp(${Array(3)
    .fill('calc(var(--width) / var(--scale) * 100vw)')
    .join(', ')})`;
  const { container } = renderEditor({ value });
  const button = container.querySelector('button');

  expect(button.textContent).toContain('\n');
  act(() => button.click());
  expect(container.querySelector('input').value).toBe(value);
});

test('keeps a long nested function inline when its value cell is wide', () => {
  jest.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
    bottom: 0,
    height: 20,
    left: 0,
    right: 1200,
    top: 0,
    width: 1200,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  });
  const value =
    'light-dark(var(--xa5j11c), color-mix(in oklab, var(--xa5j11c) 88%, var(--x1q2bivm)))';
  const { container } = renderEditor({ value });

  expect(container.querySelector('button').textContent).toBe(value);
});

test('shows a variable resolved on the inspected element when hovered', () => {
  const { container } = renderEditor({
    resolvedVariables: { '--color': 'rgb(10, 20, 30)' },
    value: 'var(--color)',
  });
  const reference = container.querySelector('[data-css-variable="--color"]');

  expect(reference).not.toBeNull();
  act(() => {
    reference.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
  });
  expect(container.querySelector('[aria-hidden="true"]').textContent).toBe(
    'rgb(10, 20, 30)',
  );
  expect(container.querySelector('input')).toBeNull();
});

test('cancels an empty value on blur without committing', () => {
  const { container, onCommit } = renderEditor();
  act(() => container.querySelector('button').click());
  const input = container.querySelector('input');
  act(() => {
    input.value = '';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
  });

  expect(onCommit).not.toHaveBeenCalled();
  expect(container.querySelector('input')).toBeNull();
  expect(container.textContent).toContain('red');
});

test('cancels on Escape', () => {
  const { container, onCommit } = renderEditor();
  act(() => container.querySelector('button').click());
  act(() => {
    container
      .querySelector('input')
      .dispatchEvent(
        new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' }),
      );
  });
  expect(onCommit).not.toHaveBeenCalled();
  expect(container.querySelector('input')).toBeNull();
});
