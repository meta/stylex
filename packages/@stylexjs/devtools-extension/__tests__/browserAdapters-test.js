/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

afterEach(() => {
  jest.useRealTimers();
  jest.resetModules();
  delete global.browser;
  delete global.chrome;
});

test('uses the Chrome callback eval overload without unsupported options', async () => {
  const evaluate = jest.fn((expression, callback) => callback(42));
  global.chrome = {
    devtools: { inspectedWindow: { eval: evaluate }, panels: {} },
    runtime: { getURL: jest.fn() },
  };
  const { evalInInspectedPage } = require('../src/devtools/inspectedClient');

  await expect(evalInInspectedPage('6 * 7')).resolves.toBe(42);
  expect(evaluate).toHaveBeenCalledWith('6 * 7', expect.any(Function));
});

test('normalizes Firefox promise eval results without passing options', async () => {
  const evaluate = jest.fn(() => Promise.resolve([42, undefined]));
  global.browser = {
    devtools: { inspectedWindow: { eval: evaluate }, panels: {} },
    runtime: { getURL: jest.fn() },
  };
  const { evalInInspectedPage } = require('../src/devtools/inspectedClient');

  await expect(evalInInspectedPage('6 * 7')).resolves.toBe(42);
  expect(evaluate).toHaveBeenCalledWith('6 * 7');
});

test('creates a Firefox sidebar without calling unsupported setHeight', async () => {
  const pane = { setPage: jest.fn() };
  const createSidebarPane = jest.fn(() => Promise.resolve(pane));
  global.browser = {
    devtools: {
      inspectedWindow: {},
      panels: { elements: { createSidebarPane } },
    },
    runtime: { getURL: jest.fn() },
  };
  const {
    createStylexSidebarPane,
  } = require('../src/devtools/createSidebarPane');

  await createStylexSidebarPane();

  expect(createSidebarPane).toHaveBeenCalledWith('StyleX');
  expect(pane.setPage).toHaveBeenCalledWith('panel.html');
});

test('creates an instance-scoped Safari Web Inspector tab', async () => {
  const create = jest.fn(() => Promise.resolve({}));
  global.browser = {
    devtools: { inspectedWindow: {}, panels: { create } },
    runtime: { getURL: jest.fn() },
  };
  const {
    createStylexSidebarPane,
  } = require('../src/devtools/createSidebarPane');

  await createStylexSidebarPane('inspector 1');

  expect(create).toHaveBeenCalledWith(
    'StyleX',
    'assets/stylex-icon.svg',
    'panel.html?stylexRelay=inspector%201',
  );
});

test('uses direct inspected-page calls in Firefox', () => {
  global.browser = {
    devtools: {
      inspectedWindow: {},
      panels: { elements: { createSidebarPane: jest.fn() } },
    },
    runtime: { getURL: jest.fn() },
  };

  const browserApi = require('../src/devtools/browserApi');

  expect(browserApi.supportsElementsSidebar).toBe(true);
  expect(browserApi.requiresDevtoolsPageRelay).toBe(false);
});

test('uses the inspected-page relay only in Safari', () => {
  global.browser = {
    devtools: { inspectedWindow: {}, panels: { create: jest.fn() } },
    runtime: { getURL: jest.fn() },
  };

  const browserApi = require('../src/devtools/browserApi');

  expect(browserApi.supportsElementsSidebar).toBe(false);
  expect(browserApi.requiresDevtoolsPageRelay).toBe(true);
});

test('keeps Firefox source APIs disabled even if partial methods exist', () => {
  global.browser = {
    devtools: {
      inspectedWindow: { getResources: jest.fn() },
      panels: { openResource: jest.fn() },
    },
    runtime: { getURL: jest.fn() },
  };

  const resources = require('../src/devtools/sourceResources');

  expect(resources.supportsSourcePreview).toBe(false);
  expect(resources.supportsOpenResource).toBe(false);
});

test('uses Safari resource APIs when the runtime exposes them', async () => {
  const resource = {
    url: 'webpack://src/App.js',
    getContent: jest.fn(() =>
      Promise.resolve(['const first = 1;\nconst second = 2;', '']),
    ),
  };
  const getResources = jest.fn(() => Promise.resolve([resource]));
  const openResource = jest.fn(() => Promise.resolve());
  global.browser = {
    devtools: {
      inspectedWindow: { getResources },
      panels: { create: jest.fn(), openResource },
    },
    runtime: { getURL: jest.fn() },
  };

  const resources = require('../src/devtools/sourceResources');

  expect(resources.supportsSourcePreview).toBe(true);
  expect(resources.supportsOpenResource).toBe(true);
  await expect(resources.getSourcePreview('src/App.js', 2)).resolves.toEqual({
    url: resource.url,
    snippet: expect.stringContaining('> 2 | const second = 2;'),
  });
  await resources.openSource('src/App.js', 2);

  expect(getResources).toHaveBeenCalledTimes(1);
  expect(resource.getContent).toHaveBeenCalledTimes(1);
  expect(openResource).toHaveBeenCalledWith(resource.url, 1);
});

test('tolerates missing optional selection and navigation events', () => {
  global.browser = {
    devtools: { inspectedWindow: {}, panels: {} },
    runtime: { getURL: jest.fn() },
  };
  const { devtoolsBridge } = require('../src/devtools/bridge');

  expect(() => devtoolsBridge.subscribe(jest.fn())()).not.toThrow();
});

test('polls selection identity when Safari has no selection event', async () => {
  jest.useFakeTimers();
  global.browser = {
    devtools: { inspectedWindow: {}, panels: {} },
    runtime: { getURL: jest.fn() },
  };
  const { subscribeToDevtoolsChanges } = require('../src/devtools/bridge');
  const callback = jest.fn();
  const readIdentity = jest
    .fn()
    .mockResolvedValueOnce('selection-1')
    .mockResolvedValueOnce('selection-2');

  const unsubscribe = subscribeToDevtoolsChanges(callback, readIdentity, 100);
  await Promise.resolve();
  await jest.advanceTimersByTimeAsync(100);

  expect(callback).toHaveBeenCalledTimes(1);
  unsubscribe();
});

test('uses Chrome 148 callback resource APIs for previews and navigation', async () => {
  const resource = {
    url: 'webpack://src/App.js',
    getContent: jest.fn((callback) =>
      callback('const first = 1;\nconst second = 2;', ''),
    ),
  };
  const getResources = jest.fn((callback) => callback([resource]));
  const openResource = jest.fn();
  global.chrome = {
    devtools: {
      inspectedWindow: { getResources },
      panels: { openResource },
    },
    runtime: { getURL: jest.fn() },
  };
  const resources = require('../src/devtools/sourceResources');

  await expect(resources.getSourcePreview('src/App.js', 2)).resolves.toEqual({
    url: resource.url,
    snippet: expect.stringContaining('> 2 | const second = 2;'),
  });
  await resources.openSource('src/App.js', 2);

  expect(getResources).toHaveBeenCalledTimes(1);
  expect(resource.getContent).toHaveBeenCalledTimes(1);
  expect(openResource).toHaveBeenCalledWith(resource.url, 1);
});
