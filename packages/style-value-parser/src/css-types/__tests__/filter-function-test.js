/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import {
  FilterFunction,
  BlurFilterFunction,
  BrightnessFilterFunction,
  ContrastFilterFunction,
  GrayscaleFilterFunction,
  HueRotateFilterFunction,
  InverFilterFunction,
  OpacityFilterFunction,
  SaturateFilterFunction,
  SepiaFilterFunction,
} from '../filter-function';
import { Length } from '../length';
import { Angle } from '../angle';

describe('Test CSS Type: <filter-function>', () => {
  test('parses blur filter', () => {
    expect(FilterFunction.parser.parse('blur(5px)')).toEqual(
      new BlurFilterFunction(new Length(5, 'px')),
    );
  });

  test('parses brightness filter', () => {
    expect(FilterFunction.parser.parse('brightness(150%)')).toEqual(
      new BrightnessFilterFunction(1.5),
    );
  });

  test('parses contrast filter', () => {
    expect(FilterFunction.parser.parse('contrast(200%)')).toEqual(
      new ContrastFilterFunction(2),
    );
  });

  test('parses grayscale filter', () => {
    expect(FilterFunction.parser.parse('grayscale(50%)')).toEqual(
      new GrayscaleFilterFunction(0.5),
    );
  });

  test('parses hue-rotate filter', () => {
    expect(FilterFunction.parser.parse('hue-rotate(90deg)')).toEqual(
      new HueRotateFilterFunction(new Angle(90, 'deg')),
    );
  });

  test('parses invert filter', () => {
    expect(FilterFunction.parser.parse('invert(100%)')).toEqual(
      new InverFilterFunction(1),
    );
  });

  test('parses opacity filter', () => {
    expect(FilterFunction.parser.parse('opacity(75%)')).toEqual(
      new OpacityFilterFunction(0.75),
    );
  });

  test('parses saturate filter', () => {
    expect(FilterFunction.parser.parse('saturate(120%)')).toEqual(
      new SaturateFilterFunction(1.2),
    );
  });

  test('parses sepia filter', () => {
    expect(FilterFunction.parser.parse('sepia(30%)')).toEqual(
      new SepiaFilterFunction(0.3),
    );
  });

  test('rejects invalid filter functions', () => {
    expect(() => FilterFunction.parser.parseToEnd('invalid()')).toThrow();
    expect(() => FilterFunction.parser.parseToEnd('blur()')).toThrow();
    expect(() => FilterFunction.parser.parseToEnd('brightness(abc)')).toThrow();
    expect(() => FilterFunction.parser.parseToEnd('unknown(2)')).toThrow();
    expect(() => FilterFunction.parser.parseToEnd('unknown(5px)')).toThrow();
    expect(() => FilterFunction.parser.parseToEnd('unknown(90deg)')).toThrow();
  });

  test('keeps the function name when serializing', () => {
    for (const filter of [
      'blur(5px)',
      'brightness(1.5)',
      'contrast(2)',
      'grayscale(0.5)',
      'hue-rotate(90deg)',
      'invert(1)',
      'opacity(0.75)',
      'saturate(1.2)',
      'sepia(0.3)',
    ]) {
      expect(FilterFunction.parser.parseToEnd(filter).toString()).toBe(filter);
    }
  });
});
