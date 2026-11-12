/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

import type { InjectableStyle, StyleXOptions } from '../common-types';
import { defaultOptions } from './default-options';

import generateLtr from '../physical-rtl/generate-ltr';
import generateRtl from '../physical-rtl/generate-rtl';
import { getPriority } from '@stylexjs/shared';

const THUMB_VARIANTS = [
  '::-webkit-slider-thumb',
  '::-moz-range-thumb',
  '::-ms-thumb',
];

function buildNestedCSSRule(
  className: string,
  decls: string,
  pseudos: $ReadOnlyArray<string>,
  atRules: $ReadOnlyArray<string>,
  constRules: $ReadOnlyArray<string>,
): string {
  // Pseudo-elements (::before, ::after, etc.) must come after pseudo-classes
  // in the selector. e.g. `.class:hover::before` not `.class::before:hover`
  const { classes, elements } = pseudos.reduce(
    (acc, p) => {
      if (p === '::thumb') return acc;

      if (p.startsWith('::')) {
        acc.elements += p;
      } else {
        acc.classes += p;
      }
      return acc;
    },
    { classes: '', elements: '' },
  );

  const pseudo = classes + elements;
  const combinedAtRules = atRules.concat(constRules);

  // Bump specificity of stylex.when selectors
  const hasWhere = pseudo.includes(':where(');
  const extraClassForWhere = hasWhere ? `.${className}` : '';

  let selectorForAtRules =
    `.${className}` +
    extraClassForWhere +
    combinedAtRules.map(() => `.${className}`).join('') +
    pseudo;

  if (pseudos.includes('::thumb')) {
    selectorForAtRules = THUMB_VARIANTS.map(
      (suffix) => selectorForAtRules + suffix,
    ).join(', ');
  }

  return combinedAtRules.reduce(
    (acc, combinedAtRules) => `${combinedAtRules}{${acc}}`,
    `${selectorForAtRules}{${decls}}`,
  );
}

export function generateCSSRule(
  className: string,
  key: string,
  value: string | $ReadOnlyArray<string>,
  pseudos: $ReadOnlyArray<string>,
  atRules: $ReadOnlyArray<string>,
  constRules: $ReadOnlyArray<string>,
  options: StyleXOptions = defaultOptions,
): InjectableStyle {
  const pairs: $ReadOnlyArray<[string, string]> = Array.isArray(value)
    ? value.map((eachValue) => [key, eachValue])
    : [[key, value]];

  const ltrPairs = pairs.map((pair) => generateLtr(pair, options));
  const ltrDecls = ltrPairs.map((pair) => pair.join(':')).join(';');

  const rtlDecls = pairs
    .map((pair) => generateRtl(pair, options))
    .filter(Boolean)
    .map((pair) => pair.join(':'))
    .join(';');

  const ltrRule = buildNestedCSSRule(
    className,
    ltrDecls,
    pseudos,
    atRules,
    constRules,
  );
  const rtlRule = !rtlDecls
    ? null
    : buildNestedCSSRule(className, rtlDecls, pseudos, atRules, constRules);

  const priority =
    getPriority(key) +
    pseudos.map(getPriority).reduce((a, b) => a + b, 0) +
    atRules.map(getPriority).reduce((a, b) => a + b, 0) +
    constRules.map(getPriority).reduce((a, b) => a + b, 0);

  return { priority, ltr: ltrRule, rtl: rtlRule };
}
