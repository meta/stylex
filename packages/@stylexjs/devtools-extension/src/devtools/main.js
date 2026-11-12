/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import { createStylexSidebarPane } from './createSidebarPane.js';
import {
  createInspectedPageRelayId,
  installInspectedPageRelay,
} from './inspectedPageRelay.js';

const relayId = createInspectedPageRelayId();
installInspectedPageRelay(relayId);
createStylexSidebarPane(relayId).catch((error) => {
  console.error('Could not create the StyleX DevTools sidebar.', error);
});
