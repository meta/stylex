/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const {
  getFixtureNames,
  readExpectedOutput,
  renderGoldenFixture,
} = require('../test-utils/golden-fixtures');

describe('@stylexjs/babel-plugin golden fixtures', () => {
  test.each(getFixtureNames())('%s', (fixtureName) => {
    expect(renderGoldenFixture(fixtureName)).toEqual(
      readExpectedOutput(fixtureName),
    );
  });
});
