/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

const PORT_PREFIX = '@stylexjs/devtools/safari-relay/v4';

export function getSafariRelayPortName(relayId: string): string {
  return `${PORT_PREFIX}/${relayId}`;
}

export function parseSafariRelayPortName(name: string): ?string {
  const prefix = `${PORT_PREFIX}/`;
  if (!name.startsWith(prefix)) {
    return null;
  }
  const relayId = name.slice(prefix.length);
  return relayId === '' || relayId.includes('/') ? null : relayId;
}
