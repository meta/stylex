/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import {
  createFromReadableStream,
  createTemporaryReferenceSet,
  encodeReply,
  setServerCallback,
} from '@vitejs/plugin-rsc/browser';
import { startTransition, StrictMode } from 'react';
import { hydrateRoot } from 'react-dom/client';
import {
  type unstable_RSCPayload as RSCServerPayload,
  type DataRouter,
} from 'react-router';
import {
  unstable_createCallServer as createCallServer,
  unstable_getRSCStream as getRSCStream,
  unstable_RSCHydratedRouter as RSCHydratedRouter,
} from 'react-router/dom';

// Create and set the callServer function to support post-hydration server actions.
setServerCallback(
  createCallServer({
    createFromReadableStream,
    createTemporaryReferenceSet,
    encodeReply,
  }),
);

// Get and decode the initial server payload
createFromReadableStream<RSCServerPayload>(getRSCStream()).then((payload) => {
  startTransition(async () => {
    const formState =
      payload.type === 'render' ? await payload.formState : undefined;

    hydrateRoot(
      document,
      <StrictMode>
        <RSCHydratedRouter
          createFromReadableStream={createFromReadableStream}
          payload={payload}
        />
      </StrictMode>,
      {
        formState,
      },
    );
  });
});

if (import.meta.hot) {
  // Minimal StyleX CSS HMR: listen for stylex:css-update and bust the CSS link
  // @ts-ignore
  import.meta.hot.on('rsc:update', () => {
    const reactDataRouter =
      (window as unknown as { __router: DataRouter }).__router ??
      (window as unknown as { __reactRouterDataRouter: DataRouter })
        .__reactRouterDataRouter;
    reactDataRouter?.revalidate();
  });
}
