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

jest.mock('../src/devtools/bridge', () => ({
  devtoolsBridge: {
    capabilities: { openSource: false, sourcePreview: false },
    getSourcePreview: jest.fn(),
    openSource: jest.fn(),
  },
}));
jest.mock('../src/utils/clipboard', () => ({
  copyText: jest.fn(() => Promise.resolve(true)),
}));
jest.mock('@stylexjs/stylex', () => ({
  create: (styles) => styles,
  defineConsts: (values) => values,
  props: () => ({}),
}));

const {
  devtoolsBridge: mockDevtoolsBridge,
} = require('../src/devtools/bridge');
const { copyText: mockCopyText } = require('../src/utils/clipboard');
const { SourceRow } = require('../src/panel/components/SourceRow');

global.IS_REACT_ACT_ENVIRONMENT = true;

const source = {
  raw: 'src/example.js:12',
  file: 'src/example.js',
  line: 12,
};
const mountedRoots = [];

function renderSourceRow(nextSource = source) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  mountedRoots.push(root);
  act(() => root.render(<SourceRow revision={1} source={nextSource} />));
  return { container, root };
}

beforeEach(() => {
  mockDevtoolsBridge.capabilities.openSource = false;
  mockDevtoolsBridge.capabilities.sourcePreview = false;
  mockDevtoolsBridge.getSourcePreview.mockReset();
  mockDevtoolsBridge.openSource.mockReset();
  mockCopyText.mockReset();
  mockCopyText.mockResolvedValue(true);
});

afterEach(() => {
  act(() => {
    while (mountedRoots.length > 0) mountedRoots.pop().unmount();
  });
  document.body.innerHTML = '';
});

test('copies the normalized location from the filename in Firefox', async () => {
  const prefixedSource = {
    raw: 'website:src/example.js:12',
    file: 'website:src/example.js',
    line: 12,
  };
  const { container } = renderSourceRow(prefixedSource);

  expect(container.textContent).toBe('website:src/example.js:12');
  expect(container.textContent).not.toContain('Copy location');
  expect(container.querySelector('[aria-label="Preview source"]')).toBeNull();

  await act(async () => {
    container
      .querySelector('[aria-label="Copy source location src/example.js:12"]')
      .click();
    await Promise.resolve();
  });

  expect(mockCopyText).toHaveBeenCalledWith('src/example.js:12');
  expect(container.querySelector('[role="status"]').textContent).toBe('Copied');
});

test('shows a visible failure when copying a Firefox source fails', async () => {
  mockCopyText.mockResolvedValue(false);
  const { container } = renderSourceRow();

  await act(async () => {
    container
      .querySelector('[aria-label="Copy source location src/example.js:12"]')
      .click();
    await Promise.resolve();
  });

  expect(container.querySelector('[role="status"]').textContent).toBe(
    'Copy failed',
  );
});

test('shows Chrome source controls when capabilities are available', () => {
  mockDevtoolsBridge.capabilities.openSource = true;
  mockDevtoolsBridge.capabilities.sourcePreview = true;
  const { container } = renderSourceRow();

  expect(
    container.querySelector('[aria-label="Preview source"]'),
  ).not.toBeNull();
  expect(container.textContent).not.toContain('Copy location');
});

test('keeps source preview failures local to the source row', async () => {
  mockDevtoolsBridge.capabilities.openSource = true;
  mockDevtoolsBridge.capabilities.sourcePreview = true;
  mockDevtoolsBridge.getSourcePreview.mockRejectedValue(
    new Error('Preview unavailable'),
  );
  const { container } = renderSourceRow();

  await act(async () => {
    container.querySelector('[aria-label="Preview source"]').click();
    await Promise.resolve();
  });

  expect(container.querySelector('[role="alert"]').textContent).toBe(
    'Preview unavailable',
  );
});
