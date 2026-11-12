/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { useCallback, useEffect, useRef, useState } from 'react';

import type { OverrideCommand, StylexDebugData } from '../../types';
import { devtoolsBridge } from '../../devtools/bridge';

export function useDebugData(): {
  data: ?StylexDebugData,
  error: ?string,
  loading: boolean,
  mutate: (command: OverrideCommand) => Promise<void>,
  refresh: () => Promise<void>,
  revision: number,
} {
  const [data, setData] = useState<?StylexDebugData>(null);
  const [error, setError] = useState<?string>(null);
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  const requestId = useRef(0);

  const refresh = useCallback(async () => {
    const id = requestId.current + 1;
    requestId.current = id;
    setError(null);
    try {
      const nextData = await devtoolsBridge.collect();
      if (requestId.current !== id) return;
      setData(nextData);
      setRevision((current) => current + 1);
    } catch (caught) {
      if (requestId.current !== id) return;
      setError(
        caught instanceof Error
          ? caught.message
          : 'Could not inspect the page.',
      );
    } finally {
      if (requestId.current === id) setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const unsubscribe = devtoolsBridge.subscribe(refresh);
    return () => {
      requestId.current += 1;
      unsubscribe();
    };
  }, [refresh]);

  const mutate = useCallback(async (command: OverrideCommand) => {
    const id = requestId.current + 1;
    requestId.current = id;
    setError(null);
    let result;
    try {
      result = await devtoolsBridge.mutate(command);
    } catch (caught) {
      if (requestId.current !== id) return;
      const message =
        caught instanceof Error ? caught.message : 'Could not update the page.';
      setError(message);
      throw new Error(message);
    }
    if (requestId.current !== id) return;
    if (!result.ok) {
      setError(result.message);
      throw new Error(result.message);
    }
    setData(result.data);
    setRevision((current) => current + 1);
  }, []);

  return { data, error, loading, mutate, refresh, revision };
}
