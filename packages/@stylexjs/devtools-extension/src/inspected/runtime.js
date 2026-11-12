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

import { collectStylexDebugData } from './collector';
import { applyOverrideMutation } from './overrideStore';
import { identifySelection } from './runtimeState';

export type InspectedPageRuntime = {
  version: 1,
  collect: (target: mixed) => StylexDebugData,
  identify: (target: mixed) => string,
  mutate: (command: OverrideCommand, target: mixed) => OverrideMutationResult,
};

export function createInspectedPageRuntime(
  collector: (target: mixed) => StylexDebugData = collectStylexDebugData,
): InspectedPageRuntime {
  return {
    version: 1,
    collect: collector,
    identify: identifySelection,
    mutate(command, target) {
      const result = applyOverrideMutation(command, target);
      if (!result.ok) {
        return result;
      }
      try {
        const data = collector(target);
        result.commit();
        return { ok: true, data };
      } catch (error) {
        result.rollback();
        return {
          ok: false,
          code: 'mutation-failed',
          message:
            error instanceof Error
              ? error.message
              : 'Could not refresh the inspected data.',
        };
      }
    },
  };
}
