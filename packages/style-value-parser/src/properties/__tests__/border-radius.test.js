/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import {
  BorderRadiusIndividual,
  BorderRadiusShorthand,
} from '../border-radius';
import { Percentage } from '../../css-types/common-types';
import { Length } from '../../css-types/length';

describe('Test CSS property: `border-<dir>-<dir>-radius`', () => {
  test('Valid: border-<dir>-<dir>-radius: <length-percentage>', () => {
    expect(BorderRadiusIndividual.parse.parseToEnd('10px')).toEqual(
      new BorderRadiusIndividual(new Length(10, 'px')),
    );
    expect(BorderRadiusIndividual.parse.parseToEnd('0.5px')).toEqual(
      new BorderRadiusIndividual(new Length(0.5, 'px')),
    );
    expect(BorderRadiusIndividual.parse.parseToEnd('.5px')).toEqual(
      new BorderRadiusIndividual(new Length(0.5, 'px')),
    );
    expect(BorderRadiusIndividual.parse.parseToEnd('1rem')).toEqual(
      new BorderRadiusIndividual(new Length(1, 'rem')),
    );
  });

  test('Valid: border-<dir>-<dir>-radius: <length-percentage> <length-percentage>', () => {
    expect(BorderRadiusIndividual.parse.parseToEnd('10px 20px')).toEqual(
      new BorderRadiusIndividual(new Length(10, 'px'), new Length(20, 'px')),
    );
    expect(BorderRadiusIndividual.parse.parseToEnd('0.5px 2rem')).toEqual(
      new BorderRadiusIndividual(new Length(0.5, 'px'), new Length(2, 'rem')),
    );
    expect(BorderRadiusIndividual.parse.parseToEnd('.5px \n   4.5rem')).toEqual(
      new BorderRadiusIndividual(new Length(0.5, 'px'), new Length(4.5, 'rem')),
    );
    expect(BorderRadiusIndividual.parse.parseToEnd('1rem .0005px')).toEqual(
      new BorderRadiusIndividual(
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
      ),
    );
  });

  test('Invalid: border-<dir>-<dir>-radius', () => {
    expect(() => BorderRadiusIndividual.parse.parseToEnd('skchdj')).toThrow();
    expect(() => BorderRadiusIndividual.parse.parseToEnd('red')).toThrow();
    expect(() => BorderRadiusIndividual.parse.parseToEnd('10')).toThrow();
    expect(() => BorderRadiusIndividual.parse.parseToEnd('1.0 2.0')).toThrow();
    expect(() => BorderRadiusIndividual.parse.parseToEnd('0.5')).toThrow();
    expect(() => BorderRadiusIndividual.parse.parseToEnd('1.5')).toThrow();
    expect(() => BorderRadiusIndividual.parse.parseToEnd('1.5 20')).toThrow();
  });
});

