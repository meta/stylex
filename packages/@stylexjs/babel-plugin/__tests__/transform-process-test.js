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
  const pluginOpts = {
    debug: true,
    styleResolution: 'property-specificity',
    unstable_moduleResolution: {
      rootDir: '/src/app/',
      type: 'commonJS',
    },
    ...opts,
  };

  const tokens = transformSync(
    `
    import * as stylex from '@stylexjs/stylex';
    export const constants = stylex.defineConsts({
      YELLOW: 'yellow',
      ORANGE: 'var(--orange-theme-color)',
      mediaBig: '@media (max-width: 1000px)',
      mediaSmall: '@media (max-width: 500px)'
    });
    export const vars = stylex.defineVars({
      blue: 'blue',
      marginTokens: {
        default: "10px",
        "@media (min-width: 600px)": "20px"
      },
      colorTokens: {
        default: 'red',
        '@media (prefers-color-scheme: dark)': {
          default: 'lightblue',
          '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
        }
      },
    });
    `,
    {
      filename: '/src/app/tokens.stylex.js',
      parserOpts: { flow: 'all' },
      babelrc: false,
      plugins: [[stylexPlugin, pluginOpts]],
    },
  );

  const otherTokens = transformSync(
    `
    import * as stylex from '@stylexjs/stylex';
    export const spacing = stylex.defineVars({
      small: '2px',
      medium: '4px',
      large: '8px'
    });
    `,
    {
      filename: '/src/app/otherTokens.stylex.js',
      parserOpts: { flow: 'all' },
      babelrc: false,
      plugins: [[stylexPlugin, pluginOpts]],
    },
  );

  const main = transformSync(
    `
  ${tokens.code}
  ${otherTokens.code.replace("import * as stylex from '@stylexjs/stylex';", '')}
  ${source.replace("import * as stylex from '@stylexjs/stylex';", '')}
  `,
    {
      filename: opts.filename ?? '/src/app/main.js',
      parserOpts: { flow: 'all' },
      babelrc: false,
      plugins: [[stylexPlugin, pluginOpts]],
    },
  );

  const metadata = [
    ...(tokens.metadata.stylex || []),
    ...(otherTokens.metadata.stylex || []),
    ...(main.metadata.stylex || []),
  ];

  return { code: main.code, metadata };
}

const fixture = `
import * as stylex from '@stylexjs/stylex';
export const themeColor = stylex.createTheme(vars, {
  blue: 'lightblue'
});
export const themeSpacing = stylex.createTheme(spacing, {
  small: '5px',
  medium: '10px',
  large: '20px'
});
export const styles = stylex.create({
  root: {
    animationName: stylex.keyframes({
      '0%': {
        boxShadow: '1px 2px 3px 4px red',
        color: constants.YELLOW
      },
      '100%': {
        boxShadow: '10px 20px 30px 40px green',
        color: constants.ORANGE
      }
    }),
    backgroundColor: {
      default: 'red',
      ':hover': 'blue',
      [stylex.when.ancestor(':focus')]: 'green',
      '@media (max-width: 1000px)': {
        default: 'yellow',
        [stylex.when.descendant(':focus')]: 'purple',
        [stylex.when.anySibling(':active')]: 'orange',
      }
    },
    margin: vars.marginTokens,
    borderColor: {
      default: 'green',
      [constants.mediaBig]: {
        default: vars.blue,
        [constants.mediaSmall]: 'yellow',
      }
    },
    outlineColor: vars.colorTokens,
    textShadow: {
      default: '1px 2px 3px 4px red',
      '@media (min-width:320px)': '10px 20px 30px 40px green'
    },
    padding: spacing.large,
    margin: '10px 20px',
    float: 'inline-start'
  },
  overrideColor: {
    [constants.ORANGE]: 'red'
  },
  dynamic: (color) => ({ color })
});
`;

