/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import type { ReactNode } from 'react';
import { SiteErrorBoundary } from '@/components/SiteErrorBoundary';

// Backport Waku's version-skew recovery while the site is on v1.0.0-alpha.0.
// https://github.com/wakujs/waku/pull/2240
const VERSION_SKEW_RECOVERY_SCRIPT = `
(() => {
  const key = 'stylex:chunk-reload:' + window.location.pathname;
  const retryWindowMs = 60_000;
  const dynamicImportError =
    /failed to fetch dynamically imported module|error loading dynamically imported module|importing a module script failed/i;

  const recover = (event) => {
    try {
      const lastReload = Number(window.sessionStorage.getItem(key) || 0);
      if (Date.now() - lastReload < retryWindowMs) {
        return;
      }
      window.sessionStorage.setItem(key, String(Date.now()));
    } catch {
      return;
    }

    event.preventDefault();
    const reload = () => window.location.reload();
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', reload, { once: true });
    } else {
      reload();
    }
  };

  window.addEventListener('vite:preloadError', recover);
  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason;
    const message =
      reason && typeof reason === 'object' && 'message' in reason
        ? String(reason.message)
        : String(reason);
    if (dynamicImportError.test(message)) {
      recover(event);
    }
  });
})();
`;

export default function RootElement({ children }: { children: ReactNode }) {
  return (
    <SiteErrorBoundary>
      <html lang="en">
        <head>
          <script>{VERSION_SKEW_RECOVERY_SCRIPT}</script>
        </head>
        <body>{children}</body>
      </html>
    </SiteErrorBoundary>
  );
}