describe('Test CSS property shorthand: `border-radius`', () => {
  test('Valid: border-radius: <length-percentage>', () => {
    expect(BorderRadiusShorthand.parse.parseToEnd('10px')).toEqual(
      new BorderRadiusShorthand(new Length(10, 'px')),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('0.5px')).toEqual(
      new BorderRadiusShorthand(new Length(0.5, 'px')),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('.5px')).toEqual(
      new BorderRadiusShorthand(new Length(0.5, 'px')),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('1rem')).toEqual(
      new BorderRadiusShorthand(new Length(1, 'rem')),
    );
  });

  test('Valid: border-radius: <length-percentage> <length-percentage>', () => {
    expect(BorderRadiusShorthand.parse.parseToEnd('10px 20px')).toEqual(
      new BorderRadiusShorthand(new Length(10, 'px'), new Length(20, 'px')),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('0.5px 2rem')).toEqual(
      new BorderRadiusShorthand(new Length(0.5, 'px'), new Length(2, 'rem')),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('.5px \n   4.5rem')).toEqual(
      new BorderRadiusShorthand(new Length(0.5, 'px'), new Length(4.5, 'rem')),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('1rem .0005px')).toEqual(
      new BorderRadiusShorthand(new Length(1, 'rem'), new Length(0.0005, 'px')),
    );
  });

  test('Valid: border-radius: <length-percentage> <length-percentage> <length-percentage>', () => {
    expect(BorderRadiusShorthand.parse.parseToEnd('10px 20px 30px')).toEqual(
      new BorderRadiusShorthand(
        new Length(10, 'px'),
        new Length(20, 'px'),
        new Length(30, 'px'),
      ),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('0.5px 2rem 3rem')).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(2, 'rem'),
        new Length(3, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('.5px \n   4.5rem 6rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(4.5, 'rem'),
        new Length(6, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('1rem .0005px 0.0005rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
        new Length(0.0005, 'rem'),
      ),
    );
  });

  test('Valid: border-radius: <length-percentage> <length-percentage> <length-percentage> <length-percentage>', () => {
    expect(
      BorderRadiusShorthand.parse.parseToEnd('10px 20px 30px 40px'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(10, 'px'),
        new Length(20, 'px'),
        new Length(30, 'px'),
        new Length(40, 'px'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('0.5px 2rem 3rem 4rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(2, 'rem'),
        new Length(3, 'rem'),
        new Length(4, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('.5px \n   4.5rem 6rem 7rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(4.5, 'rem'),
        new Length(6, 'rem'),
        new Length(7, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('1rem .0005px 0.0005rem 0.5rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
        new Length(0.0005, 'rem'),
        new Length(0.5, 'rem'),
      ),
    );
  });

  test('Valid: border-radius: <percentage>...', () => {
    expect(BorderRadiusShorthand.parse.parseToEnd('50%')).toEqual(
      new BorderRadiusShorthand(new Percentage(50)),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('0.5%')).toEqual(
      new BorderRadiusShorthand(new Percentage(0.5)),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('.5%')).toEqual(
      new BorderRadiusShorthand(new Percentage(0.5)),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('10% 20%')).toEqual(
      new BorderRadiusShorthand(new Percentage(10), new Percentage(20)),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('10% 20% 30%')).toEqual(
      new BorderRadiusShorthand(
        new Percentage(10),
        new Percentage(20),
        new Percentage(30),
      ),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('10% 20% 30% 40%')).toEqual(
      new BorderRadiusShorthand(
        new Percentage(10),
        new Percentage(20),
        new Percentage(30),
        new Percentage(40),
      ),
    );
  });

  // Asymmetric border-radius
  test('Valid: border-radius: <length-percentage> / <length-percentage>', () => {
    expect(BorderRadiusShorthand.parse.parseToEnd('10px / 20px')).toEqual(
      new BorderRadiusShorthand(
        new Length(10, 'px'),
        new Length(10, 'px'),
        new Length(10, 'px'),
        new Length(10, 'px'),
        new Length(20, 'px'),
      ),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('0.5px / 2rem')).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(0.5, 'px'),
        new Length(0.5, 'px'),
        new Length(0.5, 'px'),
        new Length(2, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('.5px \n   / 4.5rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(0.5, 'px'),
        new Length(0.5, 'px'),
        new Length(0.5, 'px'),
        new Length(4.5, 'rem'),
      ),
    );
    expect(BorderRadiusShorthand.parse.parseToEnd('1rem / .0005px')).toEqual(
      new BorderRadiusShorthand(
        new Length(1, 'rem'),
        new Length(1, 'rem'),
        new Length(1, 'rem'),
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
      ),
    );
  });

  test('toString: distinct vertical radii are not overwritten by horizontal', () => {
    // Horizontal group collapses to a single value (all 5px); the vertical
    // group is fully asymmetric (1/2/3/4), so it must be serialized as-is and
    // not fall back to the horizontal values.
    const radius = new BorderRadiusShorthand(
      new Length(5, 'px'),
      new Length(5, 'px'),
      new Length(5, 'px'),
      new Length(5, 'px'),
      new Length(1, 'px'),
      new Length(2, 'px'),
      new Length(3, 'px'),
      new Length(4, 'px'),
    );
    expect(radius.toString()).toBe('5px / 1px 2px 3px 4px');
  });

  test('Valid: border-radius: <length-percentage> <length-percentage> / <length-percentage> <length-percentage>', () => {
    expect(
      BorderRadiusShorthand.parse.parseToEnd('10px 20px / 30px 40px'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(10, 'px'),
        new Length(20, 'px'),
        new Length(10, 'px'),
        new Length(20, 'px'),
        new Length(30, 'px'),
        new Length(40, 'px'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('0.5px 2rem / 3rem 4rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(2, 'rem'),
        new Length(0.5, 'px'),
        new Length(2, 'rem'),
        new Length(3, 'rem'),
        new Length(4, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('.5px \n   4.5rem / 6rem 7rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(4.5, 'rem'),
        new Length(0.5, 'px'),
        new Length(4.5, 'rem'),
        new Length(6, 'rem'),
        new Length(7, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd('1rem .0005px / 0.0005rem 0.5rem'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
        new Length(0.0005, 'rem'),
        new Length(0.5, 'rem'),
      ),
    );
  });

  test('Valid: border-radius: <length-percentage> <length-percentage> <length-percentage> / <length-percentage> <length-percentage> <length-percentage>', () => {
    expect(
      BorderRadiusShorthand.parse.parseToEnd('10px 20px 30px / 40px 50px 60px'),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(10, 'px'),
        new Length(20, 'px'),
        new Length(30, 'px'),
        new Length(20, 'px'),
        new Length(40, 'px'),
        new Length(50, 'px'),
        new Length(60, 'px'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd(
        '0.5px 2rem 3rem / 4rem 5rem 6rem',
      ),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(2, 'rem'),
        new Length(3, 'rem'),
        new Length(2, 'rem'),
        new Length(4, 'rem'),
        new Length(5, 'rem'),
        new Length(6, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd(
        '.5px \n   4.5rem 6rem / 7rem 8rem 9rem',
      ),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(0.5, 'px'),
        new Length(4.5, 'rem'),
        new Length(6, 'rem'),
        new Length(4.5, 'rem'),
        new Length(7, 'rem'),
        new Length(8, 'rem'),
        new Length(9, 'rem'),
      ),
    );
    expect(
      BorderRadiusShorthand.parse.parseToEnd(
        '1rem .0005px 0.0005rem / 0.5rem 0.6rem 0.7rem',
      ),
    ).toEqual(
      new BorderRadiusShorthand(
        new Length(1, 'rem'),
        new Length(0.0005, 'px'),
        new Length(0.0005, 'rem'),
        new Length(0.0005, 'px'),
        new Length(0.5, 'rem'),
        new Length(0.6, 'rem'),
        new Length(0.7, 'rem'),
      ),
    );
  });

  describe('serialization: `BorderRadiusShorthand.toString`', () => {
    const roundTrip = (input: string): string =>
      BorderRadiusShorthand.parse.parseToEnd(input).toString();

    test('symmetric radii serialize without a `/`', () => {
      expect(roundTrip('10px')).toBe('10px');
      expect(roundTrip('10px 20px')).toBe('10px 20px');
      expect(roundTrip('10px 20px 30px')).toBe('10px 20px 30px');
      expect(roundTrip('10px 20px 30px 40px')).toBe('10px 20px 30px 40px');
    });

    test('asymmetric radii keep the vertical values after the `/`', () => {
      // Regression: the vertical (post-slash) radii were previously rendered
      // using the horizontal values, which collapsed the whole value back to
      // the horizontal radii and dropped the vertical radii entirely.
      expect(roundTrip('10px 20px 30px 40px / 40px 30px 20px 10px')).toBe(
        '10px 20px 30px 40px / 40px 30px 20px 10px',
      );
      expect(roundTrip('10px / 20px')).toBe('10px / 20px');
      expect(roundTrip('10px 20px / 30px 40px')).toBe('10px 20px / 30px 40px');
    });
  });
});
