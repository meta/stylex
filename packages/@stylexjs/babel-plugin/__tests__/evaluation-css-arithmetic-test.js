/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { transform, stylexPlugin } = require('./__fixtures__/css-arithmetic');

function transformWithConstants(source) {
  const tokens = transform(
    `
      import * as stylex from '@stylexjs/stylex';
      export const constants = stylex.defineConsts({
        a: 26,
        gutter: '16px',
        url: 'https://example.com/x.png',
        line: 'sidebar',
      });
      export const variables = stylex.defineVars({ gap: '8px' });
    `,
    '/src/arithmetic.stylex.js',
  );
  const main = transform(source);
  return {
    code: main.code,
    metadata: main.metadata.stylex,
    css: stylexPlugin.processStylexRules(
      [...tokens.metadata.stylex, ...main.metadata.stylex],
      {
        useLayers: false,
      },
    ),
  };
}

const definitions = `
      import * as stylex from '@stylexjs/stylex';
      export const constants = stylex.defineConsts({ a: 26, b: 14, gutter: '16px' });
      export const variables = stylex.defineVars({ gap: '8px' });
    `;

describe('arithmetic regression snapshots', () => {
  test('calc-shaped keys in intermediate JavaScript lookup tables', () => {
    const { code, metadata, css } = transformWithConstants(`
      import * as stylex from '@stylexjs/stylex';
      const key = 'calc(100% - 10px)';
      const values = {
        [key]: 0.5,
      };
      export const styles = stylex.create({
        root: {
          opacity: values[key],
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      const key = 'calc(100% - 10px)';
      const values = {
        [key]: 0.5
      };
      export const styles = {
        root: {
          kSiTet: "xbyyjgo",
          $$css: true
        }
      };"
    `);
    expect(metadata).toMatchInlineSnapshot(`
      [
        [
          "xbyyjgo",
          {
            "ltr": ".xbyyjgo{opacity:.5}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .xbyyjgo:not(#\\#){opacity:.5}"
    `);
  });

  test('imported constants inside quoted URLs', () => {
    const { code, metadata, css } = transformWithConstants(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          backgroundImage: \`url("\${constants.url}")\`,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kKwaWg: "x1xmaqyp",
          $$css: true
        }
      };"
    `);
    expect(metadata).toMatchInlineSnapshot(`
      [
        [
          "x1xmaqyp",
          {
            "ltr": ".x1xmaqyp{background-image:url("var(--x1fhnzir)")}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .x1xmaqyp:not(#\\#){background-image:url("https://example.com/x.png")}"
    `);
  });

  test('imported constants inside grid line brackets', () => {
    const { code, metadata, css } = transformWithConstants(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          gridTemplateColumns: \`[\${constants.line}] 1fr\`,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kumcoG: "x1w9nni0",
          $$css: true
        }
      };"
    `);
    expect(metadata).toMatchInlineSnapshot(`
      [
        [
          "x1w9nni0",
          {
            "ltr": ".x1w9nni0{grid-template-columns:[var(--x1idfbv)] 1fr}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .x1w9nni0:not(#\\#){grid-template-columns:[sidebar] 1fr}"
    `);
  });

  test('rejects an unsupported Math call through an arrow helper', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const round = (value) => Math.round(value);
      export const styles = stylex.create({
        root: {
          zIndex: round(constants.a),
        },
      });
    `),
    ).toThrow();
  });

  test('slash-separated imported constants and variables in templates', () => {
    const { code, metadata, css } = transformWithConstants(`
      import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      const gutter = constants.gutter;
      export const styles = stylex.create({
        root: {
          borderRadius: \`\${gutter}/\${variables.gap}\`,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      const gutter = constants.gutter;
      export const styles = {
        root: {
          kaIpWk: "x1d5sp0r",
          $$css: true
        }
      };"
    `);
    expect(metadata).toMatchInlineSnapshot(`
      [
        [
          "x1d5sp0r",
          {
            "ltr": ".x1d5sp0r{border-radius:var(--x42dvfa) / var(--x1ez17s4)}",
            "rtl": null,
          },
          2000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .x1d5sp0r:not(#\\#){border-radius:16px / var(--x1ez17s4)}"
    `);
  });

  test('rejects an appended unit after a token inside a prefixed template', () => {
    expect(() =>
      transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          transform: \`translateX(\${constants.gutter}px)\`,
        },
      });
    `),
    ).toThrow();
  });

  test('modern CSS length units survive constant substitution', () => {
    const { code, metadata, css } = transformWithConstants(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const gutter = constants.gutter;
      export const styles = stylex.create({
        root: {
          width: gutter - '2cqw',
          height: gutter - '2cqi',
          marginTop: gutter - '2svmin',
          paddingTop: gutter - '2dvb',
          fontSize: gutter - '2rcap',
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const gutter = constants.gutter;
      export const styles = {
        root: {
          kzqmXN: "x20rgyk",
          kZKoxP: "xfgf0vk",
          keoZOQ: "x11rugz5",
          kLKAdn: "xe27ax8",
          kGuDYH: "x1gogj6f",
          $$css: true
        }
      };"
    `);
    expect(metadata).toMatchInlineSnapshot(`
      [
        [
          "x20rgyk",
          {
            "ltr": ".x20rgyk{width:calc(var(--x42dvfa) - 2cqw)}",
            "rtl": null,
          },
          4000,
        ],
        [
          "xfgf0vk",
          {
            "ltr": ".xfgf0vk{height:calc(var(--x42dvfa) - 2cqi)}",
            "rtl": null,
          },
          4000,
        ],
        [
          "x11rugz5",
          {
            "ltr": ".x11rugz5{margin-top:calc(var(--x42dvfa) - 2svmin)}",
            "rtl": null,
          },
          4000,
        ],
        [
          "xe27ax8",
          {
            "ltr": ".xe27ax8{padding-top:calc(var(--x42dvfa) - 2dvb)}",
            "rtl": null,
          },
          4000,
        ],
        [
          "x1gogj6f",
          {
            "ltr": ".x1gogj6f{font-size:calc(var(--x42dvfa) - 2rcap)}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .x1gogj6f:not(#\\#){font-size:calc(16px - 2rcap)}
      .xfgf0vk:not(#\\#):not(#\\#){height:calc(16px - 2cqi)}
      .x11rugz5:not(#\\#):not(#\\#){margin-top:calc(16px - 2svmin)}
      .xe27ax8:not(#\\#):not(#\\#){padding-top:calc(16px - 2dvb)}
      .x20rgyk:not(#\\#):not(#\\#){width:calc(16px - 2cqw)}"
    `);
  });

  test('Math-derived divisors retain their precision after constant substitution', () => {
    const { code, metadata, css } = transformWithConstants(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      export const styles = stylex.create({
        root: {
          zIndex: local / Math.pow(10, -5),
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      export const styles = {
        root: {
          kY2c9j: "x4pdra",
          $$css: true
        }
      };"
    `);
    expect(metadata).toMatchInlineSnapshot(`
      [
        [
          "x4pdra",
          {
            "ltr": ".x4pdra{z-index:calc(var(--xkc4to4) / .00001)}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .x4pdra:not(#\\#){z-index:calc(26 / .00001)}"
    `);
  });
});

describe('CSS token arithmetic', () => {
  test('reuses a derived alias without losing expression grouping', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      const derived = (local + 2) * 3;
      export const styles = stylex.create({
        root: {
          zIndex: derived / (derived - 1),
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const local = constants.a;
      const derived = (local + 2) * 3;
      export const styles = {
        root: {
          kY2c9j: "x1xutyev",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x1xutyev",
          {
            "ltr": ".x1xutyev{z-index:calc(((var(--xkc4to4) + 2) * 3) / (((var(--xkc4to4) + 2) * 3) - 1))}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('arrow helper retains the imported token', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const add = (value) => value + 2;
      export const styles = stylex.create({
        root: {
          zIndex: add(constants.a),
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      const add = value => value + 2;
      export const styles = {
        root: {
          kY2c9j: "xoqvakk",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "xoqvakk",
          {
            "ltr": ".xoqvakk{z-index:calc(var(--xkc4to4) + 2)}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('negates an imported arithmetic expression', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          zIndex: -(constants.a + 2),
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kY2c9j: "xvj3xgs",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "xvj3xgs",
          {
            "ltr": ".xvj3xgs{z-index:calc(-1 * (var(--xkc4to4) + 2))}",
            "rtl": null,
          },
          3000,
        ],
      ]
    `);
  });

  test('supports explicit Unicode custom property names', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { variables } from 'arithmetic.stylex';
      const local = variables['--größe'];
      export const styles = stylex.create({
        root: {
          width: local * 2,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { variables } from 'arithmetic.stylex';
      const local = variables['--größe'];
      export const styles = {
        root: {
          kzqmXN: "x1a350m5",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x1a350m5",
          {
            "ltr": ".x1a350m5{width:calc(var(--größe) * 2)}",
            "rtl": null,
          },
          4000,
        ],
      ]
    `);
  });

  test('sets a custom property using a computed token key', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          [variables.gap]: constants.a * 2,
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants, variables } from 'arithmetic.stylex';
      export const styles = {
        root: {
          "--x1ez17s4": "x1ltxabd",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x1ltxabd",
          {
            "ltr": ".x1ltxabd{--x1ez17s4:calc(var(--xkc4to4) * 2)}",
            "rtl": null,
          },
          1,
        ],
      ]
    `);
  });

  test('arithmetic inside firstThatWorks and conditional values', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { variables } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          width: {
            default: stylex.firstThatWorks(variables.gap * 2, variables.gap),
            ':hover': variables.gap * 3,
            '@media (min-width: 600px)': variables.gap / 2,
          },
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { variables } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kzqmXN: "x4uyozk x16tcvvl xzlu0ra",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x4uyozk",
          {
            "ltr": ".x4uyozk{width:var(--x1ez17s4);width:calc(var(--x1ez17s4) * 2)}",
            "rtl": null,
          },
          4000,
        ],
        [
          "x16tcvvl",
          {
            "ltr": ".x16tcvvl:hover{width:calc(var(--x1ez17s4) * 3)}",
            "rtl": null,
          },
          4130,
        ],
        [
          "xzlu0ra",
          {
            "ltr": "@media (min-width: 600px){.xzlu0ra.xzlu0ra{width:calc(var(--x1ez17s4) / 2)}}",
            "rtl": null,
          },
          4200,
        ],
      ]
    `);
  });

  test('keeps separated string concatenation as a CSS list', () => {
    const { code, metadata } = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { variables } from 'arithmetic.stylex';
      export const styles = stylex.create({
        root: {
          margin: variables.gap + ' 4px',
        },
      });
    `);
    expect(code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { variables } from 'arithmetic.stylex';
      export const styles = {
        root: {
          kogj98: "x1hbe5so",
          $$css: true
        }
      };"
    `);
    expect(metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x1hbe5so",
          {
            "ltr": ".x1hbe5so{margin:var(--x1ez17s4) 4px}",
            "rtl": null,
          },
          1000,
        ],
      ]
    `);
  });

  test('substitutes constants after evaluating numeric Math and aliases', () => {
    const tokens = transform(definitions, '/src/arithmetic.stylex.js');
    const main = transform(`
      import * as stylex from '@stylexjs/stylex';
      import { constants as imported, variables } from 'arithmetic.stylex';
      const local = imported.a;
      const alias = local;
      const scale = Math.max(Math.floor(2.9), Math.sqrt(4));
      const combined = (alias + imported.b) / scale;
      const gutter = imported.gutter;
      export const styles = stylex.create({
        root: {
          zIndex: combined,
          paddingTop: gutter * scale,
          marginTop: gutter + variables.gap,
          width: \`clamp(8px, \${gutter * scale}, 100px)\`,
        },
      });
    `);
    const rules = [...tokens.metadata.stylex, ...main.metadata.stylex];
    const css = stylexPlugin.processStylexRules(rules, {
      useLayers: false,
    });
    expect(main.code).toMatchInlineSnapshot(`
      "import * as stylex from '@stylexjs/stylex';
      import { constants as imported, variables } from 'arithmetic.stylex';
      const local = imported.a;
      const alias = local;
      const scale = Math.max(Math.floor(2.9), Math.sqrt(4));
      const combined = (alias + imported.b) / scale;
      const gutter = imported.gutter;
      export const styles = {
        root: {
          kY2c9j: "x6llc3m",
          kLKAdn: "x1mcz15a",
          keoZOQ: "xuukq4p",
          kzqmXN: "x1f5zy5f",
          $$css: true
        }
      };"
    `);
    expect(main.metadata.stylex).toMatchInlineSnapshot(`
      [
        [
          "x6llc3m",
          {
            "ltr": ".x6llc3m{z-index:calc((var(--xkc4to4) + var(--xm3quem)) / 2)}",
            "rtl": null,
          },
          3000,
        ],
        [
          "x1mcz15a",
          {
            "ltr": ".x1mcz15a{padding-top:calc(var(--x42dvfa) * 2)}",
            "rtl": null,
          },
          4000,
        ],
        [
          "xuukq4p",
          {
            "ltr": ".xuukq4p{margin-top:calc(var(--x42dvfa) + var(--x1ez17s4))}",
            "rtl": null,
          },
          4000,
        ],
        [
          "x1f5zy5f",
          {
            "ltr": ".x1f5zy5f{width:clamp(8px,calc(var(--x42dvfa) * 2),100px)}",
            "rtl": null,
          },
          4000,
        ],
      ]
    `);
    expect(css).toMatchInlineSnapshot(`
      ":root, .x1w9femo{--x1ez17s4:8px;}
      .x6llc3m:not(#\\#){z-index:calc((26 + 14) / 2)}
      .xuukq4p:not(#\\#):not(#\\#){margin-top:calc(16px + var(--x1ez17s4))}
      .x1mcz15a:not(#\\#):not(#\\#){padding-top:calc(16px * 2)}
      .x1f5zy5f:not(#\\#):not(#\\#){width:clamp(8px,calc(16px * 2),100px)}"
    `);
  });
});
