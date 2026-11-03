/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { Angle } from '../angle';
import { Percentage } from '../common-types';
import { Length } from '../length';
import {
  Matrix,
  Matrix3d,
  Perspective,
  Rotate,
  Rotate3d,
  RotateXYZ,
  Scale,
  Scale3d,
  ScaleAxis,
  Skew,
  TransformFunction,
  Translate,
  Translate3d,
  TranslateAxis,
} from '../transform-function';

describe('Test CSS Type: <transform-function>', () => {
  describe('Test <matrix> function', () => {
    test('valid uses', () => {
      expect(
        TransformFunction.parser.parseToEnd('matrix(1, 0, 0, 1, 0, 0)'),
      ).toEqual(new Matrix(1, 0, 0, 1, 0, 0));
      expect(
        TransformFunction.parser.parseToEnd(
          'matrix(1.2,0.2,  -1, 0.9, 0, 20 )',
        ),
      ).toEqual(new Matrix(1.2, 0.2, -1, 0.9, 0, 20));
      expect(
        TransformFunction.parser.parseToEnd(
          'matrix(\n.4,0,0.5,1.200,60,10   )',
        ),
      ).toEqual(new Matrix(0.4, 0, 0.5, 1.2, 60, 10));
      expect(
        TransformFunction.parser.parseToEnd('matrix(0.1, 1, -0.3, 1, 0, 0)'),
      ).toEqual(new Matrix(0.1, 1, -0.3, 1, 0, 0));
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() =>
        TransformFunction.parser.parseToEnd('matrix(1, 0, 0, 1, 0)'),
      ).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('matrix(1, 0, 0, 1, 0, 0, 0)'),
      ).toThrow();
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('matrix(1, 0, 0, 1, 0, foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('matrix(1px, 0, 0, 1, 0, 0)'),
      ).toThrow();
      // wrong separator
      expect(() =>
        TransformFunction.parser.parseToEnd('matrix(1 0 0 1 0 0)'),
      ).toThrow();
    });
  });

  describe('Test <matrix3d> function', () => {
    test('valid uses', () => {
      expect(
        TransformFunction.parser.parseToEnd(
          `matrix3d(
            1,  0, 0, 0, 
            0,  1, 0, 0,
            0, .5, 1, 0,
            0, 0, 0, 1
          )`,
        ),
      ).toEqual(
        // prettier-ignore
        new Matrix3d([
          1,   0, 0, 0,
          0,   1, 0, 0,
          0, 0.5, 1, 0,
          0,   0, 0, 1
        ]),
      );
      expect(
        TransformFunction.parser.parseToEnd(
          'matrix3d(-0.6,1.34788,0,0,-2.34788,-.6,0, 0,0,0,1,0,0,0,10,1)',
        ),
      ).toEqual(
        // prettier-ignore
        new Matrix3d([
          -0.6,  1.34788,  0, 0, 
          -2.34788, -0.6,  0, 0, 
           0,          0,  1, 0, 
           0,          0, 10, 1,
        ]),
      );
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() =>
        TransformFunction.parser.parseToEnd(
          `matrix3d(
            1,  0, 0, 0, 
            0,  1, 0, 0,
            0, .5, 1, 0,
            0, 0, 0
          )`,
        ),
      ).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd(
          `matrix3d(
            1,  0, 0, 0, 
            0,  1, 0, 0,
            0, .5, 1, 0,
            0, 0, 0, 1, 0
          )`,
        ),
      ).toThrow();
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd(
          `matrix3d(
            1,  0, 0, 0, 
            0,  1, 0, 0,
            0, .5, 1, 0,
            0, 0, 0, foo
          )`,
        ),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd(
          `matrix3d(
            1px,  0, 0, 0, 
            0,  1, 0, 0,
            0, .5, 1, 0,
            0, 0, 0, 1
          )`,
        ),
      ).toThrow();
    });
  });

  describe('Test <perspective> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('perspective(0)')).toEqual(
        new Perspective(new Length(0, '')),
      );
      expect(TransformFunction.parser.parseToEnd('perspective(100px)')).toEqual(
        new Perspective(new Length(100, 'px')),
      );
      expect(TransformFunction.parser.parseToEnd('perspective(1.5em)')).toEqual(
        new Perspective(new Length(1.5, 'em')),
      );
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('perspective(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('perspective(1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('perspective(1%)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('perspective(1deg)'),
      ).toThrow();
    });
  });

  describe('Test <rotate> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('rotate(0)')).toEqual(
        new Rotate(new Angle(0, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(45deg)')).toEqual(
        new Rotate(new Angle(45, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(90deg)')).toEqual(
        new Rotate(new Angle(90, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(180deg)')).toEqual(
        new Rotate(new Angle(180, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(270deg)')).toEqual(
        new Rotate(new Angle(270, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(-90deg)')).toEqual(
        new Rotate(new Angle(-90, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(0.5turn)')).toEqual(
        new Rotate(new Angle(0.5, 'turn')),
      ); // Changed from (0, 'deg') to (180, 'deg')
      expect(TransformFunction.parser.parseToEnd('rotate(2rad)')).toEqual(
        new Rotate(new Angle(2, 'rad')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(100grad)')).toEqual(
        new Rotate(new Angle(100, 'grad')),
      ); // Changed from (0, 'deg') to (100, 'grad')
      expect(TransformFunction.parser.parseToEnd('rotate(1.5deg)')).toEqual(
        new Rotate(new Angle(1.5, 'deg')),
      ); // Changed from (0, 'deg') to (1.5, 'deg')
      expect(TransformFunction.parser.parseToEnd('rotate(360deg)')).toEqual(
        new Rotate(new Angle(360, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(1turn)')).toEqual(
        new Rotate(new Angle(1, 'turn')),
      );
      expect(TransformFunction.parser.parseToEnd('rotate(-1turn)')).toEqual(
        new Rotate(new Angle(-1, 'turn')),
      );
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() => TransformFunction.parser.parseToEnd('rotate(1)')).toThrow();
      expect(() => TransformFunction.parser.parseToEnd('rotate(1%)')).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate(1px)'),
      ).toThrow();
    });
  });

  describe('Test <rotate3d> function', () => {
    test('valid uses', () => {
      expect(
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, 0)'),
      ).toEqual(new Rotate3d(0, 0, 1, new Angle(0, 'deg')));
      expect(
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, 45deg)'),
      ).toEqual(new Rotate3d(0, 0, 1, new Angle(45, 'deg')));
      expect(
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, .1, 90deg)'),
      ).toEqual(new Rotate3d(0, 0, 0.1, new Angle(90, 'deg')));
      expect(
        TransformFunction.parser.parseToEnd('rotate3d(0, 0.5, 1, 180rad)'),
      ).toEqual(new Rotate3d(0, 0.5, 1, new Angle(180, 'rad')));
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1)'),
      ).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, 0, 0)'),
      ).toThrow();
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, 1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, 1%)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('rotate3d(0, 0, 1, 1px)'),
      ).toThrow();
    });
  });

  describe('Test <rotate-axis> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('rotateX(0)')).toEqual(
        new RotateXYZ(new Angle(0, 'deg'), 'X'),
      );
      expect(TransformFunction.parser.parseToEnd('rotateX(  0   )')).toEqual(
        new RotateXYZ(new Angle(0, 'deg'), 'X'),
      );
      expect(TransformFunction.parser.parseToEnd('rotateY( 45deg\n)')).toEqual(
        new RotateXYZ(new Angle(45, 'deg'), 'Y'),
      );
      expect(
        TransformFunction.parser.parseToEnd('rotateZ(   90deg     )'),
      ).toEqual(new RotateXYZ(new Angle(90, 'deg'), 'Z'));
      expect(TransformFunction.parser.parseToEnd('rotateZ( 90rad )')).toEqual(
        new RotateXYZ(new Angle(90, 'rad'), 'Z'),
      );
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('rotateX(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() => TransformFunction.parser.parseToEnd('rotateX(1)')).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('rotateX(1%)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('rotateX(1px)'),
      ).toThrow();
    });
  });

  describe('Test <scale> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('scale(0)')).toEqual(
        new Scale(0),
      );
      expect(TransformFunction.parser.parseToEnd('scale(1.5)')).toEqual(
        new Scale(1.5),
      );
      expect(TransformFunction.parser.parseToEnd('scale(1.5, 2)')).toEqual(
        new Scale(1.5, 2),
      );
      expect(TransformFunction.parser.parseToEnd('scale(150%, 2.5)')).toEqual(
        new Scale(1.5, 2.5),
      );
      expect(TransformFunction.parser.parseToEnd('scale(1.5, 2.5)')).toEqual(
        new Scale(1.5, 2.5),
      );
      expect(TransformFunction.parser.parseToEnd('scale(1.5, 2.5)')).toEqual(
        new Scale(1.5, 2.5),
      );
      // Negative values must be preserved (e.g. scale(-1) flips/mirrors).
      expect(TransformFunction.parser.parseToEnd('scale(-1)')).toEqual(
        new Scale(-1),
      );
      expect(TransformFunction.parser.parseToEnd('scale(-1.5, -2)')).toEqual(
        new Scale(-1.5, -2),
      );
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() => TransformFunction.parser.parseToEnd('scale()')).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('scale(1, 2, 3)'),
      ).toThrow();
      // Non-numeric values
      expect(() => TransformFunction.parser.parseToEnd('scale(foo)')).toThrow();
      // wrong type of values
      expect(() => TransformFunction.parser.parseToEnd('scale(1px)')).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('scale(1, 1px)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('scale(1px, 1)'),
      ).toThrow();
    });
  });

  describe('Test <scale3d> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('scale3d(0, 0, 1)')).toEqual(
        new Scale3d(0, 0, 1),
      );
      expect(
        TransformFunction.parser.parseToEnd('scale3d(1.5, 2, 0.5)'),
      ).toEqual(new Scale3d(1.5, 2, 0.5));
      expect(
        TransformFunction.parser.parseToEnd('scale3d(150%, 2.5, 1)'),
      ).toEqual(new Scale3d(1.5, 2.5, 1));
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() =>
        TransformFunction.parser.parseToEnd('scale3d(0, 0)'),
      ).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('scale3d(1, 2, 3, 4)'),
      ).toThrow();
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('scale3d(foo, 0, 1)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('scale3d(1px, 0, 1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('scale3d(1, 0, 1px)'),
      ).toThrow();
    });
  });

  describe('Test <scale-axis> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('scaleX(0)')).toEqual(
        new ScaleAxis(0, 'X'),
      );
      expect(TransformFunction.parser.parseToEnd('scaleX(  0   )')).toEqual(
        new ScaleAxis(0, 'X'),
      );
      expect(TransformFunction.parser.parseToEnd('scaleY( 45\n)')).toEqual(
        new ScaleAxis(45, 'Y'),
      );
      expect(TransformFunction.parser.parseToEnd('scaleY( 45%\n)')).toEqual(
        new ScaleAxis(0.45, 'Y'),
      );
      expect(TransformFunction.parser.parseToEnd('scaleZ(   90     )')).toEqual(
        new ScaleAxis(90, 'Z'),
      );
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('scaleX(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('scaleX(1px)'),
      ).toThrow();
      // too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('scaleX(1, 2)'),
      ).toThrow();
    });
  });

  describe('Test <skew> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('skew(0)')).toEqual(
        new Skew(new Angle(0, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('skew( 0   \n)')).toEqual(
        new Skew(new Angle(0, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('skew(15deg)')).toEqual(
        new Skew(new Angle(15, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('skew(\n15deg\n)')).toEqual(
        new Skew(new Angle(15, 'deg')),
      );
      expect(TransformFunction.parser.parseToEnd('skew(15deg, 2turn)')).toEqual(
        new Skew(new Angle(15, 'deg'), new Angle(2, 'turn')),
      );
      expect(TransformFunction.parser.parseToEnd('skew(15deg,2turn)')).toEqual(
        new Skew(new Angle(15, 'deg'), new Angle(2, 'turn')),
      );
      expect(
        TransformFunction.parser.parseToEnd('skew(   150deg,\n2.5rad)'),
      ).toEqual(new Skew(new Angle(150, 'deg'), new Angle(2.5, 'rad')));
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() => TransformFunction.parser.parseToEnd('skew()')).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('skew(1deg, 2deg, 3rad)'),
      ).toThrow();
      // Non-numeric values
      expect(() => TransformFunction.parser.parseToEnd('skew(foo)')).toThrow();
      // wrong type of values
      expect(() => TransformFunction.parser.parseToEnd('skew(1px)')).toThrow();
      expect(() => TransformFunction.parser.parseToEnd('skew(1, 1)')).toThrow();
      expect(() => TransformFunction.parser.parseToEnd('skew(1)')).toThrow();
    });
  });

  describe('Test <skew-axis> function', () => {
    test('valid uses', () => {
      expect(() =>
        TransformFunction.parser.parseToEnd('skewX(0)'),
      ).not.toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('skewX(  0   )'),
      ).not.toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('skewY( 45deg\n)'),
      ).not.toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('skewY( 45deg\n)'),
      ).not.toThrow();
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() => TransformFunction.parser.parseToEnd('skewX(foo)')).toThrow();
      // wrong unit
      expect(() => TransformFunction.parser.parseToEnd('skewX(1px)')).toThrow();
      expect(() => TransformFunction.parser.parseToEnd('skewX(1)')).toThrow();
      expect(() => TransformFunction.parser.parseToEnd('skewX(1ms)')).toThrow();
      expect(() => TransformFunction.parser.parseToEnd('skewX(.1)')).toThrow();
      // too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('skewX(1, 2)'),
      ).toThrow();
    });
  });

  describe('Test <translate> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('translate(0)')).toEqual(
        new Translate(new Length(0, '')),
      );
      expect(TransformFunction.parser.parseToEnd('translate( 0   \n)')).toEqual(
        new Translate(new Length(0, '')),
      );
      expect(TransformFunction.parser.parseToEnd('translate(15px)')).toEqual(
        new Translate(new Length(15, 'px')),
      );
      expect(
        TransformFunction.parser.parseToEnd('translate(\n15px\n)'),
      ).toEqual(new Translate(new Length(15, 'px')));
      expect(
        TransformFunction.parser.parseToEnd('translate(15px, 2em)'),
      ).toEqual(new Translate(new Length(15, 'px'), new Length(2, 'em')));
      expect(
        TransformFunction.parser.parseToEnd('translate(15px,2em)'),
      ).toEqual(new Translate(new Length(15, 'px'), new Length(2, 'em')));
      expect(
        TransformFunction.parser.parseToEnd('translate(   150%,\n2.5px)'),
      ).toEqual(new Translate(new Percentage(150), new Length(2.5, 'px')));
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate()'),
      ).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate(1px, 2px, 3px)'),
      ).toThrow();
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate(1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translate(1deg)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translate(1, 1px)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translate(1px, 1)'),
      ).toThrow();
    });
  });

  describe('Test <translate3d> function', () => {
    test('valid uses', () => {
      expect(Translate3d.parser.parseToEnd('translate3d(  0,0, 1px)')).toEqual(
        new Translate3d(
          new Length(0, ''),
          new Length(0, ''),
          new Length(1, 'px'),
        ),
      );
      expect(
        Translate3d.parser.parseToEnd('translate3d(1.5px, 2em,\n.5px)'),
      ).toEqual(
        new Translate3d(
          new Length(1.5, 'px'),
          new Length(2, 'em'),
          new Length(0.5, 'px'),
        ),
      );
      expect(
        Translate3d.parser.parseToEnd('translate3d(1.5px, 2em,\n0.5px)'),
      ).toEqual(
        new Translate3d(
          new Length(1.5, 'px'),
          new Length(2, 'em'),
          new Length(0.5, 'px'),
        ),
      );
      expect(
        Translate3d.parser.parseToEnd(
          'translate3d(\n\n\n150%,\n2.5em, 1px    )',
        ),
      ).toEqual(
        new Translate3d(
          new Percentage(150),
          new Length(2.5, 'em'),
          new Length(1, 'px'),
        ),
      );
    });
    test('invalid uses', () => {
      // Not enough values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate3d(0, 0)'),
      ).toThrow();
      // Too many values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate3d(1, 2, 3, 4)'),
      ).toThrow();
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate3d(foo, 0, 1)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('translate3d(1px, 0, 1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translate3d(1, 0, 1px)'),
      ).toThrow();
    });
  });

  describe('Test <translateX> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('translateX(  0   )')).toEqual(
        new TranslateAxis(new Length(0, ''), 'X'),
      );
      expect(TransformFunction.parser.parseToEnd('translateX(15px)')).toEqual(
        new TranslateAxis(new Length(15, 'px'), 'X'),
      );
      expect(
        TransformFunction.parser.parseToEnd('translateX(\n15px\n)'),
      ).toEqual(new TranslateAxis(new Length(15, 'px'), 'X'));
      expect(TransformFunction.parser.parseToEnd('translateX(150%)')).toEqual(
        new TranslateAxis(new Percentage(150), 'X'),
      );
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('translateX(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('translateX(1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translateX(20)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translateX(20s)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translateX(1deg)'),
      ).toThrow();
    });
  });

  describe('Test <translateY> function', () => {
    test('valid uses', () => {
      expect(TransformFunction.parser.parseToEnd('translateY(  0   )')).toEqual(
        new TranslateAxis(new Length(0, ''), 'Y'),
      );
      expect(TransformFunction.parser.parseToEnd('translateY(15px)')).toEqual(
        new TranslateAxis(new Length(15, 'px'), 'Y'),
      );
      expect(
        TransformFunction.parser.parseToEnd('translateY(\n15px\n)'),
      ).toEqual(new TranslateAxis(new Length(15, 'px'), 'Y'));
      expect(TransformFunction.parser.parseToEnd('translateY(150%)')).toEqual(
        new TranslateAxis(new Percentage(150), 'Y'),
      );
    });
    test('invalid uses', () => {
      // Non-numeric values
      expect(() =>
        TransformFunction.parser.parseToEnd('translateY(foo)'),
      ).toThrow();
      // wrong type of values
      expect(() =>
        TransformFunction.parser.parseToEnd('translateY(1)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translateY(20)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translateY(20s)'),
      ).toThrow();
      expect(() =>
        TransformFunction.parser.parseToEnd('translateY(1deg)'),
      ).toThrow();
    });
  });
});
