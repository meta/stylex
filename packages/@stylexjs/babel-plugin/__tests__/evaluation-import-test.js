/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();
jest.mock('@dual-bundle/import-meta-resolve');

/* eslint-disable quotes */
const { transformSync } = require('@babel/core');
const stylexPlugin = require('../src/index');
const jsx = require('@babel/plugin-syntax-jsx');
const { utils } = require('../src/shared');
const { moduleResolve } = require('@dual-bundle/import-meta-resolve');

const hash = utils.hash;

const options = {
  classNamePrefix: '__hashed_var__',
};

function transform(source, opts = options) {
  return transformSync(source, {
    filename: opts.filename ?? 'test.js',
    parserOpts: {
      flow: 'all',
    },
    babelrc: false,
    plugins: [
      jsx,
      [
        stylexPlugin,
        {
          treeshakeCompensation: true,
          runtimeInjection: true,
          unstable_moduleResolution: { type: 'haste' },
          ...opts,
        },
      ],
    ],
  });
}

describe('Evaluation of imported values works based on configuration', () => {
  describe('Theme name hashing based on fileName alone works', () => {
    test('Importing file with ".stylex" suffix works', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex';
        const styles = stylex.create({
          red: {
            color: MyTheme.foreground,
          }
        });
        stylex(styles.red);
      `);
      const expectedVarName = `var(--${options.classNamePrefix}${hash(
        'otherFile.stylex.js//MyTheme.foreground',
      )})`;
      expect(expectedVarName).toMatchInlineSnapshot(
        `"var(--__hashed_var__1jqb1tb)"`,
      );
      expect(transformation.code).toContain(expectedVarName);
      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex';
        import { MyTheme } from 'otherFile.stylex';
        _inject2({
          ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
          priority: 3000
        });
        "__hashed_var__1r7rkhg";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1r7rkhg",
            {
              "ltr": ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
              "rtl": null,
            },
            3000,
          ],
        ]
      `);
    });

    test('Importing file with ".stylex" and reading __varGroupHash__ returns a className', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex';
        const styles = stylex.create({
          red: {
            color: MyTheme.__varGroupHash__,
          }
        });
        stylex(styles.red);
      `);
      const expectedVarName =
        options.classNamePrefix + hash('otherFile.stylex.js//MyTheme');
      expect(expectedVarName).toMatchInlineSnapshot(`"__hashed_var__jvfbhb"`);
      expect(transformation.code).toContain(expectedVarName);
      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex';
        import { MyTheme } from 'otherFile.stylex';
        _inject2({
          ltr: ".__hashed_var__1yh36a2{color:__hashed_var__jvfbhb}",
          priority: 3000
        });
        "__hashed_var__1yh36a2";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1yh36a2",
            {
              "ltr": ".__hashed_var__1yh36a2{color:__hashed_var__jvfbhb}",
              "rtl": null,
            },
            3000,
          ],
        ]
      `);
    });

    test('Maintains variable names that start with -- from "*.stylex" files', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex';
        const styles = stylex.create({
          red: {
            color: MyTheme['--foreground'],
          }
        });
        stylex(styles.red);
      `);
      const expectedVarName = 'var(--foreground)';
      expect(expectedVarName).toMatchInlineSnapshot(`"var(--foreground)"`);
      expect(transformation.code).toContain(expectedVarName);
      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex';
        import { MyTheme } from 'otherFile.stylex';
        _inject2({
          ltr: ".__hashed_var__11jfisy{color:var(--foreground)}",
          priority: 3000
        });
        "__hashed_var__11jfisy";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__11jfisy",
            {
              "ltr": ".__hashed_var__11jfisy{color:var(--foreground)}",
              "rtl": null,
            },
            3000,
          ],
        ]
      `);
    });

    test('Importing file with ".stylex" suffix works with keyframes', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex';
        const fade = stylex.keyframes({
          from: {
            color: MyTheme.foreground,
          }
        });
        const styles = stylex.create({
          red: {
            animationName: fade,
          }
        });
        stylex(styles.red);
      `);
      const expectedVarName = `var(--${options.classNamePrefix}${hash(
        'otherFile.stylex.js//MyTheme.foreground',
      )})`;
      expect(expectedVarName).toMatchInlineSnapshot(
        `"var(--__hashed_var__1jqb1tb)"`,
      );
      expect(transformation.code).toContain(expectedVarName);
      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex';
        import { MyTheme } from 'otherFile.stylex';
        _inject2({
          ltr: "@keyframes __hashed_var__1cb153o-B{from{color:var(--__hashed_var__1jqb1tb);}}",
          priority: 0
        });
        const fade = "__hashed_var__1cb153o-B";
        _inject2({
          ltr: ".__hashed_var__1xwo6t1{animation-name:__hashed_var__1cb153o-B}",
          priority: 3000
        });
        "__hashed_var__1xwo6t1";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1cb153o-B",
            {
              "ltr": "@keyframes __hashed_var__1cb153o-B{from{color:var(--__hashed_var__1jqb1tb);}}",
              "rtl": null,
            },
            0,
          ],
          [
            "__hashed_var__1xwo6t1",
            {
              "ltr": ".__hashed_var__1xwo6t1{animation-name:__hashed_var__1cb153o-B}",
              "rtl": null,
            },
            3000,
          ],
        ]
      `);
    });

    test('Importing file with ".stylex.js" suffix works', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex.js';
        const styles = stylex.create({
          red: {
            color: MyTheme.foreground,
          }
        });
        stylex(styles.red);
      `);
      const expectedVarName = `var(--${options.classNamePrefix}${hash(
        'otherFile.stylex.js//MyTheme.foreground',
      )})`;
      expect(expectedVarName).toMatchInlineSnapshot(
        `"var(--__hashed_var__1jqb1tb)"`,
      );
      expect(transformation.code).toContain(expectedVarName);
      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex.js';
        import { MyTheme } from 'otherFile.stylex.js';
        _inject2({
          ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
          priority: 3000
        });
        "__hashed_var__1r7rkhg";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1r7rkhg",
            {
              "ltr": ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
              "rtl": null,
            },
            3000,
          ],
        ]
      `);
    });
    test('Importing file with ".stylex.js" with an alias suffix works', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme as mt } from 'otherFile.stylex.js';
        const styles = stylex.create({
          red: {
            color: mt.foreground,
          }
        });
        stylex(styles.red);
      `);
      const expectedVarName = `var(--${options.classNamePrefix}${hash(
        'otherFile.stylex.js//MyTheme.foreground',
      )})`;
      expect(expectedVarName).toMatchInlineSnapshot(
        `"var(--__hashed_var__1jqb1tb)"`,
      );
      expect(transformation.code).toContain(expectedVarName);
      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex.js';
        import { MyTheme as mt } from 'otherFile.stylex.js';
        _inject2({
          ltr: ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
          priority: 3000
        });
        "__hashed_var__1r7rkhg";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1r7rkhg",
            {
              "ltr": ".__hashed_var__1r7rkhg{color:var(--__hashed_var__1jqb1tb)}",
              "rtl": null,
            },
            3000,
          ],
        ]
      `);
    });

    test('Importing file without a ".stylex" suffix fails', () => {
      const transformation = () =>
        transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile';
        const styles = stylex.create({
          red: {
            color: MyTheme.foreground,
          }
        });
        stylex(styles.red);
      `);
      expect(transformation).toThrow();
    });

    test('Imported vars with ".stylex" suffix can be used as style keys', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex';
        const styles = stylex.create({
          red: {
            [MyTheme.foreground]: 'red',
          }
        });
        stylex(styles.red);
      `);

      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex';
        import { MyTheme } from 'otherFile.stylex';
        _inject2({
          ltr: ".__hashed_var__1g7q0my{--__hashed_var__1jqb1tb:red}",
          priority: 1
        });
        "__hashed_var__1g7q0my";"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1g7q0my",
            {
              "ltr": ".__hashed_var__1g7q0my{--__hashed_var__1jqb1tb:red}",
              "rtl": null,
            },
            1,
          ],
        ]
      `);
    });

    test('Imported vars with ".stylex" suffix can be used as style keys dynamically', () => {
      const transformation = transform(`
        import stylex from 'stylex';
        import { MyTheme } from 'otherFile.stylex';
        const styles = stylex.create({
          color: (color) => ({
            [MyTheme.foreground]: color,
          })
        });
        stylex.props(styles.color('red'));
      `);

      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import 'otherFile.stylex';
        import { MyTheme } from 'otherFile.stylex';
        _inject2({
          ltr: ".__hashed_var__1w8wjxo{--__hashed_var__1jqb1tb:var(--x---__hashed_var__1jqb1tb)}",
          priority: 1
        });
        _inject2({
          ltr: "@property --x---__hashed_var__1jqb1tb { syntax: \\"*\\"; inherits: false;}",
          priority: 0
        });
        const styles = {
          color: color => [color !== undefined && {
            "--__hashed_var__1jqb1tb": color != null ? "__hashed_var__1w8wjxo" : "",
            $$css: true
          }, {
            "--x---__hashed_var__1jqb1tb": color != null ? color : undefined
          }]
        };
        stylex.props(styles.color('red'));"
      `);
      expect(transformation.metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "__hashed_var__1w8wjxo",
            {
              "ltr": ".__hashed_var__1w8wjxo{--__hashed_var__1jqb1tb:var(--x---__hashed_var__1jqb1tb)}",
              "rtl": null,
            },
            1,
          ],
          [
            "--x---__hashed_var__1jqb1tb",
            {
              "ltr": "@property --x---__hashed_var__1jqb1tb { syntax: "*"; inherits: false;}",
              "rtl": null,
            },
            0,
          ],
        ]
      `);
    });
  });

  describe('Module resolution commonJS', () => {
    afterEach(() => {
      moduleResolve.mockReset();
    });

    test('Recognizes .ts stylex imports when resolving .js relative imports', () => {
      moduleResolve.mockImplementation((value) => {
        if (!value.endsWith('/otherFile.stylex.ts')) {
          throw new Error('File not found');
        }
        return new URL('file:///project/otherFile.stylex.ts');
      });

      const transformation = transform(
        `
        import stylex from 'stylex';
        import { MyTheme } from './otherFile.stylex.js';
        const styles = stylex.create({
          red: {
            color: MyTheme.__varGroupHash__,
          }
        });
        stylex(styles.red);
        `,
        {
          unstable_moduleResolution: { type: 'commonJS' },
        },
      );

      expect(transformation.code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        import './otherFile.stylex.js';
        import { MyTheme } from './otherFile.stylex.js';
        _inject2({
          ltr: ".xoh8dld{color:xb897f7}",
          priority: 3000
        });
        "xoh8dld";"
      `);
    });
  });
});
