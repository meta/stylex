/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type {
  OverrideCommand,
  OverrideMutationResult,
  StylexDebugData,
} from '../types';

import { extensionApi, requiresDevtoolsPageRelay } from './browserApi';
import { inspectedPageClient } from './inspectedClient';
import {
  getSafariRelayPortName,
  parseSafariRelayPortName,
} from './safariRelayProtocol';

const RELAY_QUERY_PARAM = 'stylexRelay';
const RELAY_TIMEOUT_MS = 10000;

type RelayMethod = 'collect' | 'identify' | 'mutate';

type RelayRequest = {
  kind: 'request',
  method: RelayMethod,
  payload?: OverrideCommand,
  requestId: number,
};

type RelayResponse = {
  error?: string,
  kind: 'response',
  ok: boolean,
  requestId: number,
  value?: mixed,
};

type PendingRequest = {
  reject: (error: Error) => void,
  resolve: (value: any) => void,
  timeout: TimeoutID,
};

export type InspectedPageRelayController = {
  dispose: () => void,
};

let panelPort: any = null;
let nextRequestId = 1;
const pendingRequests: Map<number, PendingRequest> = new Map();

function isRelayRequest(value: any): boolean {
  return (
    value != null &&
    typeof value === 'object' &&
    value.kind === 'request' &&
    typeof value.requestId === 'number' &&
    (value.method === 'collect' ||
      value.method === 'identify' ||
      value.method === 'mutate')
  );
}

function isRelayResponse(value: any): boolean {
  return (
    value != null &&
    typeof value === 'object' &&
    value.kind === 'response' &&
    typeof value.ok === 'boolean' &&
    typeof value.requestId === 'number'
  );
}

function formatError(error: mixed): string {
  return error instanceof Error
    ? error.message
    : 'Could not communicate with the inspected page.';
}

async function executeRequest(request: RelayRequest): Promise<mixed> {
  switch (request.method) {
    case 'collect':
      return inspectedPageClient.collect();
    case 'identify':
      return inspectedPageClient.identify();
    case 'mutate':
      if (request.payload == null) {
        throw new Error('The override command is missing.');
      }
      return inspectedPageClient.mutate(request.payload);
    default:
      throw new Error('Unknown inspected-page request.');
  }
}

function rejectPendingRequests(message: string): void {
  for (const pending of pendingRequests.values()) {
    clearTimeout(pending.timeout);
    pending.reject(new Error(message));
  }
  pendingRequests.clear();
}

function closePanelPort(port: any, message: string): void {
  if (panelPort !== port) {
    return;
  }
  panelPort = null;
  try {
    port.disconnect();
  } catch {
    // Safari can retain a disconnected port object while a panel is hidden.
  }
  rejectPendingRequests(message);
}

export function createInspectedPageRelayId(): ?string {
  if (!requiresDevtoolsPageRelay) {
    return null;
  }
  return globalThis.crypto.randomUUID();
}

export function installInspectedPageRelay(
  relayId: ?string,
): InspectedPageRelayController {
  if (!requiresDevtoolsPageRelay) {
    return { dispose: () => {} };
  }
  if (relayId == null) {
    throw new Error('The Safari inspected-page relay ID is missing.');
  }

  const activePorts: Set<any> = new Set();
  const handleConnect = (port: any) => {
    if (parseSafariRelayPortName(port.name) !== relayId) {
      return;
    }
    activePorts.add(port);
    port.onMessage.addListener((message: any) => {
      if (!isRelayRequest(message)) {
        return;
      }
      const request: RelayRequest = message;
      executeRequest(request).then(
        (value) => {
          if (activePorts.has(port)) {
            port.postMessage({
              kind: 'response',
              ok: true,
              requestId: request.requestId,
              value,
            });
          }
        },
        (error) => {
          if (activePorts.has(port)) {
            port.postMessage({
              error: formatError(error),
              kind: 'response',
              ok: false,
              requestId: request.requestId,
            });
          }
        },
      );
    });
    port.onDisconnect.addListener(() => {
      activePorts.delete(port);
    });
  };

  extensionApi.runtime.onConnect.addListener(handleConnect);
  return {
    dispose: () => {
      extensionApi.runtime.onConnect.removeListener(handleConnect);
      for (const port of activePorts) {
        port.disconnect();
      }
      activePorts.clear();
    },
  };
}

function getPanelPort(): any {
  if (panelPort != null) {
    return panelPort;
  }
  const relayId = new URLSearchParams(globalThis.location.search).get(
    RELAY_QUERY_PARAM,
  );
  if (relayId == null || relayId === '') {
    throw new Error('The Safari DevTools page relay is unavailable.');
  }

  const port = extensionApi.runtime.connect({
    name: getSafariRelayPortName(relayId),
  });
  port.onMessage.addListener((message: any) => {
    if (!isRelayResponse(message)) {
      return;
    }
    const response: RelayResponse = message;
    const pending = pendingRequests.get(response.requestId);
    if (pending == null) {
      return;
    }
    clearTimeout(pending.timeout);
    pendingRequests.delete(response.requestId);
    if (response.ok) {
      pending.resolve(response.value);
    } else {
      pending.reject(
        new Error(
          response.error ?? 'Could not communicate with the inspected page.',
        ),
      );
    }
  });
  port.onDisconnect.addListener(() => {
    if (panelPort === port) {
      closePanelPort(port, 'The Safari DevTools page disconnected.');
    }
  });
  panelPort = port;
  return port;
}

function request(method: RelayMethod, payload?: OverrideCommand): Promise<any> {
  const port = getPanelPort();
  const requestId = nextRequestId;
  nextRequestId += 1;

  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      if (!pendingRequests.has(requestId)) {
        return;
      }
      if (panelPort === port) {
        closePanelPort(port, 'The Safari DevTools page did not respond.');
      } else {
        pendingRequests.delete(requestId);
        reject(new Error('The Safari DevTools page did not respond.'));
      }
    }, RELAY_TIMEOUT_MS);
    pendingRequests.set(requestId, { reject, resolve, timeout });
    try {
      port.postMessage({ kind: 'request', method, payload, requestId });
    } catch (error) {
      clearTimeout(timeout);
      pendingRequests.delete(requestId);
      if (panelPort === port) {
        panelPort = null;
      }
      reject(new Error(formatError(error)));
    }
  });
}

export const relayedInspectedPageClient = {
  collect(): Promise<StylexDebugData> {
    return request('collect');
  },
  identify(): Promise<string> {
    return request('identify');
  },
  mutate(command: OverrideCommand): Promise<OverrideMutationResult> {
    return request('mutate', command);
  },
};
