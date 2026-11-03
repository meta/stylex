/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { Length } from '../length';
import { Inset, Circle, Ellipse, Polygon, Path } from '../basic-shape';
import { Position } from '../position';
import { Percentage } from '../common-types';

describe('Basic Shapes', () => {
  describe('Inset', () => {
    it('should parse valid insets', () => {
      expect(Inset.parser.parseToEnd('inset(10px)')).toEqual(
        new Inset(
          new Length(10, 'px'),
          new Length(10, 'px'),
          new Length(10, 'px'),
          new Length(10, 'px'),
        ),
      );
      expect(Inset.parser.parseToEnd('inset(10px 20px)')).toEqual(
        new Inset(
          new Length(10, 'px'),
          new Length(20, 'px'),
          new Length(10, 'px'),
          new Length(20, 'px'),
        ),
      );
      expect(Inset.parser.parseToEnd('inset(10px 20px 30px)')).toEqual(
        new Inset(
          new Length(10, 'px'),
          new Length(20, 'px'),
          new Length(30, 'px'),
          new Length(20, 'px'),
        ),
      );
      expect(Inset.parser.parseToEnd('inset(10px 20px 30px 40px)')).toEqual(
        new Inset(
          new Length(10, 'px'),
          new Length(20, 'px'),
          new Length(30, 'px'),
          new Length(40, 'px'),
        ),
      );
      expect(Inset.parser.parseToEnd('inset(10px round 5px)')).toEqual(
        new Inset(
          new Length(10, 'px'),
          new Length(10, 'px'),
          new Length(10, 'px'),
          new Length(10, 'px'),
          new Length(5, 'px'),
        ),
      );
    });

    it('should not parse invalid insets', () => {
      expect(() => Inset.parser.parseToEnd('inset(invalid)')).toThrow();
      expect(() => Inset.parser.parseToEnd('inset(10px, invalid)')).toThrow();
    });

    it('should stringify the shortest valid form', () => {
      const roundtrip = (css: string): string =>
        Inset.parser.parseToEnd(css).toString();
      // 1 value: all four sides equal.
      expect(roundtrip('inset(10px)')).toBe('inset(10px)');
      // 2 values: vertical / horizontal.
      expect(roundtrip('inset(10px 20px)')).toBe('inset(10px 20px)');
      // 3 values: top / horizontal / bottom (left === right).
      expect(roundtrip('inset(10px 20px 30px)')).toBe('inset(10px 20px 30px)');
      // 4 values: all distinct (no trailing space before the paren).
      expect(roundtrip('inset(10px 20px 30px 40px)')).toBe(
        'inset(10px 20px 30px 40px)',
      );
      // round radius is preserved.
      expect(roundtrip('inset(10px round 5px)')).toBe('inset(10px round 5px)');
    });
  });

  describe('Circle', () => {
    it('should parse valid circles', () => {
      expect(Circle.parser.parseToEnd('circle(10px)')).toEqual(
        new Circle(new Length(10, 'px')),
      );
      expect(Circle.parser.parseToEnd('circle(closest-side)')).toEqual(
        new Circle('closest-side'),
      );
      expect(Circle.parser.parseToEnd('circle(farthest-side)')).toEqual(
        new Circle('farthest-side'),
      );
      expect(Circle.parser.parseToEnd('circle(10px at top left)')).toEqual(
        new Circle(new Length(10, 'px'), new Position('left', 'top')),
      );
    });

    it('should not parse invalid circles', () => {
      expect(() => Circle.parser.parseToEnd('circle(invalid)')).toThrow();
      expect(() => Circle.parser.parseToEnd('circle(10px, invalid)')).toThrow();
    });
  });

  describe('Ellipse', () => {
    it('should parse valid ellipses', () => {
      expect(Ellipse.parser.parseToEnd('ellipse(10px 20px)')).toEqual(
        new Ellipse(new Length(10, 'px'), new Length(20, 'px')),
      );
      expect(
        Ellipse.parser.parseToEnd('ellipse(closest-side farthest-side)'),
      ).toEqual(new Ellipse('closest-side', 'farthest-side'));
      expect(
        Ellipse.parser.parseToEnd('ellipse(10px 20px at top left)'),
      ).toEqual(
        new Ellipse(
          new Length(10, 'px'),
          new Length(20, 'px'),
          new Position('left', 'top'),
        ),
      );
    });

    it('should not parse invalid ellipses', () => {
      expect(() => Ellipse.parser.parseToEnd('ellipse(invalid)')).toThrow();
      expect(() =>
        Ellipse.parser.parseToEnd('ellipse(10px, invalid)'),
      ).toThrow();
    });
  });

  describe('Polygon', () => {
    it('should parse valid polygons', () => {
      expect(
        Polygon.parser.parseToEnd(
          'polygon(0% 0%, 100% 0%, 100% 100%, 0% 100%)',
        ),
      ).toEqual(
        new Polygon([
          [new Percentage(0), new Percentage(0)],
          [new Percentage(100), new Percentage(0)],
          [new Percentage(100), new Percentage(100)],
          [new Percentage(0), new Percentage(100)],
        ]),
      );
      expect(
        Polygon.parser.parseToEnd(
          'polygon(evenodd, 0% 0%, 100% 0%, 100% 100%, 0% 100%)',
        ),
      ).toEqual(
        new Polygon(
          [
            [new Percentage(0), new Percentage(0)],
            [new Percentage(100), new Percentage(0)],
            [new Percentage(100), new Percentage(100)],
            [new Percentage(0), new Percentage(100)],
          ],
          'evenodd',
        ),
      );
    });

    it('should not parse invalid polygons', () => {
      expect(() => Polygon.parser.parseToEnd('polygon(invalid)')).toThrow();
      expect(() =>
        Polygon.parser.parseToEnd('polygon(0% 0%, invalid)'),
      ).toThrow();
    });
  });

  describe('Path', () => {
    it('should parse valid paths', () => {
      expect(Path.parser.parseToEnd('path("M0,0 L100,100")')).toEqual(
        new Path('M0,0 L100,100'),
      );
      expect(Path.parser.parseToEnd('path(evenodd, "M0,0 L100,100")')).toEqual(
        new Path('M0,0 L100,100', 'evenodd'),
      );
    });

    it('should not parse invalid paths', () => {
      expect(() => Path.parser.parseToEnd('path(invalid)')).toThrow();
      expect(() => Path.parser.parseToEnd('path()')).toThrow();
    });
  });
});
