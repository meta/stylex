/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { InjectableStyle, StyleXOptions } from './common-types';

import createHash from './hash';
import expandShorthands from './preprocess-rules/index';
import generateLtr from './physical-rtl/generate-ltr';
import generateRtl from './physical-rtl/generate-rtl';
import transformValue from './utils/transform-value';
import dashify from './utils/dashify';
import {
  objFromEntries,
  objEntries,
  objMapKeys,
  objMap,
  Pipe,
  objMapEntry,
} from './utils/object-utils';
import { defaultOptions } from './utils/default-options';

// Similar to `stylex.create` it takes an object of keyframes
// and returns a string after hashing it.
//
// It also expands shorthand properties to maintain parity with
// `stylex.create`.
export default function styleXKeyframes(
  frames: { +[string]: { +[string]: string | number } },
  options: StyleXOptions = defaultOptions,
): [string, InjectableStyle] {
  const { classNamePrefix = 'x' } = options;
  const expandedObject = objMap(frames, (frame) =>
    Pipe.create(frame)
      .pipe((frame) => expandFrameShorthands(frame, options))
      .pipe((x) =>
        objMap(x, (value, key) => transformValue(key, value, options)),
      )
      .pipe((x) => objMapKeys(x, dashify))
      .done(),
  );

  const ltrStyles = objMap(expandedObject, (frame) =>
    objMapEntry(frame, (entry) => generateLtr(entry, options)),
  );
  const rtlStyles = objMap(expandedObject, (frame) =>
    objMapEntry(frame, (entry) => generateRtl(entry, options) ?? entry),
  );
  const stableStyles = objMap(expandedObject, (frame) =>
    objMapEntry(frame, generateLtr),
  );

  const ltrString = constructKeyframesObj(ltrStyles);
  const rtlString = constructKeyframesObj(rtlStyles);
  const stableString = constructKeyframesObj(stableStyles);

  // NOTE: Use a direction-agnostic hash to keep LTR/RTL classnames stable across builds.
  // NOTE: '<>' and '-B' is used to keep existing hashes stable.
  // They should be removed in a future version.
  const animationName =
    classNamePrefix + createHash('<>' + stableString) + '-B';

  const ltr = `@keyframes ${animationName}{${ltrString}}`;
  const rtl =
    ltrString === rtlString
      ? null
      : `@keyframes ${animationName}{${rtlString}}`;

  return [animationName, { ltr, rtl, priority: 0 }];
}

function expandFrameShorthands(
  frame: { +[key: string]: string | number },
  options: StyleXOptions,
): {
  +[key: string]: string | number,
} {
  return objFromEntries(
    objEntries(frame)
      .flatMap((pair): $ReadOnlyArray<[string, string | number]> =>
        expandShorthands(pair, options)
          .map(([key, value]) => {
            if (typeof value === 'string' || typeof value === 'number') {
              return [key, value];
            }
            return null;
          })
          .filter(Boolean),
      )
      // Keyframes are not combined. The nulls can be skipped
      .filter(([_key, value]) => value != null),
  );
}

// Create t
function constructKeyframesObj(frames: {
  +[string]: { +[string]: string },
}): string {
  return objEntries(frames)
    .map(
      ([key, value]) =>
        `${key}{${objEntries(value)
          .map(([k, v]) => `${k}:${v};`)
          .join('')}}`,
    )
    .join('');
}