describe('@stylexjs/babel-plugin', () => {
  describe('[transform] stylexPlugin.processStylexRules', () => {
    test('no rules', () => {
      const { code, metadata } = transform('');
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const constants = {
          YELLOW: "yellow",
          ORANGE: "var(--orange-theme-color)",
          mediaBig: "@media (max-width: 1000px)",
          mediaSmall: "@media (max-width: 500px)"
        };
        export const vars = {
          blue: "var(--xpqh4lw)",
          marginTokens: "var(--x8nt2k2)",
          colorTokens: "var(--xkxfyv)",
          __varGroupHash__: "xsg933n"
        };
        export const spacing = {
          small: "var(--x19twipt)",
          medium: "var(--xypjos2)",
          large: "var(--x1ec7iuc)",
          __varGroupHash__: "xbiwvf9"
        };"
      `);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        ":root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}"
      `);
    });

    test('all rules (useLayers:false)', () => {
      const { code, metadata } = transform(fixture, {
        filename: '/src/app/components/main.js',
      });
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const constants = {
          YELLOW: "yellow",
          ORANGE: "var(--orange-theme-color)",
          mediaBig: "@media (max-width: 1000px)",
          mediaSmall: "@media (max-width: 500px)"
        };
        export const vars = {
          blue: "var(--xpqh4lw)",
          marginTokens: "var(--x8nt2k2)",
          colorTokens: "var(--xkxfyv)",
          __varGroupHash__: "xsg933n"
        };
        export const spacing = {
          small: "var(--x19twipt)",
          medium: "var(--xypjos2)",
          large: "var(--x1ec7iuc)",
          __varGroupHash__: "xbiwvf9"
        };
        export const themeColor = {
          xsg933n: "x1coplze xsg933n",
          $$css: true
        };
        export const themeSpacing = {
          xbiwvf9: "x4hn0rr xbiwvf9",
          $$css: true
        };
        export const styles = {
          root: {
            "animationName-kKVMdj": "x13ah0pd",
            "backgroundColor-kWkggS": "xrkmrrc xbrh7vm xfy810d xahc4vn x1t4kl4c x975j7z",
            "margin-kogj98": "xymmreb",
            "borderColor-kVAM5u": "x1bg2uv5 xio2edn xqiy1ys",
            "outlineColor-kjBf7l": "x18abd1y",
            "textShadow-kKMj4B": "x1skrh0i xtj17id",
            "padding-kmVPX3": "x1s2izit",
            "float-kyUFMd": "x1kmio9f",
            $$css: "components/main.js:33"
          },
          overrideColor: {
            "--orange-theme-color": "xufgesz",
            $$css: "components/main.js:71"
          },
          dynamic: color => [color !== undefined && {
            "color-kMwMTN": color != null ? "x14rh7hd" : "",
            $$css: "components/main.js:74"
          }, {
            "--x-color": color != null ? color : undefined
          }]
        };"
      `);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "@property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        .xymmreb:not(#\\#){margin:10px 20px}
        .x1s2izit:not(#\\#){padding:var(--x1ec7iuc)}
        .x1bg2uv5:not(#\\#):not(#\\#){border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn:not(#\\#):not(#\\#){border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys:not(#\\#):not(#\\#){border-color:yellow}}}
        .x13ah0pd:not(#\\#):not(#\\#):not(#\\#){animation-name:x35atj5-B}
        .xrkmrrc:not(#\\#):not(#\\#):not(#\\#){background-color:red}
        .x14rh7hd:not(#\\#):not(#\\#):not(#\\#){color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f:not(#\\#):not(#\\#):not(#\\#){float:left}
        html[dir='rtl'] .x1kmio9f:not(#\\#):not(#\\#):not(#\\#){float:right}
        .x18abd1y:not(#\\#):not(#\\#):not(#\\#){outline-color:var(--xkxfyv)}
        .x1skrh0i:not(#\\#):not(#\\#):not(#\\#){text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *):not(#\\#):not(#\\#):not(#\\#){background-color:green}
        .xbrh7vm:hover:not(#\\#):not(#\\#):not(#\\#){background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn:not(#\\#):not(#\\#):not(#\\#){background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id:not(#\\#):not(#\\#):not(#\\#){text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)):not(#\\#):not(#\\#):not(#\\#){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)):not(#\\#):not(#\\#):not(#\\#){background-color:orange}}"
      `);
    });

    test('all rules (useLayers:true)', () => {
      const { code, metadata } = transform(fixture, {
        useLayers: true,
      });
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const constants = {
          YELLOW: "yellow",
          ORANGE: "var(--orange-theme-color)",
          mediaBig: "@media (max-width: 1000px)",
          mediaSmall: "@media (max-width: 500px)"
        };
        export const vars = {
          blue: "var(--xpqh4lw)",
          marginTokens: "var(--x8nt2k2)",
          colorTokens: "var(--xkxfyv)",
          __varGroupHash__: "xsg933n"
        };
        export const spacing = {
          small: "var(--x19twipt)",
          medium: "var(--xypjos2)",
          large: "var(--x1ec7iuc)",
          __varGroupHash__: "xbiwvf9"
        };
        export const themeColor = {
          xsg933n: "x1coplze xsg933n",
          $$css: true
        };
        export const themeSpacing = {
          xbiwvf9: "x4hn0rr xbiwvf9",
          $$css: true
        };
        export const styles = {
          root: {
            "animationName-kKVMdj": "x13ah0pd",
            "backgroundColor-kWkggS": "xrkmrrc xbrh7vm xfy810d xahc4vn x1t4kl4c x975j7z",
            "margin-kogj98": "xymmreb",
            "borderColor-kVAM5u": "x1bg2uv5 xio2edn xqiy1ys",
            "outlineColor-kjBf7l": "x18abd1y",
            "textShadow-kKMj4B": "x1skrh0i xtj17id",
            "padding-kmVPX3": "x1s2izit",
            "float-kyUFMd": "x1kmio9f",
            $$css: "main.js:33"
          },
          overrideColor: {
            "--orange-theme-color": "xufgesz",
            $$css: "main.js:71"
          },
          dynamic: color => [color !== undefined && {
            "color-kMwMTN": color != null ? "x14rh7hd" : "",
            $$css: "main.js:74"
          }, {
            "--x-color": color != null ? color : undefined
          }]
        };"
      `);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: true,
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer priority1, priority2, priority3, priority4;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    // Regression test for https://github.com/facebook/stylex/issues/1611
    // A `priorityLevel` (Math.floor(priority / 1000)) groups together CSS
    // custom properties (priority 1) with priority-0 at-rules such as
    // `@keyframes`. The layer-wrapping decision must not let the at-rule's
    // priority 0 pull the layerable custom property out of its `@layer`.
    test('custom properties stay inside @layer alongside priority-0 at-rules', () => {
      const { metadata } = transformSync(
        `
          import * as stylex from '@stylexjs/stylex';
          const fade = stylex.keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });
          const styles = stylex.create({
            card: {
              animationName: fade,
              '--card-padding': '16px',
              color: 'red',
            },
          });
        `,
        { babelrc: false, plugins: [[stylexPlugin, { dev: false }]] },
      );
      expect(
        stylexPlugin.processStylexRules(metadata.stylex, {
          useLayers: true,
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer priority1, priority2;
        @keyframes x18re5ia-B{from{opacity:0;}to{opacity:1;}}
        @layer priority1{
        .x1vtiyio{--card-padding:16px}
        }
        @layer priority2{
        .xqcmdr3{animation-name:x18re5ia-B}
        .x1e2nbdu{color:red}
        }"
      `);
    });

    test.each([
      '@keyframes fade{from{opacity:0}to{opacity:1}}',
      '@property --card-padding{syntax:"<length>";inherits:false;initial-value:0px}',
      '@position-try --fallback{top:0}',
    ])('keeps %s unlayered with and without custom properties', (atRule) => {
      const rules = [['atRule', { ltr: atRule, rtl: null }, 0]];
      expect(stylexPlugin.processStylexRules(rules, true)).toBe(
        `\n@layer priority1;\n${atRule}`,
      );

      // Input order and duplicate declarations must not affect the partition.
      const customProperty = [
        'customProperty',
        { ltr: '.custom{--card-padding:16px}', rtl: null },
        1,
      ];
      expect(
        stylexPlugin.processStylexRules(
          [customProperty, ...rules, customProperty],
          true,
        ),
      ).toBe(
        `\n@layer priority1;\n${atRule}\n@layer priority1{\n.custom{--card-padding:16px}\n}`,
      );
    });

    test('useLayers with before option', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            before: ['reset', 'typography'],
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer reset, typography, priority1, priority2, priority3, priority4;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('useLayers with after option', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            after: ['overrides', 'xds.theme'],
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer priority1, priority2, priority3, priority4, overrides, xds.theme;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('useLayers with both before and after', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            before: ['reset'],
            after: ['xds.theme'],
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer reset, priority1, priority2, priority3, priority4, xds.theme;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('useLayers with prefix option', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            prefix: 'stylex',
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer stylex.priority1, stylex.priority2, stylex.priority3, stylex.priority4;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer stylex.priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer stylex.priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer stylex.priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer stylex.priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('useLayers with prefix, before, and after combined', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            prefix: 'stylex',
            before: ['reset', 'typography'],
            after: ['xds.theme'],
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer reset, typography, stylex.priority1, stylex.priority2, stylex.priority3, stylex.priority4, xds.theme;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer stylex.priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer stylex.priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer stylex.priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer stylex.priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('useLayers with multi-segment dot-notated prefix (XDS use case)', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            prefix: 'xds.base',
            before: ['xds.reset', 'xds.typography'],
            after: ['xds.theme'],
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer xds.reset, xds.typography, xds.base.priority1, xds.base.priority2, xds.base.priority3, xds.base.priority4, xds.theme;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer xds.base.priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer xds.base.priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer xds.base.priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer xds.base.priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('empty before/after produce standard layer declaration', () => {
      const { metadata } = transform(fixture);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: {
            before: [],
            after: [],
          },
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        "
        @layer priority1, priority2, priority3, priority4;
        @property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        @layer priority1{
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        }
        @layer priority2{
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        }
        @layer priority3{
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        }
        @layer priority4{
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
        }"
      `);
    });

    test('all rules (legacyDisableLayers:true)', () => {
      const { code, metadata } = transform(fixture);
      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const constants = {
          YELLOW: "yellow",
          ORANGE: "var(--orange-theme-color)",
          mediaBig: "@media (max-width: 1000px)",
          mediaSmall: "@media (max-width: 500px)"
        };
        export const vars = {
          blue: "var(--xpqh4lw)",
          marginTokens: "var(--x8nt2k2)",
          colorTokens: "var(--xkxfyv)",
          __varGroupHash__: "xsg933n"
        };
        export const spacing = {
          small: "var(--x19twipt)",
          medium: "var(--xypjos2)",
          large: "var(--x1ec7iuc)",
          __varGroupHash__: "xbiwvf9"
        };
        export const themeColor = {
          xsg933n: "x1coplze xsg933n",
          $$css: true
        };
        export const themeSpacing = {
          xbiwvf9: "x4hn0rr xbiwvf9",
          $$css: true
        };
        export const styles = {
          root: {
            "animationName-kKVMdj": "x13ah0pd",
            "backgroundColor-kWkggS": "xrkmrrc xbrh7vm xfy810d xahc4vn x1t4kl4c x975j7z",
            "margin-kogj98": "xymmreb",
            "borderColor-kVAM5u": "x1bg2uv5 xio2edn xqiy1ys",
            "outlineColor-kjBf7l": "x18abd1y",
            "textShadow-kKMj4B": "x1skrh0i xtj17id",
            "padding-kmVPX3": "x1s2izit",
            "float-kyUFMd": "x1kmio9f",
            $$css: "main.js:33"
          },
          overrideColor: {
            "--orange-theme-color": "xufgesz",
            $$css: "main.js:71"
          },
          dynamic: color => [color !== undefined && {
            "color-kMwMTN": color != null ? "x14rh7hd" : "",
            $$css: "main.js:74"
          }, {
            "--x-color": color != null ? color : undefined
          }]
        };"
      `);
      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: false,
          legacyDisableLayers: true,
        }),
      ).toMatchInlineSnapshot(`
        "@property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        html:not([dir='rtl']) .x1kmio9f{float:left}
        html[dir='rtl'] .x1kmio9f{float:right}
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}"
      `);
    });

    test('legacy-expand-shorthands with logical styles polyfill', () => {
      const { code, metadata } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({
          container: {
            margin: '10px 20px',
            padding: '5px 15px',
            float: 'inline-start'
          }
        });
      `,
        {
          styleResolution: 'legacy-expand-shorthands',
          enableLogicalStylesPolyfill: true,
        },
      );

      expect(code).toMatchInlineSnapshot(`
        "import * as stylex from '@stylexjs/stylex';
        export const constants = {
          YELLOW: "yellow",
          ORANGE: "var(--orange-theme-color)",
          mediaBig: "@media (max-width: 1000px)",
          mediaSmall: "@media (max-width: 500px)"
        };
        export const vars = {
          blue: "var(--xpqh4lw)",
          marginTokens: "var(--x8nt2k2)",
          colorTokens: "var(--xkxfyv)",
          __varGroupHash__: "xsg933n"
        };
        export const spacing = {
          small: "var(--x19twipt)",
          medium: "var(--xypjos2)",
          large: "var(--x1ec7iuc)",
          __varGroupHash__: "xbiwvf9"
        };
        export const styles = {
          container: {
            "marginTop-keoZOQ": "x1anpbxc",
            "marginInlineEnd-k71WvV": "x3aesyq",
            "marginBottom-k1K539": "xyorhqc",
            "marginInlineStart-keTefX": "xqsn43r",
            "paddingTop-kLKAdn": "x123j3cw",
            "paddingInlineEnd-kwRFfy": "x1q3ajuy",
            "paddingBottom-kGO01o": "xs9asl8",
            "paddingInlineStart-kZCmMZ": "x1gx403c",
            "float-kyUFMd": "xj87blo",
            $$css: "main.js:25"
          }
        };"
      `);

      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: true,
        }),
      ).toMatchInlineSnapshot(`
        ":root, [dir="ltr"] {
          --stylex-logical-start: left;
          --stylex-logical-end: right;
        }
        [dir="rtl"] {
          --stylex-logical-start: right;
          --stylex-logical-end: left;
        }
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .xj87blo:not(#\\#){float:var(--stylex-logical-start)}
        /* @ltr begin */.xqsn43r:not(#\\#){margin-left:20px}/* @ltr end */
        /* @rtl begin */.xqsn43r:not(#\\#){margin-right:20px}/* @rtl end */
        /* @ltr begin */.x3aesyq:not(#\\#){margin-right:20px}/* @ltr end */
        /* @rtl begin */.x3aesyq:not(#\\#){margin-left:20px}/* @rtl end */
        /* @ltr begin */.x1gx403c:not(#\\#){padding-left:15px}/* @ltr end */
        /* @rtl begin */.x1gx403c:not(#\\#){padding-right:15px}/* @rtl end */
        /* @ltr begin */.x1q3ajuy:not(#\\#){padding-right:15px}/* @ltr end */
        /* @rtl begin */.x1q3ajuy:not(#\\#){padding-left:15px}/* @rtl end */
        .xyorhqc:not(#\\#):not(#\\#){margin-bottom:10px}
        .x1anpbxc:not(#\\#):not(#\\#){margin-top:10px}
        .xs9asl8:not(#\\#):not(#\\#){padding-bottom:5px}
        .x123j3cw:not(#\\#):not(#\\#){padding-top:5px}"
      `);
    });

    // The `float` value arrives via a constant, so it is only recognizable as a
    // logical float after constants are substituted.
    test('logical float vars are emitted when the float comes from a constant', () => {
      const rules = [
        [
          'cHash',
          { constKey: 'cHash', constVal: 'var(--stylex-logical-start)' },
          0,
        ],
        ['x1', { ltr: '.x1{float:var(--cHash)}', rtl: null }, 3000],
      ];

      expect(stylexPlugin.processStylexRules(rules, true))
        .toMatchInlineSnapshot(`
        ":root, [dir="ltr"] {
          --stylex-logical-start: left;
          --stylex-logical-end: right;
        }
        [dir="rtl"] {
          --stylex-logical-start: right;
          --stylex-logical-end: left;
        }

        @layer priority1;
        @layer priority1{
        .x1{float:var(--stylex-logical-start)}
        }"
      `);
    });

    // A constant's value is data, not a replacement pattern. Substituting it
    // with a string replacement made `String.prototype.replace`/`replaceAll`
    // expand its `$` sequences: `$&` re-inserted the matched text, `$$`
    // collapsed to one `$`, and `` $` ``/`$'` inserted the text around the
    // match. Both the alias pre-collapse and the per-rule substitution have
    // to insert values verbatim.
    test('substitutes a constant value containing `$&` verbatim', () => {
      // `$&` re-inserted `var(--b)` over itself, so the pre-collapse loop made
      // no progress and rescanned from the start forever -- a hung build, not
      // a thrown error.
      const rules = [
        ['a', { constKey: 'a', constVal: 'var(--b)', ltr: '', rtl: null }, 0],
        ['b', { constKey: 'b', constVal: '$&', ltr: '', rtl: null }, 0],
        ['x1', { ltr: '.x1{content:var(--a)}', rtl: null }, 3000],
      ];

      expect(
        stylexPlugin.processStylexRules(rules, { useLayers: false }),
      ).toMatchInlineSnapshot('".x1{content:$&}"');
    });

    test('substitutes a constant value containing `$&` with no alias chain', () => {
      // Without a chain the pre-collapse loop never runs, but the per-rule
      // substitution still expanded `$&` -- leaving the `var()` unsubstituted.
      const rules = [
        ['b', { constKey: 'b', constVal: '$&', ltr: '', rtl: null }, 0],
        ['x1', { ltr: '.x1{content:var(--b)}', rtl: null }, 3000],
      ];

      expect(
        stylexPlugin.processStylexRules(rules, { useLayers: false }),
      ).toMatchInlineSnapshot('".x1{content:$&}"');
    });

    test('substitutes constant values containing other `$` sequences verbatim', () => {
      const rules = [
        [
          'a1',
          { constKey: 'a1', constVal: 'var(--b1)', ltr: '', rtl: null },
          0,
        ],
        ['b1', { constKey: 'b1', constVal: '$$', ltr: '', rtl: null }, 0],
        [
          'a2',
          { constKey: 'a2', constVal: 'var(--b2)', ltr: '', rtl: null },
          0,
        ],
        ['b2', { constKey: 'b2', constVal: '$`', ltr: '', rtl: null }, 0],
        [
          'a3',
          { constKey: 'a3', constVal: 'var(--b3)', ltr: '', rtl: null },
          0,
        ],
        ['b3', { constKey: 'b3', constVal: "$'", ltr: '', rtl: null }, 0],
        [
          'x1',
          {
            ltr: '.x1{--p1:var(--a1);--p2:var(--a2);--p3:var(--a3)}',
            rtl: null,
          },
          3000,
        ],
      ];

      expect(
        stylexPlugin.processStylexRules(rules, { useLayers: false }),
      ).toMatchInlineSnapshot('".x1{--p1:$$;--p2:$`;--p3:$\'}"');
    });

    test('legacy-expand-shorthands duplicates theme selectors for higher precedence', () => {
      const { _code, metadata } = transform(
        `
        import * as stylex from '@stylexjs/stylex';
        export const themeColor = stylex.createTheme(vars, {
          blue: 'lightblue'
        });
        export const themeSpacing = stylex.createTheme(spacing, {
          small: '5px',
          medium: '10px',
          large: '20px'
        });
      `,
        {
          styleResolution: 'legacy-expand-shorthands',
        },
      );

      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: false,
        }),
      ).toMatchInlineSnapshot(`
        ":root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}"
      `);
    });

    test('no mutation of rules', () => {
      const { metadata } = transform(fixture);

      // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/freeze#deep_freezing
      function deepFreeze(object) {
        const propNames = Reflect.ownKeys(object);

        for (const name of propNames) {
          const value = object[name];

          if (
            (value && typeof value === 'object') ||
            typeof value === 'function'
          ) {
            deepFreeze(value);
          }
        }

        return Object.freeze(object);
      }

      deepFreeze(metadata);

      expect(() => {
        stylexPlugin.processStylexRules(metadata);
      }).not.toThrow();
    });

    test('useLegacyClassnamesSort: false (default behavior)', () => {
      const { _code, metadata } = transform(fixture);

      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: true,
          useLegacyClassnamesSort: false,
          legacyDisableLayers: true,
        }),
      ).toMatchInlineSnapshot(`
        "@property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .xufgesz{--orange-theme-color:red}
        .xymmreb{margin:10px 20px}
        .x1s2izit{padding:var(--x1ec7iuc)}
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        .x13ah0pd{animation-name:x35atj5-B}
        .xrkmrrc{background-color:red}
        .x14rh7hd{color:var(--x-color)}
        /* @ltr begin */.x1kmio9f{float:left}/* @ltr end */
        /* @rtl begin */.x1kmio9f{float:right}/* @rtl end */
        .x18abd1y{outline-color:var(--xkxfyv)}
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}"
      `);
    });

    test('useLegacyClassnamesSort: true (legacy behavior)', () => {
      const { _code, metadata } = transform(fixture);

      expect(
        stylexPlugin.processStylexRules(metadata, {
          useLayers: false,
          enableLTRRTLComments: true,
          useLegacyClassnamesSort: true,
          legacyDisableLayers: true,
        }),
      ).toMatchInlineSnapshot(`
        "@property --x-color { syntax: "*"; inherits: false;}
        @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
        :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
        .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
        .xufgesz{--orange-theme-color:red}
        .x1s2izit{padding:var(--x1ec7iuc)}
        .xymmreb{margin:10px 20px}
        .x1bg2uv5{border-color:green}
        @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
        @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
        .x13ah0pd{animation-name:x35atj5-B}
        .x14rh7hd{color:var(--x-color)}
        .x18abd1y{outline-color:var(--xkxfyv)}
        /* @ltr begin */.x1kmio9f{float:left}/* @ltr end */
        /* @rtl begin */.x1kmio9f{float:right}/* @rtl end */
        .x1skrh0i{text-shadow:1px 2px 3px 4px red}
        .xrkmrrc{background-color:red}
        .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
        .xbrh7vm:hover{background-color:blue}
        @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
        @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
        @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
        @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}"
      `);
    });

    test('sorts min-width with screen and media type', () => {
      const rules = [
        [
          'xLg',
          { ltr: 'var(--xLgHash){.xLg.xLg{color:blue}}', rtl: null },
          6000,
        ],
        [
          'xSm',
          { ltr: 'var(--xSmHash){.xSm.xSm{color:red}}', rtl: null },
          6000,
        ],
        [
          'xLgHash',
          {
            constKey: 'xLgHash',
            constVal: '@media screen and (min-width: 1280px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
        [
          'xSmHash',
          {
            constKey: 'xSmHash',
            constVal: '@media screen and (min-width: 768px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });

      expect(css).toMatchInlineSnapshot(`
        "@media screen and (min-width: 768px){.xSm.xSm{color:red}}
        @media screen and (min-width: 1280px){.xLg.xLg{color:blue}}"
      `);
    });

    test('does not misorder negated min-width media queries', () => {
      // "@media (not (min-width: 1000px))" means the opposite of min-width — a
      // user can produce this directly. We must not sort it as a positive
      // min-width query; instead it falls through to the existing property sort.
      const rules = [
        [
          'xNot',
          { ltr: 'var(--xNotHash){.xNot.xNot{color:red}}', rtl: null },
          6000,
        ],
        [
          'xPos',
          { ltr: 'var(--xPosHash){.xPos.xPos{color:blue}}', rtl: null },
          6000,
        ],
        [
          'xNotHash',
          {
            constKey: 'xNotHash',
            constVal: '@media (not (min-width: 1000px))',
            ltr: '',
            rtl: null,
          },
          0,
        ],
        [
          'xPosHash',
          {
            constKey: 'xPosHash',
            constVal: '@media (min-width: 1000px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 1000px){.xPos.xPos{color:blue}}
        @media (not (min-width: 1000px)){.xNot.xNot{color:red}}"
      `);
    });

    test('does not sort a min-width paired with a negated max-width', () => {
      // "(min-width: 500px) and (not (max-width: 700px))" is effectively
      // "width > 700px", so its 500px bound must not be used as a sort key —
      // that would place it before (min-width: 600px) and let the 600px rule
      // win at widths above 700px. It falls through to the existing sort.
      const mk = (cls, query, decl) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{${decl}}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk(
          'xNarrow',
          '@media screen and (min-width: 500px) and (not (max-width: 700px))',
          'color:red',
        ),
        mk('x600', '@media (min-width: 600px)', 'color:blue'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 600px){.x600.x600{color:blue}}
        @media screen and (min-width: 500px) and (not (max-width: 700px)){.xNarrow.xNarrow{color:red}}"
      `);
    });

    test('sorts min-width breakpoints nested inside another at-rule', () => {
      // The media query is not at the start of the rule, so the sort has to
      // find it within the at-rule chain. Breakpoints only sort against rules
      // sharing the same surrounding conditions — the differing `@supports`
      // pair below must not cross-sort.
      const mk = (cls, prelude) => [
        cls,
        { ltr: `${prelude}{.${cls}.${cls}{color:red}}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk('xWide', '@supports (display:grid){@media (min-width: 1500px)'),
        mk('xNarrow', '@supports (display:grid){@media (min-width: 500px)'),
        mk(
          'yOther',
          '@supports (color:oklab(0 0 0)){@media (min-width: 900px)',
        ),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@supports (color:oklab(0 0 0)){@media (min-width: 900px){.yOther.yOther{color:red}}}
        @supports (display:grid){@media (min-width: 500px){.xNarrow.xNarrow{color:red}}}
        @supports (display:grid){@media (min-width: 1500px){.xWide.xWide{color:red}}}"
      `);
    });

    test('sorts max-width defineConsts breakpoints using real transform metadata', () => {
      // Uses constants.mediaBig = '@media (max-width: 1000px)' and
      // constants.mediaSmall = '@media (max-width: 500px)' from the test fixture.
      // Two separate namespaces give them equal priority, catching the ordering bug.
      const { metadata } = transform(`
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({
          a: { color: { [constants.mediaBig]: 'red' } },
          b: { color: { [constants.mediaSmall]: 'blue' } },
        });
      `);

      const css = stylexPlugin.processStylexRules(metadata, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        ":root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
        :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
        @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
        @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
        @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
        @media (max-width: 1000px){.xz4zmo0.xz4zmo0{color:red}}
        @media (max-width: 500px){.x100plp.x100plp{color:blue}}"
      `);
    });

    test('sorts min-width defineConsts breakpoints in ascending px order', () => {
      const rules = [
        // desktop (1500px) processed first — should appear AFTER tablet in CSS
        [
          'xDesktop',
          {
            ltr: 'var(--xDesktopHash){.xDesktop.xDesktop{width:200px}}',
            rtl: null,
          },
          6000,
        ],
        [
          'xTablet',
          {
            ltr: 'var(--xTabletHash){.xTablet.xTablet{width:500px}}',
            rtl: null,
          },
          6000,
        ],
        [
          'xDesktopHash',
          {
            constKey: 'xDesktopHash',
            constVal: '@media (min-width: 1500px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
        [
          'xTabletHash',
          {
            constKey: 'xTabletHash',
            constVal: '@media (min-width: 1000px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 1000px){.xTablet.xTablet{width:500px}}
        @media (min-width: 1500px){.xDesktop.xDesktop{width:200px}}"
      `);
    });

    test('sorts min-width breakpoints via template literal partial value', () => {
      // defineConsts({ sm: '768px', lg: '1280px' }) used as @media (min-width: ${sm})
      // produces ltr with var() inside the @media condition, not as the whole at-rule
      const rules = [
        [
          'xLg',
          {
            ltr: '@media (min-width: var(--xLgHash)){.xLg.xLg{display:block}}',
            rtl: null,
          },
          6000,
        ],
        [
          'xSm',
          {
            ltr: '@media (min-width: var(--xSmHash)){.xSm.xSm{display:none}}',
            rtl: null,
          },
          6000,
        ],
        [
          'xLgHash',
          { constKey: 'xLgHash', constVal: '1280px', ltr: '', rtl: null },
          0,
        ],
        [
          'xSmHash',
          { constKey: 'xSmHash', constVal: '768px', ltr: '', rtl: null },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 768px){.xSm.xSm{display:none}}
        @media (min-width: 1280px){.xLg.xLg{display:block}}"
      `);
    });

    test('sorts max-width defineConsts breakpoints in descending px order', () => {
      const rules = [
        // small (500px) processed first — should appear AFTER large in CSS
        [
          'xSmall',
          { ltr: 'var(--xSmallHash){.xSmall.xSmall{color:blue}}', rtl: null },
          6000,
        ],
        [
          'xLarge',
          { ltr: 'var(--xLargeHash){.xLarge.xLarge{color:red}}', rtl: null },
          6000,
        ],
        [
          'xSmallHash',
          {
            constKey: 'xSmallHash',
            constVal: '@media (max-width: 500px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
        [
          'xLargeHash',
          {
            constKey: 'xLargeHash',
            constVal: '@media (max-width: 1000px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (max-width: 1000px){.xLarge.xLarge{color:red}}
        @media (max-width: 500px){.xSmall.xSmall{color:blue}}"
      `);
    });

    test('sorts CSS Level 4 range syntax (width >= Xpx) as min-width', () => {
      // MediaQuery.parser normalises (width >= 768px) to min-width: 768px,
      // so Level 4 range syntax gets sorted for free.
      const rules = [
        [
          'xLg',
          { ltr: 'var(--xLgHash){.xLg.xLg{color:red}}', rtl: null },
          6000,
        ],
        [
          'xSm',
          { ltr: 'var(--xSmHash){.xSm.xSm{color:violet}}', rtl: null },
          6000,
        ],
        [
          'xLgHash',
          {
            constKey: 'xLgHash',
            constVal: '@media (width >= 1280px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
        [
          'xSmHash',
          {
            constKey: 'xSmHash',
            constVal: '@media (width >= 768px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (width >= 768px){.xSm.xSm{color:violet}}
        @media (width >= 1280px){.xLg.xLg{color:red}}"
      `);
    });

    test('range queries (both min and max-width) fall through to existing sort', () => {
      // Range queries like (768px <= width <= 1024px) parse as an and{min-width,
      // max-width} pair. Sorting them alongside pure min/max-width queries would
      // break comparator transitivity — a range can compare by min-width against
      // one rule and by max-width against another, creating a cycle. They fall
      // through to the existing property + rule comparison instead.
      const rules = [
        [
          'xLg',
          { ltr: 'var(--xLgHash){.xLg.xLg{color:blue}}', rtl: null },
          6000,
        ],
        [
          'xSm',
          { ltr: 'var(--xSmHash){.xSm.xSm{color:violet}}', rtl: null },
          6000,
        ],
        [
          'xLgHash',
          {
            constKey: 'xLgHash',
            constVal: '@media (1024px <= width <= 1280px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
        [
          'xSmHash',
          {
            constKey: 'xSmHash',
            constVal: '@media (768px <= width <= 1024px)',
            ltr: '',
            rtl: null,
          },
          0,
        ],
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (1024px <= width <= 1280px){.xLg.xLg{color:blue}}
        @media (768px <= width <= 1024px){.xSm.xSm{color:violet}}"
      `);
    });

    test('sort is a total order across px min- and max-width rules', () => {
      // Three rules with different properties and mixed bound directions. The
      // width order applies to some pairs and the declaration order to others,
      // so an inconsistent comparator produces a cycle here and lets the
      // output depend on input order.
      const mk = (cls, query, decl) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{${decl}}}`, rtl: null },
        3000,
      ];
      const a = mk('a', '@media (min-width: 500px)', 'z-index:1');
      const b = mk('b', '@media (max-width: 300px)', 'margin:0');
      const c = mk('c', '@media (min-width: 900px)', 'align-items:start');

      const outputs = [
        [a, b, c],
        [a, c, b],
        [b, a, c],
        [b, c, a],
        [c, a, b],
        [c, b, a],
      ].map((permutation) =>
        stylexPlugin.processStylexRules(
          permutation.map(([key, styleObj, priority]) => [
            key,
            { ...styleObj },
            priority,
          ]),
          { useLayers: false, legacyDisableLayers: true },
        ),
      );

      expect(new Set(outputs).size).toBe(1);
      expect(outputs[0]).toMatchInlineSnapshot(`
        "@media (min-width: 900px){.c.c{align-items:start}}
        @media (max-width: 300px){.b.b{margin:0}}
        @media (min-width: 500px){.a.a{z-index:1}}"
      `);
    });

    test('orders the min-width group before the max-width group', () => {
      // Matches the conventional mobile-first order used by
      // sort-css-media-queries: min-width ascending, then max-width
      // descending.
      const mk = (cls, query) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{color:red}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk('xMaxNarrow', '@media (max-width: 300px)'),
        mk('xMinWide', '@media (min-width: 900px)'),
        mk('xMaxWide', '@media (max-width: 800px)'),
        mk('xMinNarrow', '@media (min-width: 400px)'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 400px){.xMinNarrow.xMinNarrow{color:red}}
        @media (min-width: 900px){.xMinWide.xMinWide{color:red}}
        @media (max-width: 800px){.xMaxWide.xMaxWide{color:red}}
        @media (max-width: 300px){.xMaxNarrow.xMaxNarrow{color:red}}"
      `);
    });

    test('sorts unitless zero, any-unit zero, and mixed-case px', () => {
      // Media feature names and units are both ASCII case-insensitive, and
      // zero is the one length valid without a unit (and zero in any unit).
      const mk = (cls, query) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{color:red}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk('xUpper', '@media (min-width: 900PX)'),
        mk('xZero', '@media (min-width: 0)'),
        mk('xMixed', '@media (MIN-WIDTH: 700Px)'),
        mk('xZeroEm', '@media (max-width: 0em)'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 0){.xZero.xZero{color:red}}
        @media (MIN-WIDTH: 700Px){.xMixed.xMixed{color:red}}
        @media (min-width: 900PX){.xUpper.xUpper{color:red}}
        @media (max-width: 0em){.xZeroEm.xZeroEm{color:red}}"
      `);
    });

    test('does not sort a width whose value is a ratio', () => {
      // "(min-width: 16/9)" is invalid CSS but does parse, as a ratio rather
      // than a length. It must not sort, and — as with a negated bound — must
      // not let a sibling px bound stand in for it.
      const mk = (cls, query) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{color:red}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk('xRatio', '@media (min-width: 16/9)'),
        mk('xPaired', '@media (min-width: 16/9) and (min-width: 400px)'),
        mk('xReal', '@media (min-width: 900px)'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 900px){.xReal.xReal{color:red}}
        @media (min-width: 16/9) and (min-width: 400px){.xPaired.xPaired{color:red}}
        @media (min-width: 16/9){.xRatio.xRatio{color:red}}"
      `);
    });

    test('does not sort a unitless non-zero width', () => {
      // "(min-width: 700)" is invalid CSS — the browser drops the query — so
      // it must not be treated as 700px.
      const mk = (cls, query) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{color:red}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk('xBare', '@media (min-width: 700)'),
        mk('xReal', '@media (min-width: 900px)'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 900px){.xReal.xReal{color:red}}
        @media (min-width: 700){.xBare.xBare{color:red}}"
      `);
    });

    test('does not sort rem breakpoints', () => {
      // A rem bound needs a root font size that is unknown at build time, so
      // these fall through to the existing sort rather than being guessed at.
      const mk = (cls, query) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{color:red}}`, rtl: null },
        3000,
      ];
      // 100rem vs 48rem: numerically ascending order would put 48rem first,
      // so the fall-through order is distinguishable from a width sort.
      const rules = [
        mk('xWide', '@media (min-width: 100rem)'),
        mk('xNarrow', '@media (min-width: 48rem)'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 100rem){.xWide.xWide{color:red}}
        @media (min-width: 48rem){.xNarrow.xNarrow{color:red}}"
      `);
    });

    test('does not sort a query with two bounds on the same side', () => {
      // "(min-width: 500px) and (min-width: 900px)" is really a 900px bound,
      // but taking either value alone would be wrong, so it falls through.
      const mk = (cls, query) => [
        cls,
        { ltr: `${query}{.${cls}.${cls}{color:red}}`, rtl: null },
        3000,
      ];
      const rules = [
        mk('xBoth', '@media (min-width: 500px) and (min-width: 900px)'),
        mk('xSingle', '@media (min-width: 700px)'),
      ];

      const css = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });
      expect(css).toMatchInlineSnapshot(`
        "@media (min-width: 700px){.xSingle.xSingle{color:red}}
        @media (min-width: 500px) and (min-width: 900px){.xBoth.xBoth{color:red}}"
      `);
    });

    test('sort is deterministic regardless of input order', () => {
      // These rules mix @media, @container, @starting-style, var()-wrapped,
      // and plain pseudo-element rules at the same priority.
      // The old comparator had a transitivity violation when comparing across
      // these categories, causing different input orders to produce different
      // output orders.
      const rules = [
        // @media rule
        [
          'xMedia1',
          {
            ltr: '@media (min-width: 48rem){.xMedia1{display:none}}',
            rtl: null,
          },
          6000,
        ],
        // @container rule
        [
          'xContainer1',
          {
            ltr: '@container card (min-width: 31.25rem){.xContainer1{display:flex}}',
            rtl: null,
          },
          6000,
        ],
        // @starting-style rule
        [
          'xStarting1',
          {
            ltr: '@starting-style{.xStarting1{opacity:0}}',
            rtl: null,
          },
          6000,
        ],
        // var()-wrapped rule (generated by StyleX for responsive vars)
        [
          'xVar1',
          {
            ltr: 'var(--x10fi87w){.xVar1.xVar1{grid-template-columns:repeat(2,1fr)}}',
            rtl: null,
          },
          6000,
        ],
        // Plain pseudo-element rule
        ['xPseudo1', { ltr: '.xPseudo1::before{inset:0}', rtl: null }, 6000],
        // Another @media rule with same property as pseudo-element
        [
          'xMedia2',
          {
            ltr: '@media (min-width: 64rem){.xMedia2{inset:0}}',
            rtl: null,
          },
          6000,
        ],
        // Plain rule with same property as @media rule
        ['xPlain1', { ltr: '.xPlain1{display:none}', rtl: null }, 6000],
      ];

      // Process in original order
      const output1 = stylexPlugin.processStylexRules(rules, {
        useLayers: false,
        legacyDisableLayers: true,
      });

      // Process in reversed order
      const reversed = [...rules].reverse();
      const output2 = stylexPlugin.processStylexRules(reversed, {
        useLayers: false,
        legacyDisableLayers: true,
      });

      // Process in a shuffled order
      const shuffled = [
        rules[4], // xPseudo1
        rules[0], // xMedia1
        rules[3], // xVar1
        rules[6], // xPlain1
        rules[2], // xStarting1
        rules[5], // xMedia2
        rules[1], // xContainer1
      ];
      const output3 = stylexPlugin.processStylexRules(shuffled, {
        useLayers: false,
        legacyDisableLayers: true,
      });

      expect(output1).toMatchInlineSnapshot(`
        "@container card (min-width: 31.25rem){.xContainer1{display:flex}}
        .xPlain1{display:none}
        @media (min-width: 48rem){.xMedia1{display:none}}
        var(--x10fi87w){.xVar1.xVar1{grid-template-columns:repeat(2,1fr)}}
        .xPseudo1::before{inset:0}
        @media (min-width: 64rem){.xMedia2{inset:0}}
        @starting-style{.xStarting1{opacity:0}}"
      `);
      expect(output2).toMatchInlineSnapshot(`
        "@container card (min-width: 31.25rem){.xContainer1{display:flex}}
        .xPlain1{display:none}
        @media (min-width: 48rem){.xMedia1{display:none}}
        var(--x10fi87w){.xVar1.xVar1{grid-template-columns:repeat(2,1fr)}}
        .xPseudo1::before{inset:0}
        @media (min-width: 64rem){.xMedia2{inset:0}}
        @starting-style{.xStarting1{opacity:0}}"
      `);
      expect(output3).toMatchInlineSnapshot(`
        "@container card (min-width: 31.25rem){.xContainer1{display:flex}}
        .xPlain1{display:none}
        @media (min-width: 48rem){.xMedia1{display:none}}
        var(--x10fi87w){.xVar1.xVar1{grid-template-columns:repeat(2,1fr)}}
        .xPseudo1::before{inset:0}
        @media (min-width: 64rem){.xMedia2{inset:0}}
        @starting-style{.xStarting1{opacity:0}}"
      `);
    });

    test('sort is deterministic with duplicate rules in different input orders', () => {
      // When the same rule appears multiple times (e.g. from multiple modules
      // importing the same component), the sort must still be deterministic.
      const ruleA = [
        'xA',
        { ltr: '@media (min-width: 48rem){.xA{display:flex}}', rtl: null },
        6000,
      ];
      const ruleB = ['xB', { ltr: '.xB::after{inset:0}', rtl: null }, 6000];
      const ruleC = [
        'xC',
        { ltr: '@starting-style{.xC{opacity:0}}', rtl: null },
        6000,
      ];

      // Order 1: A, B, B, C
      const output1 = stylexPlugin.processStylexRules(
        [ruleA, ruleB, ruleB, ruleC],
        { useLayers: false, legacyDisableLayers: true },
      );

      // Order 2: C, B, A, B
      const output2 = stylexPlugin.processStylexRules(
        [ruleC, ruleB, ruleA, ruleB],
        { useLayers: false, legacyDisableLayers: true },
      );

      // Order 3: B, C, B, A
      const output3 = stylexPlugin.processStylexRules(
        [ruleB, ruleC, ruleB, ruleA],
        { useLayers: false, legacyDisableLayers: true },
      );

      expect(output1).toMatchInlineSnapshot(`
        "@media (min-width: 48rem){.xA{display:flex}}
        .xB::after{inset:0}
        @starting-style{.xC{opacity:0}}"
      `);
      expect(output2).toMatchInlineSnapshot(`
        "@media (min-width: 48rem){.xA{display:flex}}
        .xB::after{inset:0}
        @starting-style{.xC{opacity:0}}"
      `);
      expect(output3).toMatchInlineSnapshot(`
        "@media (min-width: 48rem){.xA{display:flex}}
        .xB::after{inset:0}
        @starting-style{.xC{opacity:0}}"
      `);
    });
  });
});
