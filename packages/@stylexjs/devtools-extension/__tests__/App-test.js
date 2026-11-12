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
import { App } from '../src/panel/App';
import { useDebugData } from '../src/panel/hooks/useDebugData';

jest.mock('@stylexjs/stylex', () => ({
  create: (styles) => styles,
  defineConsts: (values) => values,
  props: () => ({}),
}));
jest.mock('../src/panel/hooks/useDebugData', () => ({
  useDebugData: jest.fn(),
}));
jest.mock('../src/panel/components/Logo', () => () => null);
jest.mock('../src/panel/components/CopyMetadataButton', () => ({
  CopyMetadataButton: () => null,
}));
jest.mock('../src/panel/components/MatchedStyles', () => ({
  MatchedStyles: () => null,
}));
jest.mock('../src/panel/components/OverridesSection', () => ({
  OverridesSection: () => null,
}));
jest.mock('../src/panel/components/SourcesList', () => ({
  SourcesList: () => null,
}));

global.IS_REACT_ACT_ENVIRONMENT = true;

function renderSelectionState(selectionState) {
  useDebugData.mockReturnValue({
    data: {
      selectionId: '',
      selectionState,
      element: { tagName: '—' },
      sources: [],
      computed: {},
      suggestions: {},
      overrides: [],
      matched: { classes: [] },
      warnings: [],
    },
    error: null,
    loading: false,
    mutate: jest.fn(),
    refresh: jest.fn(),
    revision: 0,
  });
  const container = document.createElement('div');
  const root = createRoot(container);
  act(() => root.render(<App />));
  return { container, root };
}

test.each([
  ['none', 'No element selected.'],
  ['non-element', 'The selected node is not an element.'],
])('shows a graceful %s selection empty state', (selectionState, message) => {
  const { container, root } = renderSelectionState(selectionState);

  expect(container.textContent).toContain(message);
  act(() => root.unmount());
});
