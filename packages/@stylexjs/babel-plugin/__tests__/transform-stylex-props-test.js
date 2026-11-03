/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();

import { transformSync } from '@babel/core';
import flow from '@babel/plugin-syntax-flow';
import jsx from '@babel/plugin-syntax-jsx';
import typescript from '@babel/plugin-syntax-typescript';
import stylexPlugin from '../src/index';
import path from 'path';

function transform(source, opts = {}) {
  return transformWithSyntaxPlugins(source, [flow, jsx], opts);
}

function transformTypeScript(source, opts = {}) {
  return transformWithSyntaxPlugins(
    source,
    [jsx, [typescript, { allExtensions: true, isTSX: true }]],
    { filename: 'test.tsx', ...opts },
  );
}

function transformWithSyntaxPlugins(source, syntaxPlugins, opts = {}) {
  return transformSync(source, {
    filename: opts.filename,
    parserOpts: {
      sourceType: 'module',
      flow: 'all',
    },
    babelrc: false,
    plugins: [
      ...syntaxPlugins,
      [
        stylexPlugin,
        {
          unstable_moduleResolution: {
            rootDir: path.parse(process.cwd()).root,
            type: 'commonJS',
          },
          ...opts,
          runtimeInjection: true,
        },
      ],
    ],
  }).code;
}

