/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { Angle } from './angle';
import { Flex } from './flex';
import { Frequency } from './frequency';
import { Length } from './length';
import { Resolution } from './resolution';
import { Time } from './time';

// Recognition is shared with the type parsers. Whether a unit is appropriate
// for a particular CSS property or arithmetic operation is a separate policy.
const numericUnits: Set<string> = new Set([
  ...Length.UNITS,
  ...Angle.UNITS,
  ...Time.UNITS,
  ...Frequency.UNITS.map((unit) => unit.toLowerCase()),
  ...Resolution.UNITS,
  ...Flex.UNITS,
  '%',
]);

export function isNumericUnit(unit: string): boolean {
  return numericUnits.has(unit.toLowerCase());
}
