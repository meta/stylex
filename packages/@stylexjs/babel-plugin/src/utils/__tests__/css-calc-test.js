/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { isCssVarOrCalc, isCalcTerm, buildBinaryCalc } from '../css-calc';

describe('CSS arithmetic operand boundaries', () => {
  test('rejects partial or compound strings', () => {
    expect(isCssVarOrCalc('var(--a) var(--b)')).toBe(false);
    expect(isCssVarOrCalc('calc(1) + calc(2)')).toBe(false);
    expect(isCssVarOrCalc('solid var(--a)')).toBe(false);
    expect(isCssVarOrCalc('10px')).toBe(false);
    expect(isCssVarOrCalc(10)).toBe(false);
    expect(isCssVarOrCalc(null)).toBe(false);
  });

  test('accepts finite numbers', () => {
    expect(isCalcTerm(10)).toBe(true);
    expect(isCalcTerm(-0.5)).toBe(true);
    expect(isCalcTerm(NaN)).toBe(false);
    expect(isCalcTerm(Infinity)).toBe(false);
  });

  test('accepts numeric strings with CSS units', () => {
    expect(isCalcTerm('10px')).toBe(true);
    expect(isCalcTerm('1.5rem')).toBe(true);
    expect(isCalcTerm('50%')).toBe(true);
    expect(isCalcTerm('-2em')).toBe(true);
    expect(isCalcTerm('10')).toBe(true);
  });

  test('rejects non-numeric strings', () => {
    expect(isCalcTerm('solid ')).toBe(false);
    expect(isCalcTerm('auto')).toBe(false);
    expect(isCalcTerm('px')).toBe(false);
    expect(isCalcTerm('10px 20px')).toBe(false);
  });

  test('preserves the finite divisor Number.MAX_VALUE', () => {
    expect(buildBinaryCalc('var(--a)', '/', Number.MAX_VALUE)).toBe(
      'calc(var(--a) / 1.7976931348623157e+308)',
    );
  });
});
