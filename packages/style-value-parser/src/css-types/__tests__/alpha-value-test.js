/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { AlphaValue } from '../alpha-value';

describe('Test CSS Type: <alpha-value>', () => {
  describe('Number Fractions', () => {
    test('0.5', () => {
      expect(AlphaValue.parser.parse('0.5')).toEqual(new AlphaValue(0.5));
    });
    test('.5', () => {
      expect(AlphaValue.parser.parse('.5')).toEqual(new AlphaValue(0.5));
    });
    test('0.25', () => {
      expect(AlphaValue.parser.parse('0.25')).toEqual(new AlphaValue(0.25));
    });
    test('.25', () => {
      expect(AlphaValue.parser.parse('.25')).toEqual(new AlphaValue(0.25));
    });
    test('0.75', () => {
      expect(AlphaValue.parser.parse('0.75')).toEqual(new AlphaValue(0.75));
    });
    test('.75', () => {
      expect(AlphaValue.parser.parse('.75')).toEqual(new AlphaValue(0.75));
    });
    test('1', () => {
      expect(AlphaValue.parser.parse('1')).toEqual(new AlphaValue(1));
    });
    test('parses decimal alpha values', () => {
      expect(AlphaValue.parser.parse('0')).toEqual(new AlphaValue(0));
      expect(AlphaValue.parser.parse('0.25')).toEqual(new AlphaValue(0.25));
      expect(AlphaValue.parser.parse('0.5')).toEqual(new AlphaValue(0.5));
      expect(AlphaValue.parser.parse('1')).toEqual(new AlphaValue(1));
    });
    test('preserves the sign of negative alpha values', () => {
      expect(AlphaValue.parser.parse('-0.5')).toEqual(new AlphaValue(-0.5));
      expect(AlphaValue.parser.parse('-50%')).toEqual(new AlphaValue(-0.5));
    });
  });
  describe('Percentages', () => {
    test('50%', () => {
      expect(AlphaValue.parser.parse('50%')).toEqual(new AlphaValue(0.5));
    });
    test('25%', () => {
      expect(AlphaValue.parser.parse('25%')).toEqual(new AlphaValue(0.25));
    });
    test('75%', () => {
      expect(AlphaValue.parser.parse('75%')).toEqual(new AlphaValue(0.75));
    });
    test('75.5%', () => {
      expect(AlphaValue.parser.parse('75.5%')).toEqual(new AlphaValue(0.755));
    });
    test('0.25%', () => {
      expect(AlphaValue.parser.parse('0.25%')).toEqual(new AlphaValue(0.0025));
    });
    test('.25%', () => {
      expect(AlphaValue.parser.parse('.25%')).toEqual(new AlphaValue(0.0025));
    });
    test('parses percentage alpha values', () => {
      expect(AlphaValue.parser.parse('0%')).toEqual(new AlphaValue(0));
      expect(AlphaValue.parser.parse('25%')).toEqual(new AlphaValue(0.25));
      expect(AlphaValue.parser.parse('50%')).toEqual(new AlphaValue(0.5));
      expect(AlphaValue.parser.parse('100%')).toEqual(new AlphaValue(1));
    });
  });
  describe('Rejects', () => {
    test('rejects invalid alpha values', () => {
      expect(() => AlphaValue.parser.parseToEnd('invalid')).toThrow();
      expect(() => AlphaValue.parser.parseToEnd('red')).toThrow();
      expect(() => AlphaValue.parser.parseToEnd('initial')).toThrow();
    });
  });
});
