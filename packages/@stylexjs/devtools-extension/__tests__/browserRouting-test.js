/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

function installClients() {
  const directCollect = jest.fn(() => Promise.resolve('direct'));
  const relayedCollect = jest.fn(() => Promise.resolve('relayed'));
  jest.doMock('../src/devtools/inspectedClient', () => ({
    inspectedPageClient: {
      collect: directCollect,
      identify: jest.fn(),
      mutate: jest.fn(),
    },
  }));
  jest.doMock('../src/devtools/inspectedPageRelay', () => ({
    relayedInspectedPageClient: {
      collect: relayedCollect,
      identify: jest.fn(),
      mutate: jest.fn(),
    },
  }));
  return { directCollect, relayedCollect };
}

afterEach(() => {
  jest.resetModules();
  jest.dontMock('../src/devtools/inspectedClient');
  jest.dontMock('../src/devtools/inspectedPageRelay');
  delete global.browser;
  delete global.chrome;
});

test('keeps Chrome on the direct inspected-page client', async () => {
  global.chrome = {
    devtools: {
      inspectedWindow: {},
      panels: { elements: { createSidebarPane: jest.fn() } },
    },
    runtime: { getURL: jest.fn() },
  };
  const clients = installClients();
  const { devtoolsBridge } = require('../src/devtools/bridge');

  await expect(devtoolsBridge.collect()).resolves.toBe('direct');
  expect(clients.directCollect).toHaveBeenCalledTimes(1);
  expect(clients.relayedCollect).not.toHaveBeenCalled();
});

test('keeps Firefox on the direct inspected-page client', async () => {
  global.browser = {
    devtools: {
      inspectedWindow: {},
      panels: { elements: { createSidebarPane: jest.fn() } },
    },
    runtime: { getURL: jest.fn() },
  };
  const clients = installClients();
  const { devtoolsBridge } = require('../src/devtools/bridge');

  await expect(devtoolsBridge.collect()).resolves.toBe('direct');
  expect(clients.directCollect).toHaveBeenCalledTimes(1);
  expect(clients.relayedCollect).not.toHaveBeenCalled();
});

test('routes only Safari through the DevTools-page relay', async () => {
  global.browser = {
    devtools: { inspectedWindow: {}, panels: { create: jest.fn() } },
    runtime: { getURL: jest.fn() },
  };
  const clients = installClients();
  const { devtoolsBridge } = require('../src/devtools/bridge');

  await expect(devtoolsBridge.collect()).resolves.toBe('relayed');
  expect(clients.relayedCollect).toHaveBeenCalledTimes(1);
  expect(clients.directCollect).not.toHaveBeenCalled();
});
