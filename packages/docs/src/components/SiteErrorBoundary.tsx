/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use client';

import { Component, type ReactNode } from 'react';

const DEFAULT_TITLE = 'StyleX — The styling system for ambitious interfaces';
const DEFAULT_DESCRIPTION = 'The styling system that powers Meta.';
const RELOAD_KEY_PREFIX = 'stylex:chunk-reload:';
const RELOAD_WINDOW_MS = 60_000;
const DYNAMIC_IMPORT_ERROR =
  /chunkloaderror|failed to fetch dynamically imported module|error loading dynamically imported module|importing a module script failed|loading chunk .* failed/i;

type State = {
  hasError: boolean;
};

function getReloadKey(): string {
  return `${RELOAD_KEY_PREFIX}${window.location.pathname}`;
}

function reloadAfterDynamicImportError(error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  if (!DYNAMIC_IMPORT_ERROR.test(message)) {
    return;
  }

  try {
    const key = getReloadKey();
    const lastReload = Number(window.sessionStorage.getItem(key) ?? 0);
    if (Date.now() - lastReload < RELOAD_WINDOW_MS) {
      return;
    }
    window.sessionStorage.setItem(key, String(Date.now()));
  } catch {
    return;
  }

  window.location.reload();
}

export class SiteErrorBoundary extends Component<
  { children: ReactNode },
  State
> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: unknown): void {
    reloadAfterDynamicImportError(error);
  }

  render(): ReactNode {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <html lang="en">
        <head>
          <title>{DEFAULT_TITLE}</title>
          <meta charSet="utf-8" />
          <meta content="width=device-width, initial-scale=1" name="viewport" />
          <meta content={DEFAULT_DESCRIPTION} name="description" />
        </head>
        <body>
          <main
            data-nosnippet=""
            style={{
              alignItems: 'center',
              display: 'flex',
              flexDirection: 'column',
              fontFamily: 'system-ui, sans-serif',
              height: '100dvh',
              justifyContent: 'center',
              padding: 24,
              textAlign: 'center',
            }}
          >
            <h1>StyleX is temporarily unavailable</h1>
            <p>Refresh the page to try again.</p>
            <button onClick={() => window.location.reload()} type="button">
              Refresh
            </button>
          </main>
        </body>
      </html>
    );
  }
}
