/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { StyleXOptions } from '../common-types';

export const defaultOptions: StyleXOptions = {
  classNamePrefix: 'x',
  dev: false,
  debug: false,
  propertyValidationMode: 'silent',
  enableDevClassNames: false,
  enableDebugDataProp: true,
  enableFontSizePxToRem: false,
  enableInlinedConditionalMerge: true,
  enableMediaQueryOrder: true,
  enableLegacyValueFlipping: false,
  enableLogicalStylesPolyfill: false,
  enableLTRRTLComments: false,
  enableMinifiedKeys: true,
  styleResolution: 'property-specificity',
  // runtimeInjection: false,
  importSources: [],
  treeshakeCompensation: false,
  test: false,
};
