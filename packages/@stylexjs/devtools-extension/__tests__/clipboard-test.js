/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { copyText } = require('../src/utils/clipboard');

const clipboardDescriptor = Object.getOwnPropertyDescriptor(
  navigator,
  'clipboard',
);

afterEach(() => {
  jest.restoreAllMocks();
  if (clipboardDescriptor == null) {
    delete navigator.clipboard;
  } else {
    Object.defineProperty(navigator, 'clipboard', clipboardDescriptor);
  }
  delete document.execCommand;
  document.body.replaceChildren();
});

test('focuses the clipboard fallback before selecting and copying', async () => {
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: undefined,
  });
  const focus = jest
    .spyOn(HTMLTextAreaElement.prototype, 'focus')
    .mockImplementation(() => {});
  const select = jest
    .spyOn(HTMLTextAreaElement.prototype, 'select')
    .mockImplementation(() => {});
  const execCommand = jest.fn(() => true);
  Object.defineProperty(document, 'execCommand', {
    configurable: true,
    value: execCommand,
  });

  await expect(copyText('source/file.js:10')).resolves.toBe(true);

  expect(focus).toHaveBeenCalledTimes(1);
  expect(select).toHaveBeenCalledTimes(1);
  expect(focus.mock.invocationCallOrder[0]).toBeLessThan(
    select.mock.invocationCallOrder[0],
  );
  expect(select.mock.invocationCallOrder[0]).toBeLessThan(
    execCommand.mock.invocationCallOrder[0],
  );
  expect(document.querySelector('textarea')).toBeNull();
});

test('removes the clipboard fallback when copying throws', async () => {
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: undefined,
  });
  Object.defineProperty(document, 'execCommand', {
    configurable: true,
    value: jest.fn(() => {
      throw new Error('Copy failed');
    }),
  });

  await expect(copyText('source/file.js:10')).resolves.toBe(false);

  expect(document.querySelector('textarea')).toBeNull();
});
