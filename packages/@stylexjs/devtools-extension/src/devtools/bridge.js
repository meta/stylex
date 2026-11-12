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
  SourcePreview,
  StylexDebugData,
} from '../types';

import { devtools, requiresDevtoolsPageRelay } from './browserApi';
import { inspectedPageClient } from './inspectedClient';
import { relayedInspectedPageClient } from './inspectedPageRelay';
import {
  getSourcePreview,
  invalidateSourceCache,
  openSource,
  supportsOpenResource,
  supportsSourcePreview,
} from './sourceResources';

export type DevtoolsBridge = {
  capabilities: {
    openSource: boolean,
    sourcePreview: boolean,
  },
  collect: () => Promise<StylexDebugData>,
  mutate: (command: OverrideCommand) => Promise<OverrideMutationResult>,
  getSourcePreview: (
    file: string,
    line: number | null,
  ) => Promise<SourcePreview>,
  openSource: (file: string, line: number | null) => Promise<void>,
  subscribe: (callback: () => mixed) => () => void,
};

const SELECTION_POLL_INTERVAL_MS = 500;
const activeInspectedPageClient = requiresDevtoolsPageRelay
  ? relayedInspectedPageClient
  : inspectedPageClient;

export function subscribeToDevtoolsChanges(
  callback: () => mixed,
  readSelectionIdentity: () => Promise<string> = activeInspectedPageClient.identify,
  pollInterval: number = SELECTION_POLL_INTERVAL_MS,
): () => void {
  const selectionEvent = devtools.panels?.elements?.onSelectionChanged;
  const navigationEvent = devtools.network?.onNavigated;
  let disposed = false;
  let hasIdentity = false;
  let lastIdentity = '';
  let pollInProgress = false;
  let pollTimer: ?IntervalID = null;

  const handleNavigation = () => {
    hasIdentity = false;
    invalidateSourceCache();
    callback();
  };

  const pollSelection = async () => {
    if (disposed || pollInProgress) return;
    pollInProgress = true;
    try {
      const identity = await readSelectionIdentity();
      if (!hasIdentity) {
        hasIdentity = true;
        lastIdentity = identity;
      } else if (identity !== lastIdentity) {
        lastIdentity = identity;
        callback();
      }
    } catch {
      // The initial collection surfaces permission and inspected-page errors.
    } finally {
      pollInProgress = false;
    }
  };

  selectionEvent?.addListener(callback);
  navigationEvent?.addListener(handleNavigation);
  if (selectionEvent == null) {
    pollSelection();
    pollTimer = setInterval(pollSelection, pollInterval);
  }

  return () => {
    disposed = true;
    if (pollTimer != null) clearInterval(pollTimer);
    selectionEvent?.removeListener(callback);
    navigationEvent?.removeListener(handleNavigation);
  };
}

export const devtoolsBridge: DevtoolsBridge = {
  capabilities: {
    openSource: supportsOpenResource && supportsSourcePreview,
    sourcePreview: supportsSourcePreview,
  },
  collect: activeInspectedPageClient.collect,
  mutate: activeInspectedPageClient.mutate,
  getSourcePreview,
  openSource,
  subscribe: subscribeToDevtoolsChanges,
};
