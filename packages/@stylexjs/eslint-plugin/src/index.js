/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import enforceExtension from './stylex-enforce-extension';
import enforceModuleScope from './stylex-enforce-module-scope';
import noLegacyContextualStyles from './stylex-no-legacy-contextual-styles';
import noLookaheadSelectors from './stylex-no-lookahead-selectors';
import noNonStandardStyles from './stylex-no-nonstandard-styles';
import noConflictingProps from './stylex-no-conflicting-props';
import noUnused from './stylex-no-unused';
import sortKeys from './stylex-sort-keys';
import validShorthands from './stylex-valid-shorthands';
import validStyles from './stylex-valid-styles';

const rules: {
  'enforce-extension': typeof enforceExtension,
  'enforce-module-scope': typeof enforceModuleScope,
  'no-legacy-contextual-styles': typeof noLegacyContextualStyles,
  'no-lookahead-selectors': typeof noLookaheadSelectors,
  'no-nonstandard-styles': typeof noNonStandardStyles,
  'no-conflicting-props': typeof noConflictingProps,
  'no-unused': typeof noUnused,
  'sort-keys': typeof sortKeys,
  'valid-shorthands': typeof validShorthands,
  'valid-styles': typeof validStyles,
} = {
  'enforce-extension': enforceExtension,
  'enforce-module-scope': enforceModuleScope,
  'no-legacy-contextual-styles': noLegacyContextualStyles,
  'no-lookahead-selectors': noLookaheadSelectors,
  'no-nonstandard-styles': noNonStandardStyles,
  'no-conflicting-props': noConflictingProps,
  'no-unused': noUnused,
  'sort-keys': sortKeys,
  'valid-shorthands': validShorthands,
  'valid-styles': validStyles,
};

export { rules };
