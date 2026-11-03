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
import { useDebugData } from '../src/panel/hooks/useDebugData';
import { devtoolsBridge } from '../src/devtools/bridge';

jest.mock('../src/devtools/bridge', () => ({
  devtoolsBridge: {
    collect: jest.fn(),
    mutate: jest.fn(),
    subscribe: jest.fn(),
  },
}));

global.IS_REACT_ACT_ENVIRONMENT = true;

function makeData(selectionId) {
  return {
    selectionId,
    selectionState: 'element',
    element: { tagName: 'div' },
    sources: [],
    computed: {},
    suggestions: {},
    overrides: [],
    matched: { classes: [] },
    warnings: [],
  };
}

test('does not let an older mutation overwrite a newer selection', async () => {
  let resolveMutation;
  let subscription;
  let state;
  const oldData = makeData('selection-old');
  const newData = makeData('selection-new');
  devtoolsBridge.collect
    .mockResolvedValueOnce(oldData)
    .mockResolvedValueOnce(newData);
  devtoolsBridge.mutate.mockReturnValue(
    new Promise((resolve) => {
      resolveMutation = resolve;
    }),
  );
  devtoolsBridge.subscribe.mockImplementation((callback) => {
    subscription = callback;
    return () => {};
  });

  function Probe() {
    state = useDebugData();
    return null;
  }

  const container = document.createElement('div');
  const root = createRoot(container);
  await act(async () => {
    root.render(<Probe />);
  });
  expect(state.data.selectionId).toBe('selection-old');

  let mutation;
  act(() => {
    mutation = state.mutate({
      type: 'remove',
      selectionId: 'selection-old',
      overrideId: 'override',
    });
  });
  await act(async () => {
    await subscription();
  });
  expect(state.data.selectionId).toBe('selection-new');

  await act(async () => {
    resolveMutation({ ok: true, data: oldData });
    await mutation;
  });

  expect(state.data.selectionId).toBe('selection-new');
  act(() => root.unmount());
});
