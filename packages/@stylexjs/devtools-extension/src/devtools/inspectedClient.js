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

import { INSPECTED_RUNTIME_KEY } from '../inspected/runtimeKey';
import { devtools, getExtensionUrl, usesPromiseApi } from './browserApi';

type ExceptionInfo = {
  code?: string,
  isError?: boolean,
  isException?: boolean,
  value?: mixed,
  description?: string,
};

type RuntimeBundle = {
  revision: string,
  source: string,
};

let runtimeBundlePromise: ?Promise<RuntimeBundle> = null;

function formatEvalError(exceptionInfo: ?ExceptionInfo): Error {
  const detail =
    exceptionInfo?.description ?? exceptionInfo?.value ?? 'Unknown error';
  return new Error(`Error evaluating in the inspected page: ${String(detail)}`);
}

export function evalInInspectedPage<T>(expression: string): Promise<T> {
  if (usesPromiseApi) {
    return Promise.resolve(devtools.inspectedWindow.eval(expression)).then(
      (response) => {
        const [result, exceptionInfo] = Array.isArray(response)
          ? response
          : [response, null];
        if (exceptionInfo?.isException || exceptionInfo?.isError) {
          throw formatEvalError(exceptionInfo);
        }
        return result as any;
      },
    );
  }

  return new Promise((resolve, reject) => {
    devtools.inspectedWindow.eval(expression, (result, exceptionInfo) => {
      if (exceptionInfo?.isException || exceptionInfo?.isError) {
        reject(formatEvalError(exceptionInfo));
        return;
      }
      resolve(result as any);
    });
  });
}

function fingerprintSource(source: string): string {
  let hash = 2166136261;
  for (let index = 0; index < source.length; index += 1) {
    hash = Math.imul(hash ^ source.charCodeAt(index), 16777619);
  }
  return `${source.length.toString(36)}-${(hash >>> 0).toString(36)}`;
}

function getRuntimeBundle(): Promise<RuntimeBundle> {
  if (runtimeBundlePromise == null) {
    runtimeBundlePromise = fetch(
      getExtensionUrl('assets/inspected-runtime.js'),
      { cache: 'no-store' },
    )
      .then((response) => {
        if (!response.ok) {
          throw new Error(
            `Could not load the inspected runtime (${response.status}).`,
          );
        }
        return response.text();
      })
      .then((source) => ({ revision: fingerprintSource(source), source }))
      .catch((error) => {
        runtimeBundlePromise = null;
        throw error;
      });
  }
  return runtimeBundlePromise;
}

async function ensureRuntime(): Promise<void> {
  const { revision, source } = await getRuntimeBundle();
  const key = JSON.stringify(INSPECTED_RUNTIME_KEY);
  const encodedRevision = JSON.stringify(revision);
  const installed = await evalInInspectedPage<boolean>(
    `globalThis[Symbol.for(${key})]?.version === 1 && globalThis[Symbol.for(${key})]?.revision === ${encodedRevision}`,
  );
  if (installed) {
    return;
  }
  await evalInInspectedPage<boolean>(
    `${source}\n;globalThis[Symbol.for(${key})].revision = ${encodedRevision};true`,
  );
}

export async function collectDebugData(): Promise<StylexDebugData> {
  await ensureRuntime();
  const key = JSON.stringify(INSPECTED_RUNTIME_KEY);
  return evalInInspectedPage(
    `globalThis[Symbol.for(${key})].collect(typeof $0 === 'undefined' ? null : $0)`,
  );
}

export async function getSelectionIdentity(): Promise<string> {
  await ensureRuntime();
  const key = JSON.stringify(INSPECTED_RUNTIME_KEY);
  return evalInInspectedPage(
    `globalThis[Symbol.for(${key})].identify(typeof $0 === 'undefined' ? null : $0)`,
  );
}

export async function mutateOverride(
  command: OverrideCommand,
): Promise<OverrideMutationResult> {
  await ensureRuntime();
  const key = JSON.stringify(INSPECTED_RUNTIME_KEY);
  return evalInInspectedPage(
    `globalThis[Symbol.for(${key})].mutate(${JSON.stringify(
      command,
    )}, typeof $0 === 'undefined' ? null : $0)`,
  );
}

export const inspectedPageClient = {
  collect: collectDebugData,
  identify: getSelectionIdentity,
  mutate: mutateOverride,
};
