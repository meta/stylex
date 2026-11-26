/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

export const LOGICAL_BORDER_RADIUS_TO_PHYSICAL: $ReadOnly<{
  [key: string]: $ReadOnly<{ ltr: string, rtl: string }>,
}> = {
  'border-start-start-radius': {
    ltr: 'border-top-left-radius',
    rtl: 'border-top-right-radius',
  },
  'border-start-end-radius': {
    ltr: 'border-top-right-radius',
    rtl: 'border-top-left-radius',
  },
  'border-end-start-radius': {
    ltr: 'border-bottom-left-radius',
    rtl: 'border-bottom-right-radius',
  },
  'border-end-end-radius': {
    ltr: 'border-bottom-right-radius',
    rtl: 'border-bottom-left-radius',
  },
};
