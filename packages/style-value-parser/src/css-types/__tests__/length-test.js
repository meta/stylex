/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { Length } from '../length';

describe('Test CSS Type: <length>', () => {
  test('parses font-based units', () => {
    expect(Length.parser.parse('10ch')).toEqual(new Length(10, 'ch'));
    expect(Length.parser.parse('2em')).toEqual(new Length(2, 'em'));
    expect(Length.parser.parse('1.5ex')).toEqual(new Length(1.5, 'ex'));
    expect(Length.parser.parse('3ic')).toEqual(new Length(3, 'ic'));
    expect(Length.parser.parse('1.2lh')).toEqual(new Length(1.2, 'lh'));
    expect(Length.parser.parse('2rem')).toEqual(new Length(2, 'rem'));
    expect(Length.parser.parse('1rlh')).toEqual(new Length(1, 'rlh'));
  });

  test('parses viewport-based units', () => {
    expect(Length.parser.parse('50vh')).toEqual(new Length(50, 'vh'));
    expect(Length.parser.parse('100vw')).toEqual(new Length(100, 'vw'));
    expect(Length.parser.parse('80svh')).toEqual(new Length(80, 'svh'));
    expect(Length.parser.parse('90lvw')).toEqual(new Length(90, 'lvw'));
    expect(Length.parser.parse('70dvh')).toEqual(new Length(70, 'dvh'));
    expect(Length.parser.parse('60vmin')).toEqual(new Length(60, 'vmin'));
    expect(Length.parser.parse('85vmax')).toEqual(new Length(85, 'vmax'));
  });

  test('parses container-based units', () => {
    expect(Length.parser.parse('30cqw')).toEqual(new Length(30, 'cqw'));
    expect(Length.parser.parse('40cqi')).toEqual(new Length(40, 'cqi'));
    expect(Length.parser.parse('50cqh')).toEqual(new Length(50, 'cqh'));
    expect(Length.parser.parse('60cqb')).toEqual(new Length(60, 'cqb'));
    expect(Length.parser.parse('45cqmin')).toEqual(new Length(45, 'cqmin'));
    expect(Length.parser.parse('75cqmax')).toEqual(new Length(75, 'cqmax'));
  });

  test('parses absolute units', () => {
    expect(Length.parser.parse('16px')).toEqual(new Length(16, 'px'));
    expect(Length.parser.parse('2cm')).toEqual(new Length(2, 'cm'));
    expect(Length.parser.parse('10mm')).toEqual(new Length(10, 'mm'));
    expect(Length.parser.parse('1in')).toEqual(new Length(1, 'in'));
    expect(Length.parser.parse('12pt')).toEqual(new Length(12, 'pt'));
    expect(Length.parser.parse('2pc')).toEqual(new Length(2, 'pc'));
    expect(Length.parser.parse('2q')).toEqual(new Length(2, 'q'));
  });

  test('parses root-relative lengths case-insensitively', () => {
    expect(Length.parser.parseToEnd('2rcap')).toEqual(new Length(2, 'rcap'));
    expect(Length.parser.parseToEnd('2RCAP')).toEqual(new Length(2, 'RCAP'));
  });

  test('parses logical and minimum viewport units', () => {
    expect(Length.parser.parseToEnd('2dvb')).toEqual(new Length(2, 'dvb'));
    expect(Length.parser.parseToEnd('2svmin')).toEqual(new Length(2, 'svmin'));
  });

  test('rejects invalid units', () => {
    expect(() => Length.parser.parseToEnd('10abc')).toThrow();
    expect(() => Length.parser.parseToEnd('30')).toThrow();
    expect(() => Length.parser.parseToEnd('xyz')).toThrow();
  });
});
