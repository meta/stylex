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
import {
  objFromEntries,
  objEntries,
  objMapKeys,
  objMap,
  Pipe,
} from './utils/object-utils';
import { defaultOptions } from './utils/default-options';
import transformValue from './utils/transform-value';
import dashify from './utils/dashify';

type StyleValue = { +[string]: string | number };
type StyleObj = {
  group?: StyleValue,
  imagePair?: StyleValue,
  old?: StyleValue,
  new?: StyleValue,
};

export default function styleXViewTransitionClass(
  styles: StyleObj,
  options: StyleXOptions = defaultOptions,
): [string, InjectableStyle] {
  const { classNamePrefix = 'x' } = options;
  const preprocessedObject = objMap(styles, (style) =>
    Pipe.create(style)
      .pipe((style) => preprocessProperties(style, options))
      .pipe((x) =>
        objMap(x, (value, key) => transformValue(key, value, options)),
      )
      .pipe((x) => objMapKeys(x, dashify))
      .done(),
  );

  const expandedObject = objMapKeys(
    preprocessedObject,
    (k) => `::view-transition-${dashify(k)}`,
  );

  const styleStrings = objMap(
    expandedObject,
    constructViewTransitionClassStyleStr,
  );

  const viewTransitionClassName =
    classNamePrefix +
    createHash(constructViewTransitionClassStyleStr(styleStrings));

  const style = constructFinalViewTransitionCSSStr(
    styleStrings,
    viewTransitionClassName,
  );

  return [viewTransitionClassName, { ltr: style, rtl: null, priority: 1 }];
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

function constructViewTransitionClassStyleStr(style: {
  +[string]: string,
}): string {
  return objEntries(style)
    .map(([k, v]) => `${k}:${v};`)
    .join('');
}

function constructFinalViewTransitionCSSStr(
  styles: { +[string]: string },
  className: string,
): string {
  return objEntries(styles)
    .map(([k, v]) => `${k}(*.${className}){${v}}`)
    .join('');
}
