/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import {
  devtools,
  supportsElementsSidebar,
  usesPromiseApi,
} from './browserApi';

const SAFARI_PANEL_ICON = 'assets/stylex-icon.svg';

function configurePane(pane: any): Promise<void> {
  const operations = [Promise.resolve(pane.setPage('panel.html'))];
  if (typeof pane.setHeight === 'function') {
    operations.push(Promise.resolve(pane.setHeight(400)));
  }
  return Promise.all(operations).then(() => {});
}

export async function createStylexSidebarPane(
  relayId: ?string = null,
): Promise<void> {
  const elements = devtools.panels.elements;
  if (!supportsElementsSidebar) {
    if (typeof devtools.panels.create !== 'function') {
      throw new Error('This browser cannot create a DevTools view.');
    }
    const panelPage =
      relayId == null
        ? 'panel.html'
        : `panel.html?stylexRelay=${encodeURIComponent(relayId)}`;
    await devtools.panels.create('StyleX', SAFARI_PANEL_ICON, panelPage);
    return;
  }

  if (usesPromiseApi) {
    const pane = await elements.createSidebarPane('StyleX');
    await configurePane(pane);
    return;
  }

  await new Promise((resolve, reject) => {
    elements.createSidebarPane('StyleX', (pane) => {
      configurePane(pane).then(resolve, reject);
    });
  });
}
