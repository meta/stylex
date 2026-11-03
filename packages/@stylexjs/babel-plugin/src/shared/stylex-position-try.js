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
import flatMapProperties from './preprocess-rules/index';
import generateLtr from './physical-rtl/generate-ltr';
import generateRtl from './physical-rtl/generate-rtl';
import transformValue from './utils/transform-value';
import dashify from './utils/dashify';
import {
  objFromEntries,
  objEntries,
  objMapKeys,
  objMap,
  objMapEntry,
  Pipe,
} from './utils/object-utils';
import { defaultOptions } from './utils/default-options';

// Similar to `stylex.keyframes` but for position-try-fallbacks
// Takes an object of positioning properties and returns a string after hashing it
// The generated string must be prefixed with -- for anchor positioning
export default function styleXPositionTry(
  styles: { +[string]: string | number },
  options: StyleXOptions = defaultOptions,
): [string, InjectableStyle] {
  const { classNamePrefix = 'x' } = options;
  const expandedObject = Pipe.create(styles)
    .pipe((styles) => preprocessProperties(styles, options))
    .pipe((x) => objMapKeys(x, dashify))
    .pipe((x) => objMap(x, (value, key) => transformValue(key, value, options)))
    .done();

  // The generators return a `[key, value]` pair; a direction flip can rewrite the key.
  const ltrStyles = objMapEntry(expandedObject, (entry) =>
    generateLtr(entry, options),
  );
  const rtlStyles = objMapEntry(
    expandedObject,
    (entry) => generateRtl(entry, options) ?? entry,
  );

  const ltrString = constructPositionTryObj(ltrStyles);
  const rtlString = constructPositionTryObj(rtlStyles);

  const positionTryName = '--' + classNamePrefix + createHash(ltrString);

  const ltr = `@position-try ${positionTryName} {${ltrString}}`;
  const rtl =
    ltrString === rtlString
      ? null
      : `@position-try ${positionTryName} {${rtlString}}`;

  return [positionTryName, { ltr, rtl, priority: 0 }];
}

function preprocessProperties(
  styles: { +[key: string]: string | number },
  options: StyleXOptions,
): {
  +[key: string]: string | number,
} {
  return objFromEntries(
    objEntries(styles)
      .flatMap((pair): $ReadOnlyArray<[string, string | number]> =>
        flatMapProperties(pair, options)
          .map(([key, value]) => {
            if (typeof value === 'string' || typeof value === 'number') {
              return [key, value];
            }
            return null;
          })
          .filter(Boolean),
      )
      .filter(([_key, value]) => value != null),
  );
}

// NOTE: This implementation does not account for different values within
// `@position-try` for pseudo-classes and media queries.
// Instead, we suggest defining multiple `@position-try` at-rules for each
// pseudo-class or media query, and using different values for `positionTryFallsbacks`
// instead.
function constructPositionTryObj(styles: {
  +[string]: string | $ReadOnlyArray<string>,
}): string {
  return (
    Object.keys(styles)
      // Sorting keys to ensure a deterministic output for the same styles.
      .sort()
      .map((k) => {
        const v = styles[k];
        return (Array.isArray(v) ? v : [v])
          .map((val) => `${k}:${val};`)
          .join('');
      })
      .join('')
  );
}
