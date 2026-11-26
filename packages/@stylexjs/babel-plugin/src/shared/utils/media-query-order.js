/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { lastMediaQueryWinsTransform } from 'style-value-parser';
import * as messages from '../messages';

export function applyMediaQueryOrder<T: { +[string]: mixed }>(styles: T): T {
  try {
    return lastMediaQueryWinsTransform(styles);
  } catch (error) {
    throw new Error(messages.INVALID_MEDIA_QUERY_SYNTAX);
  }
}
