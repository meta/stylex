/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.autoMockOff();

import { transformSync } from '@babel/core';
import stylexPlugin from '../src/index';

function transform(source, opts = {}) {
  const result = transformSync(source, {
    filename: opts.filename ?? 'test.js',
    parserOpts: {
      flow: 'all',
    },
    babelrc: false,
    plugins: [
      [
        stylexPlugin,
        {
          treeshakeCompensation: true,
          unstable_moduleResolution: { type: 'haste' },
          ...opts,
        },
      ],
    ],
  });

  return result;
}

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] when functions', () => {
    test('when.ancestor function', () => {
      const { code, metadata } = transform(`
        import { when, create } from '@stylexjs/stylex';
        
        const styles = create({
            container: {
              backgroundColor: {
                default: 'blue',
                [when.ancestor(':hover')]: 'red',
              },
            },
        });

        console.log(styles.container);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import { when, create } from '@stylexjs/stylex';
        const styles = {
          container: {
            kWkggS: "x1t391ir x148kuu",
            $$css: true
          }
        };
        console.log(styles.container);"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1t391ir",
            {
              "ltr": ".x1t391ir{background-color:blue}",
              "rtl": null,
            },
            3000,
          ],
          [
            "x148kuu",
            {
              "ltr": ".x148kuu.x148kuu:where(.x-default-marker:hover .x148kuu){background-color:red}",
              "rtl": null,
            },
            3011.3,
          ],
        ]
      `);
    });

    test('when.siblingBefore function', () => {
      const { code, metadata } = transform(`
        import { when, create } from '@stylexjs/stylex';
        
        const styles = create({
          container: {
            backgroundColor: {
              default: 'blue',
              [when.siblingBefore(':focus')]: 'red',
            },
          },
        });

        console.log(styles.container);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import { when, create } from '@stylexjs/stylex';
        const styles = {
          container: {
            kWkggS: "x1t391ir x1i6rnlt",
            $$css: true
          }
        };
        console.log(styles.container);"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1t391ir",
            {
              "ltr": ".x1t391ir{background-color:blue}",
              "rtl": null,
            },
            3000,
          ],
          [
            "x1i6rnlt",
            {
              "ltr": ".x1i6rnlt.x1i6rnlt:where(.x-default-marker:focus ~ .x1i6rnlt){background-color:red}",
              "rtl": null,
            },
            3031.5,
          ],
        ]
      `);
    });

    test('namespace imports', () => {
      const { code, metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        
        const styles = stylex.create({
          container: {
            backgroundColor: {
              default: 'blue',
              [stylex.when.ancestor(':hover')]: 'red',
              [stylex.when.siblingBefore(':focus')]: 'green',
              [stylex.when.anySibling(':active')]: 'yellow',
              [stylex.when.siblingAfter(':focus')]: 'purple',
              [stylex.when.descendant(':focus')]: 'orange',
            },
          },
        });

        console.log(styles.container);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const styles = {
          container: {
            kWkggS: "x1t391ir x148kuu xpijypl xoev4mv x1v1vkh3 x9zntq3",
            $$css: true
          }
        };
        console.log(styles.container);"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1t391ir",
            {
              "ltr": ".x1t391ir{background-color:blue}",
              "rtl": null,
            },
            3000,
          ],
          [
            "x148kuu",
            {
              "ltr": ".x148kuu.x148kuu:where(.x-default-marker:hover .x148kuu){background-color:red}",
              "rtl": null,
            },
            3011.3,
          ],
          [
            "xpijypl",
            {
              "ltr": ".xpijypl.xpijypl:where(.x-default-marker:focus ~ .xpijypl){background-color:green}",
              "rtl": null,
            },
            3031.5,
          ],
          [
            "xoev4mv",
            {
              "ltr": ".xoev4mv.xoev4mv:where(.x-default-marker:active ~ .xoev4mv, :has(~ .x-default-marker:active)){background-color:yellow}",
              "rtl": null,
            },
            3021.7,
          ],
          [
            "x1v1vkh3",
            {
              "ltr": ".x1v1vkh3.x1v1vkh3:where(:has(~ .x-default-marker:focus)){background-color:purple}",
              "rtl": null,
            },
            3041.5,
          ],
          [
            "x9zntq3",
            {
              "ltr": ".x9zntq3.x9zntq3:where(:has(.x-default-marker:focus)){background-color:orange}",
              "rtl": null,
            },
            3016.5,
          ],
        ]
      `);
    });

    test('aliased imports', () => {
      const { code, metadata } = transform(`
        import { when as w, create } from '@stylexjs/stylex';
        
        const styles = create({
          container: {
            backgroundColor: {
              default: 'blue',
              [w.ancestor(':hover')]: 'red',
              [w.siblingBefore(':focus')]: 'green',
            },
          },
        });

        console.log(styles.container);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import { when as w, create } from '@stylexjs/stylex';
        const styles = {
          container: {
            kWkggS: "x1t391ir x148kuu xpijypl",
            $$css: true
          }
        };
        console.log(styles.container);"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1t391ir",
            {
              "ltr": ".x1t391ir{background-color:blue}",
              "rtl": null,
            },
            3000,
          ],
          [
            "x148kuu",
            {
              "ltr": ".x148kuu.x148kuu:where(.x-default-marker:hover .x148kuu){background-color:red}",
              "rtl": null,
            },
            3011.3,
          ],
          [
            "xpijypl",
            {
              "ltr": ".xpijypl.xpijypl:where(.x-default-marker:focus ~ .xpijypl){background-color:green}",
              "rtl": null,
            },
            3031.5,
          ],
        ]
      `);
    });
  });

  describe('[validation] when functions', () => {
    test('validates pseudo selector format', () => {
      expect(() =>
        transform(`
          import { when, create } from '@stylexjs/stylex';
          
          const styles = create({
            container: {
              backgroundColor: {
                default: 'blue',
                [when.ancestor('hover')]: 'red',
              },
            },
          });
        `),
      ).toThrow('Pseudo selector must start with ":" or "["');
    });

    test('rejects pseudo-elements', () => {
      expect(() =>
        transform(`
          import { when, create } from '@stylexjs/stylex';

          const styles = create({
            container: {
              backgroundColor: {
                default: 'blue',
                [when.ancestor('::before')]: 'red',
              },
            },
          });
        `),
      ).toThrow('Pseudo selector cannot start with "::"');
    });

    test('validates attribute selector format', () => {
      expect(() =>
        transform(`
          import { when, create } from '@stylexjs/stylex';

          const styles = create({
            container: {
              backgroundColor: {
                default: 'blue',
                [when.ancestor('[data-state="open"')]: 'red',
              },
            },
          });
        `),
      ).toThrow('Attribute selector must end with "]"');
    });

    test('rejects invalid selector format', () => {
      expect(() =>
        transform(`
          import { when, create } from '@stylexjs/stylex';

          const styles = create({
            container: {
              backgroundColor: {
                default: 'blue',
                [when.ancestor('invalid')]: 'red',
              },
            },
          });
        `),
      ).toThrow('Pseudo selector must start with ":" or "["');
    });
  });

  describe('[transform] when functions with attribute selectors', () => {
    test('when.ancestor with attribute selector', () => {
      const { code, metadata } = transform(`
        import { when, create } from '@stylexjs/stylex';

        const styles = create({
          container: {
            backgroundColor: {
              default: 'blue',
              [when.ancestor('[data-panel-state="open"]')]: 'red',
            },
          },
        });

        console.log(styles.container);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import { when, create } from '@stylexjs/stylex';
        const styles = {
          container: {
            kWkggS: "x1t391ir x11omtej",
            $$css: true
          }
        };
        console.log(styles.container);"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1t391ir",
            {
              "ltr": ".x1t391ir{background-color:blue}",
              "rtl": null,
            },
            3000,
          ],
          [
            "x11omtej",
            {
              "ltr": ".x11omtej.x11omtej:where(.x-default-marker[data-panel-state="open"] .x11omtej){background-color:red}",
              "rtl": null,
            },
            3040,
          ],
        ]
      `);
    });

    test('when.descendant with attribute selector', () => {
      const { code, metadata } = transform(`
        import { when, create } from '@stylexjs/stylex';

        const styles = create({
          container: {
            backgroundColor: {
              default: 'blue',
              [when.descendant('[data-panel-state="open"]')]: 'green',
            },
          },
        });

        console.log(styles.container);
      `);

      expect(code).toMatchInlineSnapshot(`
        "import { when, create } from '@stylexjs/stylex';
        const styles = {
          container: {
            kWkggS: "x1t391ir x1doj7mj",
            $$css: true
          }
        };
        console.log(styles.container);"
      `);

      expect(metadata.stylex).toMatchInlineSnapshot(`
        [
          [
            "x1t391ir",
            {
              "ltr": ".x1t391ir{background-color:blue}",
              "rtl": null,
            },
            3000,
          ],
          [
            "x1doj7mj",
            {
              "ltr": ".x1doj7mj.x1doj7mj:where(:has(.x-default-marker[data-panel-state="open"])){background-color:green}",
              "rtl": null,
            },
            3040,
          ],
        ]
      `);
    });
  });
  describe('[transform] rule subject of relational selectors', () => {
    function rulesFor(styles) {
      const { metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        import { marker } from 'marker.stylex';
        const styles = stylex.create({ root: ${styles} });
      `);
      return metadata.stylex
        .map(([, { ltr, rtl }]) => (rtl != null ? [ltr, rtl] : [ltr]))
        .flat();
    }

    test('uses the class of the rule for each when function', () => {
      expect(
        rulesFor(`{
          color: {
            default: null,
            [stylex.when.ancestor(':hover')]: 'red',
            [stylex.when.descendant(':hover')]: 'red',
            [stylex.when.anySibling(':hover')]: 'red',
            [stylex.when.siblingBefore(':hover')]: 'red',
            [stylex.when.siblingAfter(':hover')]: 'red',
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".xq6e253.xq6e253:where(.x-default-marker:hover .xq6e253){color:red}",
          ".x1a02quk.x1a02quk:where(:has(.x-default-marker:hover)){color:red}",
          ".x1o23vnx.x1o23vnx:where(.x-default-marker:hover ~ .x1o23vnx, :has(~ .x-default-marker:hover)){color:red}",
          ".xbbeoq7.xbbeoq7:where(.x-default-marker:hover ~ .xbbeoq7){color:red}",
          ".x2lv340.x2lv340:where(:has(~ .x-default-marker:hover)){color:red}",
        ]
      `);
    });

    test.each([
      ':is(\n.a, .b)',
      ':is(\r.a, .b)',
      ':is(\r\n.a, .b)',
      ':has(\n> *)',
      ':has(\r> *)',
      ':has(\r\n> *)',
    ])('narrows the subject with a multiline pseudo %j', (pseudo) => {
      for (const kind of ['ancestor', 'siblingBefore', 'anySibling']) {
        const { metadata } = transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              color: {
                default: null,
                [stylex.when.${kind}(${JSON.stringify(pseudo)})]: 'red',
              },
            },
          });
        `);
        const [[className, { ltr }, priority]] = metadata.stylex;
        const marker = `.x-default-marker${pseudo}`;
        const condition =
          kind === 'ancestor'
            ? `:where(${marker} .${className})`
            : kind === 'siblingBefore'
              ? `:where(${marker} ~ .${className})`
              : `:where(${marker} ~ .${className}, :has(~ ${marker}))`;

        expect(ltr).toBe(`.${className}.${className}${condition}{color:red}`);
        expect(priority).toBe(3040);
      }
    });

    test('keeps a * inside the pseudo', () => {
      expect(
        rulesFor(`{
          color: {
            default: null,
            [stylex.when.ancestor(':has(> *)')]: 'red',
            [stylex.when.descendant(':has(> *)')]: 'red',
            [stylex.when.anySibling(':has(> *)')]: 'red',
            [stylex.when.siblingBefore(':has(> *)')]: 'red',
            [stylex.when.siblingAfter(':has(> *)')]: 'red',
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".x12usrbt.x12usrbt:where(.x-default-marker:has(> *) .x12usrbt){color:red}",
          ".xcsahcs.xcsahcs:where(:has(.x-default-marker:has(> *))){color:red}",
          ".xkzyb0y.xkzyb0y:where(.x-default-marker:has(> *) ~ .xkzyb0y, :has(~ .x-default-marker:has(> *))){color:red}",
          ".xq63kg.xq63kg:where(.x-default-marker:has(> *) ~ .xq63kg){color:red}",
          ".x3zjb07.x3zjb07:where(:has(~ .x-default-marker:has(> *))){color:red}",
        ]
      `);
    });

    test('handles pseudos that contain a relational selector', () => {
      expect(
        rulesFor(`{
          color: {
            default: null,
            [stylex.when.ancestor(':is(.a ~ *)')]: 'red',
            [stylex.when.anySibling(':is(.a ~ *, :has(~ .b))')]: 'red',
            [stylex.when.siblingBefore('[data-state=" *)"]')]: 'red',
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".x1y6kcpl.x1y6kcpl:where(.x-default-marker:is(.a ~ *) .x1y6kcpl){color:red}",
          ".xp47wy.xp47wy:where(.x-default-marker:is(.a ~ *, :has(~ .b)) ~ .xp47wy, :has(~ .x-default-marker:is(.a ~ *, :has(~ .b)))){color:red}",
          ".x1d40om3.x1d40om3:where(.x-default-marker[data-state=" *)"] ~ .x1d40om3){color:red}",
        ]
      `);
    });

    test('uses the class of the rule with a custom marker', () => {
      expect(
        rulesFor(`{
          color: {
            default: null,
            [stylex.when.ancestor(':hover', marker)]: 'red',
            [stylex.when.anySibling(':hover', marker)]: 'red',
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".x1amvw37.x1amvw37:where(.x147num5:hover .x1amvw37){color:red}",
          ".xr1k07e.xr1k07e:where(.x147num5:hover ~ .xr1k07e, :has(~ .x147num5:hover)){color:red}",
        ]
      `);
    });

    test('puts pseudo-elements after the relational selector', () => {
      expect(
        rulesFor(`{
          '::before': {
            color: {
              default: null,
              [stylex.when.ancestor(':hover')]: 'red',
            },
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".xpnb0uc.xpnb0uc:where(.x-default-marker:hover .xpnb0uc)::before{color:red}",
        ]
      `);
    });

    test('applies to every ::thumb variant', () => {
      expect(
        rulesFor(`{
          '::thumb': {
            color: {
              default: null,
              [stylex.when.siblingBefore(':hover')]: 'red',
            },
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".x54era8.x54era8:where(.x-default-marker:hover ~ .x54era8)::-webkit-slider-thumb, .x54era8.x54era8:where(.x-default-marker:hover ~ .x54era8)::-moz-range-thumb, .x54era8.x54era8:where(.x-default-marker:hover ~ .x54era8)::-ms-thumb{color:red}",
        ]
      `);
    });

    test('applies inside at-rules', () => {
      expect(
        rulesFor(`{
          color: {
            default: null,
            '@media (min-width: 100px)': {
              default: null,
              [stylex.when.anySibling(':hover')]: 'red',
            },
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          "@media (min-width: 100px){.x1p8m4z5.x1p8m4z5.x1p8m4z5:where(.x-default-marker:hover ~ .x1p8m4z5, :has(~ .x-default-marker:hover)){color:red}}",
        ]
      `);
    });

    test('applies next to other pseudos', () => {
      expect(
        rulesFor(`{
          color: {
            default: null,
            ':focus': {
              default: null,
              [stylex.when.ancestor(':hover')]: 'red',
            },
            [stylex.when.ancestor(':hover')]: {
              default: null,
              [stylex.when.siblingBefore(':active')]: 'blue',
            },
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".xcycu7v.xcycu7v:focus:where(.x-default-marker:hover .xcycu7v){color:red}",
          ".xjymhjz.xjymhjz:where(.x-default-marker:active ~ .xjymhjz):where(.x-default-marker:hover .xjymhjz){color:blue}",
        ]
      `);
    });

    test('applies to both ltr and rtl rules', () => {
      expect(
        rulesFor(`{
          marginInlineStart: {
            default: null,
            [stylex.when.ancestor(':hover')]: 10,
          },
          float: {
            default: null,
            [stylex.when.ancestor(':hover')]: 'start',
          },
        }`),
      ).toMatchInlineSnapshot(`
        [
          ".xms7qp0.xms7qp0:where(.x-default-marker:hover .xms7qp0){margin-inline-start:10px}",
          ".x1ss12aw.x1ss12aw:where(.x-default-marker:hover .x1ss12aw){float:left}",
          ".x1ss12aw.x1ss12aw:where(.x-default-marker:hover .x1ss12aw){float:right}",
        ]
      `);
    });
  });

  describe('[transform] using stylex.defaultMarker', () => {
    test('named import', () => {
      const { code } = transform(`
        import { defaultMarker, props } from '@stylexjs/stylex';
        
        const classNames = props(defaultMarker());
      `);

      expect(code).toMatchInlineSnapshot(`
        "import { defaultMarker, props } from '@stylexjs/stylex';
        const classNames = {
          className: "x-default-marker"
        };"
      `);
    });
    test('namespace import', () => {
      const { code } = transform(`
        import * as stylex from '@stylexjs/stylex';
        
        const classNames = stylex.props(stylex.defaultMarker());
      `);

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        const classNames = {
          className: "x-default-marker"
        };"
      `);
    });
  });

  describe('[transform] using custom markers', () => {
    test('named import of custom marker', () => {
      const { code } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        import {customMarker} from 'custom-marker.stylex';

        const styles = stylex.create({
          foo: {
            backgroundColor: {
              default: 'blue',
              [stylex.when.ancestor(':hover', customMarker)]: 'red',
            },
          },
        });
        
        const container = stylex.props(customMarker);
        const classNames = stylex.props(styles.foo);
      `,
        { runtimeInjection: true },
      );

      expect(code).toMatchInlineSnapshot(`
        "import _inject from "@stylexjs/stylex/lib/stylex-inject";
        var _inject2 = _inject;
        import * as stylex from '@stylexjs/stylex';
        import 'custom-marker.stylex';
        import { customMarker } from 'custom-marker.stylex';
        _inject2({
          ltr: ".x1t391ir{background-color:blue}",
          priority: 3000
        });
        _inject2({
          ltr: ".x7rpj1w.x7rpj1w:where(.x1lc2aw:hover .x7rpj1w){background-color:red}",
          priority: 3011.3
        });
        const container = stylex.props(customMarker);
        const classNames = {
          className: "x1t391ir x7rpj1w"
        };"
      `);
    });
  });
});
