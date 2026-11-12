/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @noflow
 */

'use strict';

import * as stylex from '../src/stylex';

describe('stylex', () => {
  describe('error handling', () => {
    [
      'create',
      'createTheme',
      'defineConsts',
      'defineVars',
      'firstThatWorks',
      'keyframes',
      'positionTry',
      'viewTransitionClass',
      'defaultMarker',
      'defineMarker',
      'unstable_conditional',
      'unstable_defineVarsNested',
      'unstable_defineConstsNested',
      'unstable_createThemeNested',
    ].forEach((api) => {
      test(`stylex.${api}`, () => {
        expect(() => stylex[api]()).toThrow();
      });
    });

    [
      'angle',
      'color',
      'url',
      'image',
      'integer',
      'lengthPercentage',
      'length',
      'percentage',
      'number',
      'resolution',
      'time',
      'transformFunction',
      'transformList',
    ].forEach((api) => {
      test(`stylex.types${api}`, () => {
        expect(() => stylex.types[api]()).toThrow();
      });
    });

    [
      'ancestor',
      'descendant',
      'siblingBefore',
      'siblingAfter',
      'anySibling',
    ].forEach((api) => {
      test(`stylex.when${api}`, () => {
        expect(() => stylex.when[api](':hover')).toThrow();
      });
    });
  });

  describe('props', () => {
    test('basic resolve', () => {
      expect(stylex.props({ a: 'aaa', b: 'bbb', $$css: true }).className).toBe(
        'aaa bbb',
      );
    });

    test('merge order', () => {
      expect(
        stylex.props([
          { a: 'a', ':hover__aa': 'aa', $$css: true },
          { b: 'b', $$css: true },
          { c: 'c', ':hover__cc': 'cc', $$css: true },
        ]).className,
      ).toBe('a aa b c cc');
    });

    test('with a top-level array of simple overridden classes', () => {
      expect(
        stylex.props([
          {
            backgroundColor: 'nu7423ey',
            $$css: true,
          },
          {
            backgroundColor: 'gh25dzvf',
            $$css: true,
          },
        ]).className,
      ).toEqual('gh25dzvf');
    });

    test('with nested arrays and pseudoClasses overriding things', () => {
      expect(
        stylex.props([
          {
            backgroundColor: 'nu7423ey',
            $$css: true,
          },
          [
            {
              backgroundColor: 'abcdefg',
              ':hover__backgroundColor': 'ksdfmwjs',
              $$css: true,
            },
          ],
          {
            color: 'gofk2cf1',
            ':hover__backgroundColor': 'rse6dlih',
            $$css: true,
          },
        ]).className,
      ).toEqual('abcdefg gofk2cf1 rse6dlih');
    });

    test('with just pseudoclasses', () => {
      expect(
        stylex.props(
          {
            ':hover__backgroundColor': 'rse6dlih',
            $$css: true,
          },
          {
            ':hover__color': 'gofk2cf1',
            $$css: true,
          },
        ).className,
      ).toEqual('rse6dlih gofk2cf1');
    });

    test('with complicated set of arguments', () => {
      const styles = [
        {
          backgroundColor: 'nu7423ey',
          borderColor: 'tpe1esc0',
          borderStyle: 'gewhe1h2',
          borderWidth: 'gcovof34',
          boxSizing: 'bdao358l',
          display: 'rse6dlih',
          listStyle: 's5oniofx',
          marginTop: 'm8h3af8h',
          marginEnd: 'l7ghb35v',
          marginBottom: 'kjdc1dyq',
          marginStart: 'kmwttqpk',
          paddingTop: 'srn514ro',
          paddingEnd: 'oxkhqvkx',
          paddingBottom: 'rl78xhln',
          paddingStart: 'nch0832m',
          WebkitTapHighlightColor: 'qi72231t',
          textAlign: 'cr00lzj9',
          textDecoration: 'rn8ck1ys',
          whiteSpace: 'n3t5jt4f',
          wordWrap: 'gh25dzvf',
          zIndex: 'g4tp4svg',
          $$css: true,
        },
        false,
        false,
        false,
        false,
        [
          {
            cursor: 'fsf7x5fv',
            touchAction: 's3jn8y49',
            $$css: true,
          },
          false,
          {
            outline: 'icdlwmnq',
            $$css: true,
          },
          [
            {
              WebkitTapHighlightColor: 'oajrlxb2',
              cursor: 'nhd2j8a9',
              touchAction: 'f1sip0of',
              $$css: true,
            },
            false,
            false,
            {
              textDecoration: 'esuyzwwr',
              ':hover__textDecoration': 'p8dawk7l',
              $$css: true,
            },
            false,
            [
              {
                backgroundColor: 'g5ia77u1',
                border: 'e4t7hp5w',
                color: 'gmql0nx0',
                cursor: 'nhd2j8a9',
                display: 'q9uorilb',
                fontFamily: 'ihxqhq3m',
                fontSize: 'l94mrbxd',
                lineHeight: 'aenfhxwr',
                marginTop: 'kvgmc6g5',
                marginEnd: 'cxmmr5t8',
                marginBottom: 'oygrvhab',
                marginStart: 'hcukyx3x',
                paddingTop: 'jb3vyjys',
                paddingEnd: 'rz4wbd8a',
                paddingBottom: 'qt6c0cv9',
                paddingStart: 'a8nywdso',
                textAlign: 'i1ao9s8h',
                textDecoration: 'myohyog2',
                ':hover__color': 'ksdfmwjs',
                ':hover__textDecoration': 'gofk2cf1',
                ':active__transform': 'lsqurvkf',
                ':active__transition': 'bj9fd4vl',
                $$css: true,
              },
              {
                display: 'a8c37x1j',
                width: 'k4urcfbm',
                $$css: true,
              },
              [
                {
                  ':active__transform': 'tm8avpzi',
                  $$css: true,
                },
              ],
            ],
          ],
        ],
      ];

      const value = stylex.props(styles).className;
      const repeat = stylex.props(styles).className;

      // Check the cached-derived result is correct
      expect(value).toEqual(repeat);

      expect(value.split(' ').sort().join(' ')).toEqual(
        'g5ia77u1 tpe1esc0 gewhe1h2 gcovof34 bdao358l a8c37x1j s5oniofx kvgmc6g5 cxmmr5t8 oygrvhab hcukyx3x jb3vyjys rz4wbd8a qt6c0cv9 a8nywdso oajrlxb2 i1ao9s8h myohyog2 n3t5jt4f gh25dzvf g4tp4svg nhd2j8a9 f1sip0of icdlwmnq e4t7hp5w gmql0nx0 ihxqhq3m l94mrbxd aenfhxwr k4urcfbm gofk2cf1 ksdfmwjs tm8avpzi bj9fd4vl'
          .split(' ')
          .sort()
          .join(' '),
      );
    });

    test('with dynamic styles', () => {
      expect(
        stylex.props([
          {
            backgroundColor: 'backgroundColor-red',
            $$css: 'components/Foo.react.js:1',
          },
          {
            color: 'red',
          },
        ]),
      ).toMatchInlineSnapshot(`
        {
          "className": "backgroundColor-red",
          "data-style-src": "components/Foo.react.js:1",
          "style": {
            "color": "red",
          },
        }
      `);
    });

    test('data prop for source map data', () => {
      expect(
        stylex.props([
          {
            backgroundColor: 'backgroundColor-red',
            $$css: 'components/Foo.react.js:1',
          },
          {
            color: 'color-blue',
            $$css: 'components/Bar.react.js:3',
          },
          [
            {
              display: 'display-block',
              $$css: 'components/Baz.react.js:5',
            },
          ],
        ]),
      ).toMatchInlineSnapshot(`
        {
          "className": "backgroundColor-red color-blue display-block",
          "data-style-src": "components/Foo.react.js:1; components/Bar.react.js:3; components/Baz.react.js:5",
        }
      `);
    });
  });

  describe('attrs', () => {
    test('basic resolve', () => {
      expect(stylex.attrs({ a: 'aaa', b: 'bbb', $$css: true }).class).toBe(
        'aaa bbb',
      );
    });

    test('with dynamic styles', () => {
      expect(
        stylex.attrs([
          {
            backgroundColor: 'backgroundColor-red',
            $$css: 'components/Foo.react.js:1',
          },
          {
            color: 'red',
            marginTop: '10px',
            opacity: 0.5,
            '--foo': 2,
            '--fooBar': 3,
            MsTransition: 'none',
            WebkitTapHighlightColor: 'transparent',
          },
        ]),
      ).toMatchInlineSnapshot(`
        {
          "class": "backgroundColor-red",
          "data-style-src": "components/Foo.react.js:1",
          "style": "color:red;margin-top:10px;opacity:0.5;--foo:2;--fooBar:3;-ms-transition:none;-webkit-tap-highlight-color:transparent",
        }
      `);
    });

    test('legacyMerge merges classNames', () => {
      expect(
        stylex.legacyMerge(
          { color: 'color-red', $$css: true },
          { backgroundColor: 'bg-blue', $$css: true },
        ),
      ).toBe('color-red bg-blue');
    });

    test('legacyMerge exposes attrs', () => {
      expect(
        stylex.legacyMerge.attrs({
          color: 'color-red',
          $$css: true,
        }),
      ).toEqual({
        class: 'color-red',
      });
    });
  });
});
