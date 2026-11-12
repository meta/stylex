/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

function createPort(name) {
  const disconnectListeners = [];
  const messageListeners = [];
  return {
    name,
    peer: null,
    pair: null,
    onDisconnect: {
      addListener: jest.fn((listener) => disconnectListeners.push(listener)),
    },
    onMessage: {
      addListener: jest.fn((listener) => messageListeners.push(listener)),
    },
    disconnect() {
      if (this.pair.closed) return;
      this.pair.closed = true;
      for (const listener of disconnectListeners) listener();
      for (const listener of this.peer.disconnectListeners) listener();
    },
    disconnectListeners,
    postMessage(message) {
      if (this.pair.closed) {
        throw new Error('Port is disconnected.');
      }
      Promise.resolve().then(() => {
        if (this.pair.closed) return;
        for (const listener of this.peer.messageListeners) listener(message);
      });
    },
    messageListeners,
  };
}

function createRuntime() {
  const connectListeners = [];
  const connections = [];
  const runtime = {
    connect: jest.fn(({ name }) => {
      const client = createPort(name);
      const host = createPort(name);
      const pair = { closed: false };
      client.peer = host;
      client.pair = pair;
      host.peer = client;
      host.pair = pair;
      connections.push({ client, host });
      for (const listener of connectListeners) listener(host);
      return client;
    }),
    getURL: jest.fn(),
    onConnect: {
      addListener: jest.fn((listener) => connectListeners.push(listener)),
      removeListener: jest.fn((listener) => {
        const index = connectListeners.indexOf(listener);
        if (index >= 0) connectListeners.splice(index, 1);
      }),
    },
  };
  return { connections, runtime };
}

function installSafariApi(runtime) {
  global.browser = {
    devtools: { inspectedWindow: {}, panels: { create: jest.fn() } },
    runtime,
  };
}

function installInspectedClient(overrides = {}) {
  const client = {
    collect: jest.fn(() => Promise.resolve({ selectionState: 'element' })),
    identify: jest.fn(() => Promise.resolve('selection-1')),
    mutate: jest.fn(() => Promise.resolve({ ok: true })),
    ...overrides,
  };
  jest.doMock('../src/devtools/inspectedClient', () => ({
    inspectedPageClient: client,
  }));
  return client;
}

afterEach(() => {
  jest.useRealTimers();
  jest.resetModules();
  jest.dontMock('../src/devtools/inspectedClient');
  delete global.browser;
  delete global.chrome;
  window.history.replaceState({}, '', '/');
});

test('parses only instance-scoped Safari relay port names', () => {
  const {
    getSafariRelayPortName,
    parseSafariRelayPortName,
  } = require('../src/devtools/safariRelayProtocol');

  const name = getSafariRelayPortName('inspector-1');
  expect(parseSafariRelayPortName(name)).toBe('inspector-1');
  expect(parseSafariRelayPortName(`${name}/extra`)).toBeNull();
  expect(parseSafariRelayPortName('unrelated')).toBeNull();
});

test('connects the Safari panel directly to its DevTools page', async () => {
  const { connections, runtime } = createRuntime();
  installSafariApi(runtime);
  window.history.replaceState(
    {},
    '',
    '/panel.html?stylexRelay=inspector-direct',
  );
  const client = installInspectedClient();
  const {
    installInspectedPageRelay,
    relayedInspectedPageClient,
  } = require('../src/devtools/inspectedPageRelay');

  const controller = installInspectedPageRelay('inspector-direct');

  await expect(relayedInspectedPageClient.collect()).resolves.toEqual({
    selectionState: 'element',
  });
  await expect(relayedInspectedPageClient.identify()).resolves.toBe(
    'selection-1',
  );
  expect(client.collect).toHaveBeenCalledTimes(1);
  expect(client.identify).toHaveBeenCalledTimes(1);
  expect(connections).toHaveLength(1);

  controller.dispose();
});

test('forwards Safari override commands without changing their payload', async () => {
  const { runtime } = createRuntime();
  installSafariApi(runtime);
  window.history.replaceState(
    {},
    '',
    '/panel.html?stylexRelay=inspector-mutate',
  );
  const mutate = jest.fn(() =>
    Promise.resolve({ ok: false, code: 'mutation-failed', message: 'failed' }),
  );
  installInspectedClient({ mutate });
  const {
    installInspectedPageRelay,
    relayedInspectedPageClient,
  } = require('../src/devtools/inspectedPageRelay');
  const command = {
    kind: 'remove',
    overrideId: 'override-1',
    selectionId: 'selection-1',
  };

  const controller = installInspectedPageRelay('inspector-mutate');

  await expect(relayedInspectedPageClient.mutate(command)).resolves.toEqual({
    ok: false,
    code: 'mutation-failed',
    message: 'failed',
  });
  expect(mutate).toHaveBeenCalledWith(command);
  controller.dispose();
});

test('reconnects directly after Safari closes a hidden panel port', async () => {
  const { connections, runtime } = createRuntime();
  installSafariApi(runtime);
  window.history.replaceState(
    {},
    '',
    '/panel.html?stylexRelay=inspector-reconnect',
  );
  installInspectedClient();
  const {
    installInspectedPageRelay,
    relayedInspectedPageClient,
  } = require('../src/devtools/inspectedPageRelay');
  const controller = installInspectedPageRelay('inspector-reconnect');

  await relayedInspectedPageClient.collect();
  connections[0].client.disconnect();
  await relayedInspectedPageClient.collect();

  expect(runtime.connect).toHaveBeenCalledTimes(2);
  expect(connections).toHaveLength(2);
  controller.dispose();
});

test('drops a silent stale Safari port after a request timeout', async () => {
  jest.useFakeTimers();
  const { runtime } = createRuntime();
  installSafariApi(runtime);
  window.history.replaceState(
    {},
    '',
    '/panel.html?stylexRelay=inspector-timeout',
  );
  installInspectedClient();
  const {
    relayedInspectedPageClient,
  } = require('../src/devtools/inspectedPageRelay');

  const firstRequest = relayedInspectedPageClient
    .collect()
    .catch((error) => error);
  await jest.advanceTimersByTimeAsync(10000);
  await expect(firstRequest).resolves.toThrow(
    'The Safari DevTools page did not respond.',
  );

  relayedInspectedPageClient.collect().catch(() => {});
  expect(runtime.connect).toHaveBeenCalledTimes(2);
});
