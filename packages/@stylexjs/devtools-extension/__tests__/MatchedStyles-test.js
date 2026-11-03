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
import { MatchedStyles } from '../src/panel/components/MatchedStyles';

jest.mock('@stylexjs/stylex', () => ({
  create: (styles) => styles,
  defineConsts: (values) => values,
  props: () => ({}),
}));
jest.mock('../src/panel/components/ValueEditor', () => ({
  ValueEditor: ({ onCommit, value }) => (
    <button onClick={() => onCommit('purple')} type="button">
      {value}
    </button>
  ),
}));

global.IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => {
  document.body.innerHTML = '';
});

test('groups declarations by property and hides layer metadata', () => {
  const layer = { kind: 'at-rule', text: '@layer priority1', active: true };
  const data = {
    selectionId: 'selection-1',
    selectionState: 'element',
    element: { tagName: 'div' },
    sources: [],
    computed: { '': { 'border-color': 'canvastext' } },
    suggestions: { default: [], media: [] },
    overrides: [],
    matched: {
      classes: [
        {
          name: 'x1',
          declarations: [
            {
              key: 'default',
              contextKey: 'default',
              property: 'border-color',
              value: 'var(--x1vk235z)',
              important: false,
              conditions: [layer],
              className: 'x1',
            },
            {
              key: 'media',
              contextKey: 'media',
              property: 'border-color',
              value: 'canvastext',
              important: false,
              conditions: [
                layer,
                {
                  kind: 'at-rule',
                  text: '@media (forced-colors: active)',
                  active: true,
                },
              ],
              className: 'x1',
            },
          ],
        },
      ],
    },
    warnings: [],
  };
  const container = document.createElement('div');
  const root = createRoot(container);

  act(() => {
    root.render(<MatchedStyles data={data} onMutate={jest.fn()} />);
  });

  expect(container.textContent.match(/border-color/g)).toHaveLength(1);
  expect(container.textContent).toContain('default:var(--x1vk235z)');
  expect(container.textContent).toContain(
    '@media (forced-colors: active):canvastext',
  );
  expect(container.textContent).not.toContain('@layer');
  const firstRow = container.querySelector(
    '[data-declaration-part]',
  )?.parentElement;
  expect(
    Array.from(firstRow.querySelectorAll('[data-declaration-part]')).map(
      (part) => part.dataset.declarationPart,
    ),
  ).toEqual(['prefix', 'value', 'class']);
  act(() => root.unmount());
});

test('renders custom properties in class-owned groups', () => {
  const variable = (className, property, value) => ({
    key: `${className}-${property}`,
    contextKey: `${className}-${property}`,
    className,
    conditions: [],
    important: false,
    property,
    value,
  });
  const data = {
    selectionId: 'selection-1',
    selectionState: 'element',
    element: { tagName: 'html' },
    sources: [],
    computed: { '': {} },
    suggestions: {},
    overrides: [],
    matched: {
      classes: [
        {
          name: 'xTheme',
          declarations: [
            variable('xTheme', '--foreground', 'white'),
            variable('xTheme', '--background', 'black'),
          ],
        },
        {
          name: 'xAccent',
          declarations: [variable('xAccent', '--accent', 'blue')],
        },
      ],
    },
    warnings: [],
  };
  const container = document.createElement('div');
  const root = createRoot(container);

  act(() => {
    root.render(<MatchedStyles data={data} onMutate={jest.fn()} />);
  });

  const groups = container.querySelectorAll('[data-variable-class]');
  expect(Array.from(groups, (group) => group.dataset.variableClass)).toEqual([
    'xTheme',
    'xAccent',
  ]);
  expect(groups[0].textContent).toContain('--foreground:white');
  expect(groups[0].textContent).toContain('--background:black');
  expect(groups[0].textContent.match(/xTheme/g)).toHaveLength(1);
  act(() => root.unmount());
});

test('commits custom pseudo-element values as stylesheet rules', async () => {
  const data = {
    selectionId: 'selection-1',
    selectionState: 'element',
    element: { tagName: 'dialog' },
    sources: [],
    computed: { '::backdrop': { 'background-color': 'black' } },
    suggestions: { backdrop: [] },
    overrides: [],
    matched: {
      classes: [
        {
          name: 'xBackdrop',
          declarations: [
            {
              key: 'backdrop-rule',
              contextKey: 'backdrop',
              property: 'background-color',
              value: 'black',
              important: false,
              conditions: [],
              pseudoElement: '::backdrop',
              className: 'xBackdrop',
            },
          ],
        },
      ],
    },
    warnings: [],
  };
  const onMutate = jest.fn(() => Promise.resolve());
  const container = document.createElement('div');
  const root = createRoot(container);

  act(() => {
    root.render(<MatchedStyles data={data} onMutate={onMutate} />);
  });
  await act(async () => {
    container.querySelector('button').click();
    await Promise.resolve();
  });

  expect(onMutate).toHaveBeenCalledWith({
    type: 'set-rule',
    selectionId: 'selection-1',
    contextKey: 'backdrop',
    property: 'background-color',
    value: 'purple',
    important: false,
    conditions: [],
    sourceEntryKey: 'backdrop-rule',
    replaceOverrideIds: [],
    pseudoElement: '::backdrop',
  });
  act(() => root.unmount());
});