const THIS_FILE = path.join(__dirname, 'transform-stylex-props-test.js');

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] stylex.props() call', () => {
    test('empty stylex.props call', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          stylex.props();
        `),
      ).toMatchInlineSnapshot(`
        "import stylex from 'stylex';
        ({});"
      `);
    });

    test('basic stylex call', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            red: {
              color: 'red',
            }
          });
          stylex.props(styles.red);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        ({
          className: "x1e2nbdu"
        });"
      `);
    });

    test('basic stylex attrs call', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            red: {
              color: 'red',
            }
          });
          stylex.attrs(styles.red);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        ({
          class: "x1e2nbdu"
        });"
      `);
    });

    test('stylex.env resolves in inline objects', () => {
      expect(
        transform(
          `
          import stylex from 'stylex';
          const styles = stylex.create({
            red: {
              color: stylex.env.primaryColor,
            }
          });
          stylex.props(styles.red);
        `,
          { env: { primaryColor: '#ff0000' } },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".xe4pkkx{color:#ff0000}",
          priority: 3000
        });
        ({
          className: "xe4pkkx"
        });"
      `);
    });

    test('stylex.attrs named import resolves in inline objects', () => {
      expect(
        transform(
          `
          import { attrs, create, env } from 'stylex';
          const styles = create({
            red: {
              color: env.primaryColor,
            }
          });
          attrs(styles.red);
        `,
          { env: { primaryColor: '#00ffaa' } },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import { attrs, create, env } from 'stylex';
        _inject2({
          ltr: ".x4iekqp{color:#00ffaa}",
          priority: 3000
        });
        ({
          class: "x4iekqp"
        });"
      `);
    });

    test('stylex.env named import resolves in inline objects', () => {
      expect(
        transform(
          `
          import { props, create, env } from 'stylex';
          const styles = create({
            red: {
              color: env.primaryColor,
            }
          });
          props(styles.red);
        `,
          { env: { primaryColor: '#00ffaa' } },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import { props, create, env } from 'stylex';
        _inject2({
          ltr: ".x4iekqp{color:#00ffaa}",
          priority: 3000
        });
        ({
          className: "x4iekqp"
        });"
      `);
    });

    describe('props calls with jsx', () => {
      const options = {
        debug: true,
        dev: true,
        enableDevClassNames: false,
        filename: '/js/node_modules/npm-package/dist/components/Foo.react.js',
      };

      test('local static styles', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              }
            });
            function Foo() {
              return (
                <>
                  <div id="test" {...stylex.props(styles.red)}>Hello World</div>
                  <div className="test" {...stylex.props(styles.red)} id="test">Hello World</div>
                  <div id="test" {...stylex.props(styles.red)} className="test">Hello World</div>
                </>
              );
            }
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          function Foo() {
            return <>
                            <div id="test" className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4">Hello World</div>
                            <div className="test" className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4" id="test">Hello World</div>
                            <div id="test" className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4" className="test">Hello World</div>
                          </>;
          }"
        `);
      });

      test('Test that sx attribute can be used instead of ...stylex.props', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              }
            });
            function Foo() {
              return (
                <>
                  <div id="test" sx={styles.red}>Hello World</div>
                  <div className="test" sx={styles.red} id="test">Hello World</div>
                  <div id="test" sx={styles.red} className="test">Hello World</div>
                </>
              );
            }
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          function Foo() {
            return <>
                            <div id="test" className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4">Hello World</div>
                            <div className="test" className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4" id="test">Hello World</div>
                            <div id="test" className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4" className="test">Hello World</div>
                          </>;
          }"
        `);
      });

      test('sx prop with custom sxPropName', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              }
            });
            function Foo() {
              return <div css={styles.red}>Hello World</div>;
            }
          `,
            { ...options, sxPropName: 'css' },
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          function Foo() {
            return <div className="x1e2nbdu" data-style-src="npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4">Hello World</div>;
          }"
        `);
      });

      describe('sx attribute runtime binding', () => {
        test('injects a value namespace import alongside a type namespace import', () => {
          expect(
            transformTypeScript(`
              import type * as stylex from '@stylexjs/stylex';
              function Foo(props) {
                const x = props.x;
                return <svg sx={x} />;
              }
            `),
          ).toMatchInlineSnapshot(`
            "import * as _stylex from "@stylexjs/stylex";
            import type * as stylex from '@stylexjs/stylex';
            function Foo(props) {
              const x = props.x;
              return <svg {..._stylex.props(x)} />;
            }"
          `);
        });

        test('injects a value namespace import alongside a named type import', () => {
          expect(
            transform(`
              import type { StyleXStyles } from '@stylexjs/stylex';
              function Foo(props) {
                const x = props.x;
                return <svg sx={x} />;
              }
            `),
          ).toMatchInlineSnapshot(`
            "import * as _stylex from "@stylexjs/stylex";
            import type { StyleXStyles } from '@stylexjs/stylex';
            function Foo(props) {
              const x = props.x;
              return <svg {..._stylex.props(x)} />;
            }"
          `);
        });

        test('injects a value namespace import when there is no stylex import', () => {
          expect(
            transform(`
              function Foo(props) {
                const x = props.x;
                return <svg sx={x} />;
              }
            `),
          ).toMatchInlineSnapshot(`
            "import * as stylex from "@stylexjs/stylex";
            function Foo(props) {
              const x = props.x;
              return <svg {...stylex.props(x)} />;
            }"
          `);
        });

        test('injects from configured import source when there is no stylex import', () => {
          expect(
            transform(
              `
                function Foo(props) {
                  const x = props.x;
                  return <svg sx={x} />;
                }
              `,
              { importSources: ['custom-stylex-path'] },
            ),
          ).toMatchInlineSnapshot(`
            "import * as stylex from "custom-stylex-path";
            function Foo(props) {
              const x = props.x;
              return <svg {...stylex.props(x)} />;
            }"
          `);
        });

        test('injects from existing type-only custom import source', () => {
          expect(
            transform(
              `
                import type { StyleXStyles } from 'custom-stylex-path';
                function Foo(props) {
                  const x = props.x;
                  return <svg sx={x} />;
                }
              `,
              { importSources: ['custom-stylex-path'] },
            ),
          ).toMatchInlineSnapshot(`
            "import * as _stylex from "custom-stylex-path";
            import type { StyleXStyles } from 'custom-stylex-path';
            function Foo(props) {
              const x = props.x;
              return <svg {..._stylex.props(x)} />;
            }"
          `);
        });

        test('reuses an existing value namespace import', () => {
          expect(
            transform(`
              import * as stylex from '@stylexjs/stylex';
              function Foo(props) {
                const x = props.x;
                return <svg sx={x} />;
              }
            `),
          ).toMatchInlineSnapshot(`
            "import * as stylex from '@stylexjs/stylex';
            function Foo(props) {
              const x = props.x;
              return <svg {...stylex.props(x)} />;
            }"
          `);
        });

        test('avoids a local stylex binding that would shadow the injected import', () => {
          expect(
            transform(`
              function Foo(props) {
                const stylex = props.stylex;
                const x = props.x;
                return <svg sx={x} />;
              }
            `),
          ).toMatchInlineSnapshot(`
            "import * as _stylex from "@stylexjs/stylex";
            function Foo(props) {
              const stylex = props.stylex;
              const x = props.x;
              return <svg {..._stylex.props(x)} />;
            }"
          `);
        });

        test('injects one value namespace import for multiple sx attributes', () => {
          expect(
            transform(`
              function Foo(props) {
                const x = props.x;
                return (
                  <>
                    <svg sx={x} />
                    <svg sx={props.y} />
                  </>
                );
              }
            `),
          ).toMatchInlineSnapshot(`
            "import * as stylex from "@stylexjs/stylex";
            function Foo(props) {
              const x = props.x;
              return <>
                                <svg {...stylex.props(x)} />
                                <svg {...stylex.props(props.y)} />
                              </>;
            }"
          `);
        });

        test('does not transform sx on capitalized components', () => {
          expect(
            transform(`
              function Foo(props) {
                const x = props.x;
                return <CustomComponent sx={x} />;
              }
            `),
          ).toMatchInlineSnapshot(`
            "function Foo(props) {
              const x = props.x;
              return <CustomComponent sx={x} />;
            }"
          `);
        });
      });

      test('local dynamic styles', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              opacity: (opacity) => ({
                opacity
              })
            });
            function Foo() {
              return (
                <div id="test" {...stylex.props(styles.red, styles.opacity(1))}>
                  Hello World
                </div>
              );
            }
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xb4nw82{opacity:var(--x-opacity)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-opacity { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const styles = {
            red: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4"
            },
            opacity: opacity => [opacity !== undefined && {
              "opacity-kSiTet": opacity != null ? "xb4nw82" : "",
              $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:7"
            }, {
              "--x-opacity": opacity != null ? opacity : undefined
            }]
          };
          function Foo() {
            return <div id="test" {...stylex.props(styles.red, styles.opacity(1))}>
                            Hello World
                          </div>;
          }"
        `);
      });

      test('non-local styles', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              }
            });
            function Foo(props) {
              return (
                <div id="test" {...stylex.props(props.style, styles.red)}>
                  Hello World
                </div>
              );
            }
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          const styles = {
            red: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "npm-package:js/node_modules/npm-package/dist/components/Foo.react.js:4"
            }
          };
          function Foo(props) {
            return <div id="test" {...stylex.props(props.style, styles.red)}>
                            Hello World
                          </div>;
          }"
        `);
      });
    });

    describe('props calls with atoms', () => {
      test('inline static styles match stylex.create', () => {
        const inline = transform(`
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          stylex.props(css.display.flex);
        `);
        expect(inline).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".x78zum5{display:flex}",
            priority: 3000
          });
          ({
            className: "x78zum5"
          });"
        `);
        const local = transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            flex: { display: 'flex' },
          });
          stylex.props(styles.flex);
        `);
        expect(local).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x78zum5{display:flex}",
            priority: 3000
          });
          ({
            className: "x78zum5"
          });"
        `);
      });

      test('inline static supports leading underscore value', () => {
        const inline = transform(`
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          stylex.props(css.padding._16px);
        `);
        expect(inline).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".x1tamke2{padding:16px}",
            priority: 1000
          });
          ({
            className: "x1tamke2"
          });"
        `);
        const local = transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            pad: { padding: '16px' },
          });
          stylex.props(styles.pad);
        `);
        expect(local).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1tamke2{padding:16px}",
            priority: 1000
          });
          ({
            className: "x1tamke2"
          });"
        `);
      });

      test('inline static supports computed key syntax', () => {
        const inline = transform(`
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          stylex.props(css.width['calc(100% - 20px)']);
        `);
        expect(inline).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".xnlsq7q{width:calc(100% - 20px)}",
            priority: 4000
          });
          ({
            className: "xnlsq7q"
          });"
        `);
        const local = transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            w: { width: 'calc(100% - 20px)' },
          });
          stylex.props(styles.w);
        `);
        expect(local).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".xnlsq7q{width:calc(100% - 20px)}",
            priority: 4000
          });
          ({
            className: "xnlsq7q"
          });"
        `);
      });

      test('inline css supports default imports', () => {
        const inline = transform(`
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          stylex.props(css.color.blue);
        `);
        expect(inline).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          ({
            className: "xju2f9n"
          });"
        `);
      });

      test('dedupes duplicate properties across create and atoms', () => {
        const output = transform(`
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          const styles = stylex.create({
            base: { color: 'red', backgroundColor: 'white' },
          });
          stylex.props(styles.base, css.color.blue, css.backgroundColor.white);
        `);
        expect(output).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".x12peec7{background-color:white}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          _inject2({
            ltr: ".x12peec7{background-color:white}",
            priority: 3000
          });
          ({
            className: "xju2f9n x12peec7"
          });"
        `);
      });

      test('dynamic style', () => {
        const inline = transform(`
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          stylex.props(css.color(color));
        `);
        const local = transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            color: (c) => ({ color: c }),
          });
          stylex.props(styles.color(color));
        `);
        expect(local).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x14rh7hd{color:var(--x-color)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const styles = {
            color: c => [c !== undefined && {
              kMwMTN: c != null ? "x14rh7hd" : "",
              $$css: true
            }, {
              "--x-color": c != null ? c : undefined
            }]
          };
          stylex.props(styles.color(color));"
        `);
        expect(inline).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".x14rh7hd{color:var(--x-color)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const _temp = {
            color: _v => [{
              "kMwMTN": _v != null ? "x14rh7hd" : _v,
              "$$css": true
            }, {
              "--x-color": _v != null ? _v : undefined
            }]
          };
          stylex.props(_temp.color(color));"
        `);
      });

      test('inline static + inline dynamic coexist', () => {
        const inline = transform(`
        import stylex from 'stylex';
        import css from '@stylexjs/atoms';
        stylex.props(css.display.flex, css.color(color));
      `);
        expect(inline).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".x78zum5{display:flex}",
            priority: 3000
          });
          _inject2({
            ltr: ".x14rh7hd{color:var(--x-color)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const _temp = {
            color: _v => [{
              "kMwMTN": _v != null ? "x14rh7hd" : _v,
              "$$css": true
            }, {
              "--x-color": _v != null ? _v : undefined
            }]
          };
          const _temp2 = {
            k1xSpc: "x78zum5",
            $$css: true
          };
          stylex.props(_temp2, _temp.color(color));"
        `);
      });

      test('inline static + create dynamic', () => {
        const output = transform(`
        import stylex from 'stylex';
        import css from '@stylexjs/atoms';
        const styles = stylex.create({
          opacity: (o) => ({ opacity: o }),
        });
        stylex.props(css.display.flex, styles.opacity(0.5));
      `);
        expect(output).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".xb4nw82{opacity:var(--x-opacity)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-opacity { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const styles = {
            opacity: o => [o !== undefined && {
              kSiTet: o != null ? "xb4nw82" : "",
              $$css: true
            }, {
              "--x-opacity": o != null ? o : undefined
            }]
          };
          _inject2({
            ltr: ".x78zum5{display:flex}",
            priority: 3000
          });
          const _temp = {
            k1xSpc: "x78zum5",
            $$css: true
          };
          stylex.props(_temp, styles.opacity(0.5));"
        `);
      });

      test('inline dynamic + create dynamic', () => {
        const output = transform(`
        import stylex from 'stylex';
        import css from '@stylexjs/atoms';
        const styles = stylex.create({
          opacity: (o) => ({ opacity: o }),
        });
        stylex.props(css.color(color), styles.opacity(0.5));
      `);
        expect(output).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          import css from '@stylexjs/atoms';
          _inject2({
            ltr: ".xb4nw82{opacity:var(--x-opacity)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-opacity { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const styles = {
            opacity: o => [o !== undefined && {
              kSiTet: o != null ? "xb4nw82" : "",
              $$css: true
            }, {
              "--x-opacity": o != null ? o : undefined
            }]
          };
          _inject2({
            ltr: ".x14rh7hd{color:var(--x-color)}",
            priority: 3000
          });
          _inject2({
            ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
            priority: 0
          });
          const _temp = {
            color: _v => [{
              "kMwMTN": _v != null ? "x14rh7hd" : _v,
              "$$css": true
            }, {
              "--x-color": _v != null ? _v : undefined
            }]
          };
          stylex.props(_temp.color(color), styles.opacity(0.5));"
        `);
      });

      describe('conditional atoms', () => {
        test('atoms with && conditional', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            stylex.props(css.display.flex, isActive && css.color.blue);
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000
            });
            ({
              0: {
                className: "x78zum5"
              },
              1: {
                className: "x78zum5 xju2f9n"
              }
            })[!!isActive << 0];"
          `);
        });

        test('atoms with ternary conditional', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            stylex.props(isActive ? css.color.blue : css.color.red);
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000
            });
            _inject2({
              ltr: ".x1e2nbdu{color:red}",
              priority: 3000
            });
            ({
              0: {
                className: "x1e2nbdu"
              },
              1: {
                className: "xju2f9n"
              }
            })[!!isActive << 0];"
          `);
        });

        test('atoms with conditional and create styles', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            const styles = stylex.create({
              active: { color: 'blue' },
            });
            stylex.props(css.display.flex, isActive && styles.active);
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000
            });
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            ({
              0: {
                className: "x78zum5"
              },
              1: {
                className: "x78zum5 xju2f9n"
              }
            })[!!isActive << 0];"
          `);
        });

        test('dynamic atoms with ternary conditional', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            stylex.props(isActive ? css.color(activeColor) : css.backgroundColor(inactiveBg));
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x14rh7hd{color:var(--x-color)}",
              priority: 3000
            });
            _inject2({
              ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
              priority: 0
            });
            const _temp = {
              color: _v => [{
                "kMwMTN": _v != null ? "x14rh7hd" : _v,
                "$$css": true
              }, {
                "--x-color": _v != null ? _v : undefined
              }]
            };
            _inject2({
              ltr: ".xl8spv7{background-color:var(--x-backgroundColor)}",
              priority: 3000
            });
            _inject2({
              ltr: "@property --x-backgroundColor { syntax: \\"*\\"; inherits: false;}",
              priority: 0
            });
            const _temp2 = {
              backgroundColor: _v => [{
                "kWkggS": _v != null ? "xl8spv7" : _v,
                "$$css": true
              }, {
                "--x-backgroundColor": _v != null ? _v : undefined
              }]
            };
            stylex.props(isActive ? _temp.color(activeColor) : _temp2.backgroundColor(inactiveBg));"
          `);
        });
      });

      describe('atoms with runtime bailout', () => {
        test('atoms mixed with unknown external styles', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            import { externalStyles } from './other.stylex';
            stylex.props(css.color.blue, externalStyles.root);
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            import { externalStyles } from './other.stylex';
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000
            });
            const _temp = {
              kMwMTN: "xju2f9n",
              $$css: true
            };
            stylex.props(_temp, externalStyles.root);"
          `);
        });

        test('atoms with dynamic conditional inside function', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            function Button({ isActive, color }) {
              return stylex.props(
                css.padding._8px,
                css.borderRadius._4px,
                isActive && css.color(color),
              );
            }
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".xe8ttls{padding:8px}",
              priority: 1000
            });
            _inject2({
              ltr: ".x12oqio5{border-radius:4px}",
              priority: 2000
            });
            _inject2({
              ltr: ".x14rh7hd{color:var(--x-color)}",
              priority: 3000
            });
            _inject2({
              ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
              priority: 0
            });
            const _temp = {
              color: _v => [{
                "kMwMTN": _v != null ? "x14rh7hd" : _v,
                "$$css": true
              }, {
                "--x-color": _v != null ? _v : undefined
              }]
            };
            const _temp2 = {
              kmVPX3: "xe8ttls",
              $$css: true
            };
            const _temp3 = {
              kaIpWk: "x12oqio5",
              $$css: true
            };
            function Button({
              isActive,
              color
            }) {
              return stylex.props(_temp2, _temp3, isActive && _temp.color(color));
            }"
          `);
        });

        test('atoms with variable reference forces bailout', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            function Component({ xstyle }) {
              return stylex.props(css.display.flex, xstyle);
            }
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            const _temp = {
              k1xSpc: "x78zum5",
              $$css: true
            };
            function Component({
              xstyle
            }) {
              return stylex.props(_temp, xstyle);
            }"
          `);
        });
      });

      describe('atoms inside functions', () => {
        test('static atoms inside function declaration', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            function Component() {
              return stylex.props(css.display.flex, css.color.blue);
            }
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000
            });
            function Component() {
              return {
                className: "x78zum5 xju2f9n"
              };
            }"
          `);
        });

        test('dynamic atoms inside function declaration', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            function Component(color) {
              return stylex.props(css.display.flex, css.color(color));
            }
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            _inject2({
              ltr: ".x14rh7hd{color:var(--x-color)}",
              priority: 3000
            });
            _inject2({
              ltr: "@property --x-color { syntax: \\"*\\"; inherits: false;}",
              priority: 0
            });
            const _temp = {
              color: _v => [{
                "kMwMTN": _v != null ? "x14rh7hd" : _v,
                "$$css": true
              }, {
                "--x-color": _v != null ? _v : undefined
              }]
            };
            const _temp2 = {
              k1xSpc: "x78zum5",
              $$css: true
            };
            function Component(color) {
              return stylex.props(_temp2, _temp.color(color));
            }"
          `);
        });

        test('atoms inside arrow function', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            const Component = () => stylex.props(css.display.flex, css.color.blue);
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            _inject2({
              ltr: ".xju2f9n{color:blue}",
              priority: 3000
            });
            const Component = () => ({
              className: "x78zum5 xju2f9n"
            });"
          `);
        });

        test('multiple stylex.props calls with atoms in same function', () => {
          const output = transform(`
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            function Component(isHeader) {
              const headerStyles = stylex.props(css.fontSize['24px'], css.fontWeight.bold);
              const bodyStyles = stylex.props(css.fontSize['16px'], css.color.black);
              return [headerStyles, bodyStyles];
            }
          `);
          expect(output).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x1pvqxga{font-size:24px}",
              priority: 3000
            });
            _inject2({
              ltr: ".x117nqv4{font-weight:bold}",
              priority: 3000
            });
            _inject2({
              ltr: ".x1j61zf2{font-size:16px}",
              priority: 3000
            });
            _inject2({
              ltr: ".x1mqxbix{color:black}",
              priority: 3000
            });
            function Component(isHeader) {
              const headerStyles = {
                className: "x1pvqxga x117nqv4"
              };
              const bodyStyles = {
                className: "x1j61zf2 x1mqxbix"
              };
              return [headerStyles, bodyStyles];
            }"
          `);
        });
      });

      describe('with options', () => {
        test('dev classnames and debug data for atoms', () => {
          const inline = transform(
            `
              import stylex from 'stylex';
              import css from '@stylexjs/atoms';
              stylex.props(css.display.flex);
            `,
            {
              dev: true,
              debug: true,
              enableDevClassNames: true,
              filename: '/tmp/Foo.js',
            },
          );
          expect(inline).toMatchInlineSnapshot(`
            "import _inject from "@stylexjs/stylex/lib/stylex-inject";
            var _inject2 = _inject;
            import stylex from 'stylex';
            import css from '@stylexjs/atoms';
            _inject2({
              ltr: ".x78zum5{display:flex}",
              priority: 3000
            });
            ({
              className: "Foo____inline__ x78zum5"
            });"
          `);
        });
      });
    });

    test('stylex call with number', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            0: {
              color: 'red',
            },
            1: {
              backgroundColor: 'blue',
            }
          });
          stylex.props([styles[0], styles[1]]);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        ({
          className: "x1e2nbdu x1t391ir"
        });"
      `);
    });

    test('stylex call with computed number', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            [0]: {
              color: 'red',
            },
            [1]: {
              backgroundColor: 'blue',
            }
          });
          stylex.props([styles[0], styles[1]]);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        ({
          className: "x1e2nbdu x1t391ir"
        });"
      `);
    });

    test('stylex call with computed string', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            'default': {
              color: 'red',
            }
          });
          stylex.props(styles['default']);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        ({
          className: "x1e2nbdu"
        });"
      `);
    });

    test('stylex call with multiple namespaces', () => {
      expect(
        transform(`
          import {create, props} from 'stylex';
          const styles = create({
            default: {
              color: 'red',
            },
          });
          const otherStyles = create({
            default: {
              backgroundColor: 'blue',
            }
          });
          props([styles.default, otherStyles.default]);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import { create, props } from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        ({
          className: "x1e2nbdu x1t391ir"
        });"
      `);
    });

    test('stylex call within variable declarations', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            foo: { color: 'red' }
          });
          const a = function() {
            return stylex.props(styles.foo);
          }
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        const a = function () {
          return {
            className: "x1e2nbdu"
          };
        };"
      `);
    });

    test('stylex call with styles variable assignment', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            foo: {
              color: 'red',
            },
            bar: {
              backgroundColor: 'blue',
            }
          });
          stylex.props([styles.foo, styles.bar]);
          const foo = styles;
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        const styles = {
          foo: {
            kMwMTN: "x1e2nbdu",
            $$css: true
          },
          bar: {
            kWkggS: "x1t391ir",
            $$css: true
          }
        };
        ({
          className: "x1e2nbdu x1t391ir"
        });
        const foo = styles;"
      `);
    });

    test('stylex call within export declarations', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            foo: { color: 'red' }
          });
          export default function MyExportDefault() {
            return stylex.props(styles.foo);
          }
          export function MyExport() {
            return stylex.props(styles.foo);
          }
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        export default function MyExportDefault() {
          return {
            className: "x1e2nbdu"
          };
        }
        export function MyExport() {
          return {
            className: "x1e2nbdu"
          };
        }"
      `);
    });

    test('stylex call with short-form properties', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            foo: {
              padding: 5
            }
          });
          stylex.props(styles.foo);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x14odnwx{padding:5px}",
          priority: 1000
        });
        ({
          className: "x14odnwx"
        });"
      `);
    });

    test('stylex call with exported short-form properties', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          export const styles = stylex.create({
            foo: {
              padding: 5
            }
          });
          stylex.props([styles.foo]);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x14odnwx{padding:5px}",
          priority: 1000
        });
        export const styles = {
          foo: {
            kmVPX3: "x14odnwx",
            $$css: true
          }
        };
        ({
          className: "x14odnwx"
        });"
      `);
    });

    test('Last property wins, even if shorthand', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const borderRadius = 2;
          export const styles = stylex.create({
            default: {
              marginBottom: 15,
              marginInlineEnd: 10,
              marginInlineStart: 20,
              marginTop: 5,
            },
            override: {
              margin: 0,
              marginBottom: 100,
            }
          });
          const result = stylex.props(styles.default, styles.override);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        const borderRadius = 2;
        _inject2({
          ltr: ".x1fqp7bg{margin-bottom:15px}",
          priority: 4000
        });
        _inject2({
          ltr: ".x1sa5p1d{margin-inline-end:10px}",
          priority: 3000
        });
        _inject2({
          ltr: ".xqsn43r{margin-inline-start:20px}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1ok221b{margin-top:5px}",
          priority: 4000
        });
        _inject2({
          ltr: ".x1ghz6dp{margin:0}",
          priority: 1000
        });
        _inject2({
          ltr: ".xiv7p99{margin-bottom:100px}",
          priority: 4000
        });
        export const styles = {
          default: {
            k1K539: "x1fqp7bg",
            k71WvV: "x1sa5p1d",
            keTefX: "xqsn43r",
            keoZOQ: "x1ok221b",
            $$css: true
          },
          override: {
            kogj98: "x1ghz6dp",
            k1K539: "xiv7p99",
            $$css: true
          }
        };
        const result = {
          className: "x1sa5p1d xqsn43r x1ok221b x1ghz6dp xiv7p99"
        };"
      `);
    });

    test('stylex call using styles with pseudo selectors', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            default: {
              color: 'red',
              ':hover': {
                color: 'blue',
              }
            }
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x17z2mba:hover{color:blue}",
          priority: 3130
        });
        ({
          className: "x1e2nbdu x17z2mba"
        });"
      `);
    });

    test('stylex call using styles with pseudo selectors within property', () => {
      expect(
        transform(`
          import * as stylex from 'stylex';
          const styles = stylex.create({
            default: {
              color: {
                default: 'red',
                ':hover': 'blue',
              }
            }
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x17z2mba:hover{color:blue}",
          priority: 3130
        });
        ({
          className: "x1e2nbdu x17z2mba"
        });"
      `);
    });

    test('stylex call using styles with Media Queries', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            default: {
              backgroundColor: 'red',
              '@media (min-width: 1000px)': {
                backgroundColor: 'blue',
              },
              '@media (min-width: 2000px)': {
                backgroundColor: 'purple',
              },
            },
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".xrkmrrc{background-color:red}",
          priority: 3000
        });
        _inject2({
          ltr: "@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",
          priority: 3200
        });
        _inject2({
          ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
          priority: 3200
        });
        ({
          className: "xrkmrrc xc445zv x1ssfqz5"
        });"
      `);
    });

    test('stylex call using styles with Media Queries within property', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            default: {
              backgroundColor: {
                default:'red',
                '@media (min-width: 1000px)': 'blue',
                '@media (min-width: 2000px)': 'purple',
              },
            },
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".xrkmrrc{background-color:red}",
          priority: 3000
        });
        _inject2({
          ltr: "@media (min-width: 1000px) and (max-width: 1999.98px){.xr725a3.xr725a3{background-color:blue}}",
          priority: 3200
        });
        _inject2({
          ltr: "@media (min-width: 2000px){.x1ssfqz5.x1ssfqz5{background-color:purple}}",
          priority: 3200
        });
        ({
          className: "xrkmrrc xr725a3 x1ssfqz5"
        });"
      `);
    });

    test('stylex call using styles with Support Queries', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            default: {
              backgroundColor: 'red',
              '@supports (hover: hover)': {
                backgroundColor: 'blue',
              },
              '@supports not (hover: hover)': {
                backgroundColor: 'purple',
              },
            },
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".xrkmrrc{background-color:red}",
          priority: 3000
        });
        _inject2({
          ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
          priority: 3030
        });
        _inject2({
          ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
          priority: 3030
        });
        ({
          className: "xrkmrrc x6m3b6q x6um648"
        });"
      `);
    });

    test('stylex call using styles with Support Queries within property', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            default: {
              backgroundColor: {
                default:'red',
                '@supports (hover: hover)': 'blue',
                '@supports not (hover: hover)': 'purple',
              },
            },
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".xrkmrrc{background-color:red}",
          priority: 3000
        });
        _inject2({
          ltr: "@supports (hover: hover){.x6m3b6q.x6m3b6q{background-color:blue}}",
          priority: 3030
        });
        _inject2({
          ltr: "@supports not (hover: hover){.x6um648.x6um648{background-color:purple}}",
          priority: 3030
        });
        ({
          className: "xrkmrrc x6m3b6q x6um648"
        });"
      `);
    });

    describe('with contextual styles and collisions', () => {
      test('stylex call with conditions', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                backgroundColor: 'red',
              },
              active: {
                color: 'blue',
              }
            });
            stylex.props([styles.default, isActive && styles.active]);
          `,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".xrkmrrc{background-color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          ({
            0: {
              className: "xrkmrrc"
            },
            1: {
              className: "xrkmrrc xju2f9n"
            }
          })[!!isActive << 0];"
        `);
      });

      test('stylex call with conditions - skip conditional', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              default: {
                backgroundColor: 'red',
              },
              active: {
                color: 'blue',
              }
            });
            stylex.props([styles.default, isActive && styles.active]);
            `,
            { enableInlinedConditionalMerge: false },
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".xrkmrrc{background-color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          const styles = {
            default: {
              kWkggS: "xrkmrrc",
              $$css: true
            },
            active: {
              kMwMTN: "xju2f9n",
              $$css: true
            }
          };
          stylex.props([styles.default, isActive && styles.active]);"
        `);
      });

      test('stylex call with property collisions', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              blue: {
                color: 'blue',
              }
            });
            stylex.props([styles.red, styles.blue]);
            stylex.props([styles.blue, styles.red]);
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          ({
            className: "xju2f9n"
          });
          ({
            className: "x1e2nbdu"
          });"
        `);
      });

      test('stylex call with reverting by null', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              revert: {
                color: null,
              }
            });
            stylex.props([styles.red, styles.revert]);
            stylex.props([styles.revert, styles.red]);
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          ({});
          ({
            className: "x1e2nbdu"
          });"
        `);
      });

      test('stylex call with short-form property collisions', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingEnd: 10,
              },

              bar: {
                padding: 2,
                paddingStart: 10,
              },
            });
            stylex.props([styles.foo, styles.bar]);
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x14odnwx{padding:5px}",
            priority: 1000
          });
          _inject2({
            ltr: ".x2vl965{padding-inline-end:10px}",
            priority: 3000
          });
          _inject2({
            ltr: ".x1i3ajwb{padding:2px}",
            priority: 1000
          });
          _inject2({
            ltr: ".xe2zdcy{padding-inline-start:10px}",
            priority: 3000
          });
          ({
            className: "x2vl965 x1i3ajwb xe2zdcy"
          });"
        `);
      });

      test('stylex call with short-form property collisions with null', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            const styles = stylex.create({
              foo: {
                padding: 5,
                paddingEnd: 10,
              },

              bar: {
                padding: 2,
                paddingStart: null,
              },
            });
            stylex.props([styles.foo, styles.bar]);
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x14odnwx{padding:5px}",
            priority: 1000
          });
          _inject2({
            ltr: ".x2vl965{padding-inline-end:10px}",
            priority: 3000
          });
          _inject2({
            ltr: ".x1i3ajwb{padding:2px}",
            priority: 1000
          });
          ({
            className: "x2vl965 x1i3ajwb"
          });"
        `);
      });

      test('stylex call with conditions and collisions', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              blue: {
                color: 'blue',
              }
            });
            stylex.props([styles.red, isActive && styles.blue]);
          `,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          ({
            0: {
              className: "x1e2nbdu"
            },
            1: {
              className: "xju2f9n"
            }
          })[!!isActive << 0];"
        `);
      });

      test('stylex call with conditions and collisions - skip conditional', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              blue: {
                color: 'blue',
              }
            });
            stylex.props([styles.red, isActive && styles.blue]);
            `,
            { enableInlinedConditionalMerge: false },
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          const styles = {
            red: {
              kMwMTN: "x1e2nbdu",
              $$css: true
            },
            blue: {
              kMwMTN: "xju2f9n",
              $$css: true
            }
          };
          stylex.props([styles.red, isActive && styles.blue]);"
        `);
      });

      test('stylex call with conditions and null collisions', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              blue: {
                color: null,
              }
            });
            stylex.props([styles.red, isActive && styles.blue]);
          `,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          ({
            0: {
              className: "x1e2nbdu"
            },
            1: {}
          })[!!isActive << 0];"
        `);
      });

      test('stylex call with conditions and null collisions - skip conditional', () => {
        expect(
          transform(
            `
            import stylex from 'stylex';
            const styles = stylex.create({
              red: {
                color: 'red',
              },
              blue: {
                color: null,
              }
            });
            stylex.props([styles.red, isActive && styles.blue]);
            `,
            { enableInlinedConditionalMerge: false },
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          const styles = {
            red: {
              kMwMTN: "x1e2nbdu",
              $$css: true
            },
            blue: {
              kMwMTN: null,
              $$css: true
            }
          };
          stylex.props([styles.red, isActive && styles.blue]);"
        `);
      });
    });

    // COMPOSITION
    describe('with plugin options', () => {
      test('dev:true and enableInlinedConditionalMerge:false', () => {
        const options = {
          filename: '/html/js/FooBar.react.js',
          dev: true,
          enableInlinedConditionalMerge: false,
          enableDevClassNames: false,
        };
        expect(
          transform(
            `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  color: 'red',
                },
              });
              stylex.props(styles.default);
            `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          ({
            className: "x1e2nbdu",
            "data-style-src": "html/js/FooBar.react.js:4"
          });"
        `);

        expect(
          transform(
            `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  color: 'red',
                },
              });
              const otherStyles = stylex.create({
                default: {
                  backgroundColor: 'blue',
                }
              });
              stylex.props([styles.default, isActive && otherStyles.default]);
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          const styles = {
            default: {
              "color-kMwMTN": "x1e2nbdu",
              $$css: "html/js/FooBar.react.js:4"
            }
          };
          _inject2({
            ltr: ".x1t391ir{background-color:blue}",
            priority: 3000
          });
          const otherStyles = {
            default: {
              "backgroundColor-kWkggS": "x1t391ir",
              $$css: "html/js/FooBar.react.js:9"
            }
          };
          stylex.props([styles.default, isActive && otherStyles.default]);"
        `);
      });

      test('dev:true', () => {
        const options = {
          filename: '/html/js/FooBar.react.js',
          dev: true,
          enableDevClassNames: false,
        };
        expect(
          transform(
            `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  color: 'red',
                },
              });
              const otherStyles = stylex.create({
                default: {
                  backgroundColor: 'blue',
                }
              });
              stylex.props([styles.default, isActive && otherStyles.default]);
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".x1t391ir{background-color:blue}",
            priority: 3000
          });
          ({
            0: {
              className: "x1e2nbdu",
              "data-style-src": "html/js/FooBar.react.js:4"
            },
            1: {
              className: "x1e2nbdu x1t391ir",
              "data-style-src": "html/js/FooBar.react.js:4; html/js/FooBar.react.js:9"
            }
          })[!!isActive << 0];"
        `);

        expect(
          transform(
            `
              import stylex from 'stylex';
              const styles = stylex.create({
                default: {
                  color: 'red',
                },
                active: {
                  color: 'blue',
                }
              });
              stylex.props([styles.default, isActive && styles.active]);
          `,
            options,
          ),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".xju2f9n{color:blue}",
            priority: 3000
          });
          ({
            0: {
              className: "x1e2nbdu",
              "data-style-src": "html/js/FooBar.react.js:4"
            },
            1: {
              className: "xju2f9n",
              "data-style-src": "html/js/FooBar.react.js:4; html/js/FooBar.react.js:7"
            }
          })[!!isActive << 0];"
        `);
      });
    });
  });

  describe('Keep stylex.create when needed', () => {
    test('stylex call with computed key access', () => {
      expect(
        transform(`
            import stylex from 'stylex';
            const styles = stylex.create({
              [0]: {
                color: 'red',
              },
              [1]: {
                backgroundColor: 'blue',
              }
            });
            stylex.props(styles[variant]);
          `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        const styles = {
          "0": {
            kMwMTN: "x1e2nbdu",
            $$css: true
          },
          "1": {
            kWkggS: "x1t391ir",
            $$css: true
          }
        };
        stylex.props(styles[variant]);"
      `);
    });

    test('stylex call with composition of external styles', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          const styles = stylex.create({
            default: {
              color: 'red',
            },
          });
          stylex.props([styles.default, props]);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        const styles = {
          default: {
            kMwMTN: "x1e2nbdu",
            $$css: true
          }
        };
        stylex.props([styles.default, props]);"
      `);
    });

    test('stylex call using exported styles with pseudo selectors, and queries', () => {
      expect(
        transform(`
          import stylex from 'stylex';
          export const styles = stylex.create({
            default: {
              ':hover': {
                color: 'blue',
              },
              '@media (min-width: 1000px)': {
                backgroundColor: 'blue',
              },
            }
          });
          stylex.props(styles.default);
        `),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'stylex';
        _inject2({
          ltr: ".x17z2mba:hover{color:blue}",
          priority: 3130
        });
        _inject2({
          ltr: "@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",
          priority: 3200
        });
        export const styles = {
          default: {
            kDPRdz: "x17z2mba",
            ksQ81T: "xc445zv",
            $$css: true
          }
        };
        ({
          className: "x17z2mba xc445zv"
        });"
      `);
    });

    describe('even when stylex calls come first', () => {
      test('stylex call with computed key access', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            stylex.props(styles[variant]);
            const styles = stylex.create({
              [0]: {
                color: 'red',
              },
              [1]: {
                backgroundColor: 'blue',
              }
            });
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          stylex.props(styles[variant]);
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".x1t391ir{background-color:blue}",
            priority: 3000
          });
          const styles = {
            "0": {
              kMwMTN: "x1e2nbdu",
              $$css: true
            },
            "1": {
              kWkggS: "x1t391ir",
              $$css: true
            }
          };"
        `);
      });

      test('stylex call with mixed access', () => {
        expect(
          transform(`
            import stylex from 'stylex';

            function MyComponent() {
              return (
                <>
                  <div {...stylex.props(styles.foo)} />
                  <div {...stylex.props(styles.bar)} />
                  <CustomComponent xstyle={styles.foo} />
                  <div {...stylex.props([styles.foo, styles.bar])} />
                </>
              );
            }

            const styles = stylex.create({
              foo: {
                color: 'red',
              },
              bar: {
                backgroundColor: 'blue',
              }
            });
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          function MyComponent() {
            return <>
                            <div className="x1e2nbdu" />
                            <div className="x1t391ir" />
                            <CustomComponent xstyle={styles.foo} />
                            <div className="x1e2nbdu x1t391ir" />
                          </>;
          }
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          _inject2({
            ltr: ".x1t391ir{background-color:blue}",
            priority: 3000
          });
          const styles = {
            foo: {
              kMwMTN: "x1e2nbdu",
              $$css: true
            }
          };"
        `);
      });

      test('stylex call with composition of external styles', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            stylex.props([styles.default, props]);
            const styles = stylex.create({
              default: {
                color: 'red',
              },
            });
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          stylex.props([styles.default, props]);
          _inject2({
            ltr: ".x1e2nbdu{color:red}",
            priority: 3000
          });
          const styles = {
            default: {
              kMwMTN: "x1e2nbdu",
              $$css: true
            }
          };"
        `);
      });

      test('stylex call using exported styles with pseudo selectors, and queries', () => {
        expect(
          transform(`
            import stylex from 'stylex';
            stylex.props(styles.default);
            export const styles = stylex.create({
              default: {
                ':hover': {
                  color: 'blue',
                },
                '@media (min-width: 1000px)': {
                  backgroundColor: 'blue',
                },
              }
            });
          `),
        ).toMatchInlineSnapshot(`
          "import _inject from "@stylexjs/stylex/lib/stylex-inject";
          var _inject2 = _inject;
          import stylex from 'stylex';
          ({
            className: "x17z2mba xc445zv"
          });
          _inject2({
            ltr: ".x17z2mba:hover{color:blue}",
            priority: 3130
          });
          _inject2({
            ltr: "@media (min-width: 1000px){.xc445zv.xc445zv{background-color:blue}}",
            priority: 3200
          });
          export const styles = {
            default: {
              kDPRdz: "x17z2mba",
              ksQ81T: "xc445zv",
              $$css: true
            }
          };"
        `);
      });
    });
  });

  describe('Setting custom import paths', () => {
    test('Basic stylex call', () => {
      expect(
        transform(
          `
          import stylex from 'custom-stylex-path';
          const styles = stylex.create({
            red: {
              color: 'red',
            }
          });
          stylex.props(styles.red);
        `,
          { importSources: ['custom-stylex-path'] },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import stylex from 'custom-stylex-path';
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        ({
          className: "x1e2nbdu"
        });"
      `);
    });
  });

  describe('Specific edge-case bugs', () => {
    test('Basic stylex call', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            sidebar: {
              boxSizing: 'border-box',
              gridArea: 'sidebar',
            },
            content: {
              gridArea: 'content',
            },
            root: {
              display: 'grid',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"content"',
            },
            withSidebar: {
              gridTemplateColumns: 'auto minmax(0, 1fr)',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"sidebar content"',
              '@media (max-width: 640px)': {
                gridTemplateRows: 'minmax(0, 1fr) auto',
                gridTemplateAreas: '"content" "sidebar"',
                gridTemplateColumns: '100%',
              },
            },
            noSidebar: {
              gridTemplateColumns: 'minmax(0, 1fr)',
            },
          });
          stylex.props([
            styles.root,
            sidebar == null ? styles.noSidebar : styles.withSidebar,
          ]);
        `,
          {
            dev: true,
            enableDevClassNames: false,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x9f619{box-sizing:border-box}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1yc5d2u{grid-area:sidebar}",
          priority: 1000
        });
        _inject2({
          ltr: ".x1fdo2jl{grid-area:content}",
          priority: 1000
        });
        _inject2({
          ltr: ".xrvj5dj{display:grid}",
          priority: 3000
        });
        _inject2({
          ltr: ".x7k18q3{grid-template-rows:100%}",
          priority: 3000
        });
        _inject2({
          ltr: ".x5gp9wm{grid-template-areas:\\"content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: ".x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
          priority: 3000
        });
        _inject2({
          ltr: ".x17lh93j{grid-template-areas:\\"sidebar content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xmr4b4k.xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
          priority: 3200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xesbpuc.xesbpuc{grid-template-areas:\\"content\\" \\"sidebar\\"}}",
          priority: 2200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.x15nfgh4.x15nfgh4{grid-template-columns:100%}}",
          priority: 3200
        });
        _inject2({
          ltr: ".x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
          priority: 3000
        });
        export const styles = {
          sidebar: {
            "boxSizing-kB7OPa": "x9f619",
            "gridArea-kJuA4N": "x1yc5d2u",
            $$css: "@stylexjs/babel-plugin::4"
          },
          content: {
            "gridArea-kJuA4N": "x1fdo2jl",
            $$css: "@stylexjs/babel-plugin::8"
          },
          root: {
            "display-k1xSpc": "xrvj5dj",
            "gridTemplateRows-k9llMU": "x7k18q3",
            "gridTemplateAreas-kC13JO": "x5gp9wm",
            $$css: "@stylexjs/babel-plugin::11"
          },
          withSidebar: {
            "gridTemplateColumns-kumcoG": "x1rkzygb",
            "gridTemplateRows-k9llMU": "x7k18q3",
            "gridTemplateAreas-kC13JO": "x17lh93j",
            "@media (max-width: 640px)_gridTemplateRows-k9pwkU": "xmr4b4k",
            "@media (max-width: 640px)_gridTemplateAreas-kOnEH4": "xesbpuc",
            "@media (max-width: 640px)_gridTemplateColumns-k1JLwA": "x15nfgh4",
            $$css: "@stylexjs/babel-plugin::16"
          },
          noSidebar: {
            "gridTemplateColumns-kumcoG": "x1mkdm3x",
            $$css: "@stylexjs/babel-plugin::26"
          }
        };
        ({
          0: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4",
            "data-style-src": "@stylexjs/babel-plugin::11; @stylexjs/babel-plugin::16"
          },
          1: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x",
            "data-style-src": "@stylexjs/babel-plugin::11; @stylexjs/babel-plugin::26"
          }
        })[!!(sidebar == null) << 0];"
      `);
    });

    test('Basic stylex call', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            sidebar: {
              boxSizing: 'border-box',
              gridArea: 'sidebar',
            },
            content: {
              gridArea: 'content',
            },
            root: {
              display: 'grid',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"content"',
            },
            withSidebar: {
              gridTemplateColumns: 'auto minmax(0, 1fr)',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"sidebar content"',
              '@media (max-width: 640px)': {
                gridTemplateRows: 'minmax(0, 1fr) auto',
                gridTemplateAreas: '"content" "sidebar"',
                gridTemplateColumns: '100%',
              },
            },
            noSidebar: {
              gridTemplateColumns: 'minmax(0, 1fr)',
            },
          });
          const complex = stylex.props([
            styles.root,
            sidebar == null && !isSidebar ? styles.noSidebar : styles.withSidebar,
            isSidebar && styles.sidebar,
            isContent && styles.content,
          ]);
        `,
          {
            filename: '/html/js/FooBar.react.js',
            dev: true,
            enableDevClassNames: false,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x9f619{box-sizing:border-box}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1yc5d2u{grid-area:sidebar}",
          priority: 1000
        });
        _inject2({
          ltr: ".x1fdo2jl{grid-area:content}",
          priority: 1000
        });
        _inject2({
          ltr: ".xrvj5dj{display:grid}",
          priority: 3000
        });
        _inject2({
          ltr: ".x7k18q3{grid-template-rows:100%}",
          priority: 3000
        });
        _inject2({
          ltr: ".x5gp9wm{grid-template-areas:\\"content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: ".x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
          priority: 3000
        });
        _inject2({
          ltr: ".x17lh93j{grid-template-areas:\\"sidebar content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xmr4b4k.xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
          priority: 3200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xesbpuc.xesbpuc{grid-template-areas:\\"content\\" \\"sidebar\\"}}",
          priority: 2200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.x15nfgh4.x15nfgh4{grid-template-columns:100%}}",
          priority: 3200
        });
        _inject2({
          ltr: ".x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
          priority: 3000
        });
        const complex = {
          0: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16"
          },
          4: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26"
          },
          2: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x9f619 x1yc5d2u",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:4"
          },
          6: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x9f619 x1yc5d2u",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:4"
          },
          1: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:8"
          },
          5: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:8"
          },
          3: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x9f619 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:4; html/js/FooBar.react.js:8"
          },
          7: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x9f619 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:4; html/js/FooBar.react.js:8"
          }
        }[!!(sidebar == null && !isSidebar) << 2 | !!isSidebar << 1 | !!isContent << 0];"
      `);
    });

    test('Stylex call with debug on', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            sidebar: {
              boxSizing: 'border-box',
              gridArea: 'sidebar',
            },
            content: {
              gridArea: 'content',
            },
            root: {
              display: 'grid',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"content"',
            },
            withSidebar: {
              gridTemplateColumns: 'auto minmax(0, 1fr)',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"sidebar content"',
              '@media (max-width: 640px)': {
                gridTemplateRows: 'minmax(0, 1fr) auto',
                gridTemplateAreas: '"content" "sidebar"',
                gridTemplateColumns: '100%',
              },
            },
            noSidebar: {
              gridTemplateColumns: 'minmax(0, 1fr)',
            },
          });
          const complex = stylex.props([
            styles.root,
            sidebar == null && !isSidebar ? styles.noSidebar : styles.withSidebar,
            isSidebar && styles.sidebar,
            isContent && styles.content,
          ]);
        `,
          {
            filename: '/html/js/FooBar.react.js',
            dev: true,
            debug: true,
            enableDevClassNames: false,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x9f619{box-sizing:border-box}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1yc5d2u{grid-area:sidebar}",
          priority: 1000
        });
        _inject2({
          ltr: ".x1fdo2jl{grid-area:content}",
          priority: 1000
        });
        _inject2({
          ltr: ".xrvj5dj{display:grid}",
          priority: 3000
        });
        _inject2({
          ltr: ".x7k18q3{grid-template-rows:100%}",
          priority: 3000
        });
        _inject2({
          ltr: ".x5gp9wm{grid-template-areas:\\"content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: ".x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
          priority: 3000
        });
        _inject2({
          ltr: ".x17lh93j{grid-template-areas:\\"sidebar content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xmr4b4k.xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
          priority: 3200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xesbpuc.xesbpuc{grid-template-areas:\\"content\\" \\"sidebar\\"}}",
          priority: 2200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.x15nfgh4.x15nfgh4{grid-template-columns:100%}}",
          priority: 3200
        });
        _inject2({
          ltr: ".x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
          priority: 3000
        });
        const complex = {
          0: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16"
          },
          4: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26"
          },
          2: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x9f619 x1yc5d2u",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:4"
          },
          6: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x9f619 x1yc5d2u",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:4"
          },
          1: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:8"
          },
          5: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:8"
          },
          3: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x9f619 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:4; html/js/FooBar.react.js:8"
          },
          7: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x9f619 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:4; html/js/FooBar.react.js:8"
          }
        }[!!(sidebar == null && !isSidebar) << 2 | !!isSidebar << 1 | !!isContent << 0];"
      `);
    });

    test('Stylex call with debug on uses hashed classnames', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            sidebar: {
              boxSizing: 'border-box',
              gridArea: 'sidebar',
            },
            content: {
              gridArea: 'content',
            },
            root: {
              display: 'grid',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"content"',
            },
            withSidebar: {
              gridTemplateColumns: 'auto minmax(0, 1fr)',
              gridTemplateRows: '100%',
              gridTemplateAreas: '"sidebar content"',
              '@media (max-width: 640px)': {
                gridTemplateRows: 'minmax(0, 1fr) auto',
                gridTemplateAreas: '"content" "sidebar"',
                gridTemplateColumns: '100%',
              },
            },
            noSidebar: {
              gridTemplateColumns: 'minmax(0, 1fr)',
            },
          });
          const complex = stylex.props([
            styles.root,
            sidebar == null && !isSidebar ? styles.noSidebar : styles.withSidebar,
            isSidebar && styles.sidebar,
            isContent && styles.content,
          ]);
        `,
          {
            filename: '/html/js/FooBar.react.js',
            dev: true,
            debug: true,
            enableDevClassNames: false,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x9f619{box-sizing:border-box}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1yc5d2u{grid-area:sidebar}",
          priority: 1000
        });
        _inject2({
          ltr: ".x1fdo2jl{grid-area:content}",
          priority: 1000
        });
        _inject2({
          ltr: ".xrvj5dj{display:grid}",
          priority: 3000
        });
        _inject2({
          ltr: ".x7k18q3{grid-template-rows:100%}",
          priority: 3000
        });
        _inject2({
          ltr: ".x5gp9wm{grid-template-areas:\\"content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: ".x1rkzygb{grid-template-columns:auto minmax(0,1fr)}",
          priority: 3000
        });
        _inject2({
          ltr: ".x17lh93j{grid-template-areas:\\"sidebar content\\"}",
          priority: 2000
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xmr4b4k.xmr4b4k{grid-template-rows:minmax(0,1fr) auto}}",
          priority: 3200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.xesbpuc.xesbpuc{grid-template-areas:\\"content\\" \\"sidebar\\"}}",
          priority: 2200
        });
        _inject2({
          ltr: "@media (max-width: 640px){.x15nfgh4.x15nfgh4{grid-template-columns:100%}}",
          priority: 3200
        });
        _inject2({
          ltr: ".x1mkdm3x{grid-template-columns:minmax(0,1fr)}",
          priority: 3000
        });
        const complex = {
          0: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16"
          },
          4: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26"
          },
          2: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x9f619 x1yc5d2u",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:4"
          },
          6: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x9f619 x1yc5d2u",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:4"
          },
          1: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:8"
          },
          5: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:8"
          },
          3: {
            className: "xrvj5dj x1rkzygb x7k18q3 x17lh93j xmr4b4k xesbpuc x15nfgh4 x9f619 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:16; html/js/FooBar.react.js:4; html/js/FooBar.react.js:8"
          },
          7: {
            className: "xrvj5dj x7k18q3 x5gp9wm x1mkdm3x x9f619 x1fdo2jl",
            "data-style-src": "html/js/FooBar.react.js:11; html/js/FooBar.react.js:26; html/js/FooBar.react.js:4; html/js/FooBar.react.js:8"
          }
        }[!!(sidebar == null && !isSidebar) << 2 | !!isSidebar << 1 | !!isContent << 0];"
      `);
    });

    test('hoisting correctly with duplicate names', () => {
      expect(
        transform(
          `
            import * as stylex from "@stylexjs/stylex";
            import * as React from "react";

            function Foo() {
              const styles = stylex.create({
                div: { color: "red" },
              });
              return <div {...stylex.props(styles.div)}>Hello, foo!</div>;
            }

            function Bar() {
              const styles = stylex.create({
                div: { color: "blue" },
              });
              return <div {...stylex.props(styles.div)}>Hello, bar!</div>;
            }

            export function App() {
              return (
                <>
                  <Foo />
                  <Bar />
                </>
              );
            }
          `,
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from "@stylexjs/stylex";
        import * as React from "react";
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        const _styles = {
          div: {
            kMwMTN: "x1e2nbdu",
            $$css: true
          }
        };
        function Foo() {
          const styles = _styles;
          return <div {...stylex.props(styles.div)}>Hello, foo!</div>;
        }
        _inject2({
          ltr: ".xju2f9n{color:blue}",
          priority: 3000
        });
        const _styles2 = {
          div: {
            kMwMTN: "xju2f9n",
            $$css: true
          }
        };
        function Bar() {
          const styles = _styles2;
          return <div {...stylex.props(styles.div)}>Hello, bar!</div>;
        }
        export function App() {
          return <>
                          <Foo />
                          <Bar />
                        </>;
        }"
      `);
    });

    test('does not hoist $$css objects that reference local variables', () => {
      expect(
        transform(
          `
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: { display: 'flex' },
            });
            function Foo({className}) {
              return <div {...stylex.props(styles.root, className && {$$css: true, __: className})}>Hello, foo!</div>;
            }
          `,
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x78zum5{display:flex}",
          priority: 3000
        });
        const styles = {
          root: {
            k1xSpc: "x78zum5",
            $$css: true
          }
        };
        function Foo({
          className
        }) {
          return <div {...stylex.props(styles.root, className && {
            $$css: true,
            __: className
          })}>Hello, foo!</div>;
        }"
      `);
    });
  });

  describe('dealing with imports', () => {
    test('all local styles', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            default: {
              color: 'black',
            },
            red: {
              color: 'red',
            },
            blueBg: {
              backgroundColor: 'blue',
            },

          });

          <div {...stylex.props(styles.default, styles.red, styles.blueBg)} />
        `,
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x1mqxbix{color:black}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        <div className="x1e2nbdu x1t391ir" />;"
      `);
    });
    test('local array styles', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            default: {
              color: 'black',
            },
            red: {
              color: 'red',
            },
            blueBg: {
              backgroundColor: 'blue',
            },
          });

          const base = [styles.default, styles.red];

          <div {...stylex.props(base, styles.blueBg)} />
        `,
          {
            enableMinifiedKeys: false,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        _inject2({
          ltr: ".x1mqxbix{color:black}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1e2nbdu{color:red}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        const styles = {
          default: {
            color: "x1mqxbix",
            $$css: true
          },
          red: {
            color: "x1e2nbdu",
            $$css: true
          }
        };
        const base = [styles.default, styles.red];
        <div className="x1e2nbdu x1t391ir" />;"
      `);
    });
    test('regular style import', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          import {someStyle} from './otherFile';
          const styles = stylex.create({
            default: {
              color: 'black',
            },
          });
          <div {...stylex.props(styles.default, someStyle)} />
        `,
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        import { someStyle } from './otherFile';
        _inject2({
          ltr: ".x1mqxbix{color:black}",
          priority: 3000
        });
        const styles = {
          default: {
            kMwMTN: "x1mqxbix",
            $$css: true
          }
        };
        <div {...stylex.props(styles.default, someStyle)} />;"
      `);
    });
    test('default import from .stylex.js file', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          import {someStyle, vars} from './__fixtures__/constants.stylex.js';
          const styles = stylex.create({
            default: {
              color: 'black',
              backgroundColor: vars.foo,
            },
          });
          <div {...stylex.props(styles.default, someStyle)} />
        `,
          {
            filename: THIS_FILE,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        import { someStyle, vars } from './__fixtures__/constants.stylex.js';
        _inject2({
          ltr: ".x1mqxbix{color:black}",
          priority: 3000
        });
        _inject2({
          ltr: ".x1ptj8da{background-color:var(--xu6xsfm)}",
          priority: 3000
        });
        const styles = {
          default: {
            kMwMTN: "x1mqxbix",
            kWkggS: "x1ptj8da",
            $$css: true
          }
        };
        <div {...stylex.props(styles.default, someStyle)} />;"
      `);
    });
    test('object import from .stylex.js file', () => {
      expect(
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          import {someStyle} from './__fixtures__/constants.stylex.js';
          const styles = stylex.create({
            default: {
              color: 'black',
              backgroundColor: someStyle.foo,
            },
          });
          <div {...stylex.props(styles.default, someStyle.foo)} />
        `,
          {
            filename: THIS_FILE,
          },
        ),
      ).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        import { someStyle } from './__fixtures__/constants.stylex.js';
        _inject2({
          ltr: ".x1mqxbix{color:black}",
          priority: 3000
        });
        _inject2({
          ltr: ".xxtkuhj{background-color:var(--x18h8e3f)}",
          priority: 3000
        });
        const styles = {
          default: {
            kMwMTN: "x1mqxbix",
            kWkggS: "xxtkuhj",
            $$css: true
          }
        };
        <div {...stylex.props(styles.default, someStyle.foo)} />;"
      `);
    });
  });
});
