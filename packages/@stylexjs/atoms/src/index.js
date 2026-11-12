/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

'use strict';

const errorMessage = (prop) =>
  "'@stylexjs/atoms' must be compiled away by '@stylexjs/babel-plugin'. " +
  `Attempted to access '${prop}' at runtime.`;

const _proxy = new Proxy(
  {},
  {
    get(target, prop) {
      if (typeof prop === 'symbol') {
        return target[prop];
      }
      if (prop === 'default' || prop === '__esModule') {
        return target[prop];
      }
      throw new Error(errorMessage(prop));
    },
  },
);

module.exports = _proxy;
module.exports.default = _proxy;
