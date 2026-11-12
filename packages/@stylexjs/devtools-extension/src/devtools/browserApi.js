/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

const root: any = globalThis;

export const usesPromiseApi: boolean =
  root.browser != null && root.browser.devtools != null;

export const extensionApi: any = usesPromiseApi ? root.browser : root.chrome;

if (extensionApi == null || extensionApi.devtools == null) {
  throw new Error('The WebExtension DevTools API is unavailable.');
}

export const devtools: any = extensionApi.devtools;

export const supportsElementsSidebar: boolean =
  typeof devtools.panels?.elements?.createSidebarPane === 'function';

// Safari exposes a full Web Inspector tab, but DevTools API calls made from
// that tab are not accepted as privileged messages. Route those calls through
// the DevTools page that created the tab.
export const requiresDevtoolsPageRelay: boolean =
  usesPromiseApi &&
  !supportsElementsSidebar &&
  typeof devtools.panels?.create === 'function';

export function getExtensionUrl(path: string): string {
  return extensionApi.runtime.getURL(path);
}
