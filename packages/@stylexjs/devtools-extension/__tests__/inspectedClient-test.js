/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

jest.mock('../src/devtools/browserApi', () => ({
  devtools: { inspectedWindow: { eval: jest.fn() } },
  getExtensionUrl: (path) => `moz-extension://stylex/${path}`,
  usesPromiseApi: true,
}));

const { devtools } = require('../src/devtools/browserApi');
const {
  collectDebugData,
  getSelectionIdentity,
} = require('../src/devtools/inspectedClient');

test('replaces an inspected runtime from an older extension build', async () => {
  const data = { selectionState: 'none' };
  global.fetch = jest.fn(() =>
    Promise.resolve({
      ok: true,
      text: () =>
        Promise.resolve(
          "globalThis[Symbol.for('@stylexjs/devtools/inspected-runtime')] = { version: 1, collect: () => ({ selectionState: 'none' }) };",
        ),
    }),
  );
  devtools.inspectedWindow.eval.mockImplementation((expression) => {
    if (expression.includes('?.revision ===')) {
      return Promise.resolve([false, null]);
    }
    if (expression.includes('.collect(')) {
      return Promise.resolve([data, null]);
    }
    return Promise.resolve([true, null]);
  });

  await expect(collectDebugData()).resolves.toBe(data);

  expect(global.fetch).toHaveBeenCalledWith(
    'moz-extension://stylex/assets/inspected-runtime.js',
    { cache: 'no-store' },
  );
  expect(devtools.inspectedWindow.eval.mock.calls[1][0]).toContain(
    '].revision = ',
  );
});

test('reads the selected element identity without collecting styles', async () => {
  devtools.inspectedWindow.eval.mockImplementation((expression) => {
    if (expression.includes('?.revision ===')) {
      return Promise.resolve([true, null]);
    }
    if (expression.includes('.identify(')) {
      return Promise.resolve(['selection-2', null]);
    }
    return Promise.resolve([true, null]);
  });

  await expect(getSelectionIdentity()).resolves.toBe('selection-2');
  expect(devtools.inspectedWindow.eval.mock.calls.at(-1)[0]).toContain(
    '].identify(',
  );
});
