/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

jest.autoMockOff();

import { transformSync } from '@babel/core';
import { messages } from '../src/shared';
import stylexPlugin from '../src/index';

// Valid string terminator sequences are BEL, ESC\, and 0x9c
const ST = '(?:\\u0007|\\u001B\\u005C|\\u009C)';
const pattern = [
  `[\\u001B\\u009B][[\\]()#;?]*(?:(?:(?:(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]+)*|[a-zA-Z\\d]+(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]*)*)?${ST})`,
  '(?:(?:\\d{1,4}(?:;\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]))',
].join('|');
const regex = new RegExp(pattern, 'g');
const cleanExpect = (fn: () => mixed) =>
  expect(() => {
    try {
      fn();
    } catch (error) {
      if (typeof error.message !== 'string') {
        throw new TypeError(
          `Expected a \`string\`, got \`${typeof error.message}\``,
        );
      }
      // Even though the regex is global, we don't need to reset the `.lastIndex`
      // because unlike `.exec()` and `.test()`, `.replace()` does it automatically
      // and doing it manually has a performance penalty.
      error.message = error.message.replace(regex, '');
      throw error;
    }
  });

function transform(source: string, opts: { [key: string]: any } = {}) {
  return transformSync(source, {
    filename: opts.filename,
    parserOpts: {
      flow: 'all',
    },
    plugins: [[stylexPlugin, { ...opts }]],
  });
}

describe('@stylexjs/babel-plugin', () => {
  /**
   * stylex.create
   */

  describe('[validation] stylex.create()', () => {
    test('invalid use: not bound', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          stylex.create({});
        `);
      }).toThrow(messages.unboundCallValue('create'));
    });

    test('invalid argument: none', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create();
        `);
      }).toThrow(messages.illegalArgumentLength('create', 1));
    });

    test('invalid argument: too many', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({}, {});
        `);
      }).toThrow(messages.illegalArgumentLength('create', 1));
    });

    test('invalid argument: non-static', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create(genStyles());
        `);
      }).toThrow(messages.nonStyleObject('create'));
    });

    test('valid argument: object', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({});
        `);
      }).not.toThrow();
    });

    /* Style rule keys */

    describe('style rules', () => {
      test('invalid key: non-static', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          export const styles = stylex.create({
            [root]: {
              backgroundColor: 'red',
            }
          });
        `);
        }).toThrow(messages.nonStaticValue('create'));
      });

      /* Style rules */

      test('invalid rule: non-object', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            namespace: false,
          });
        `);
        }).toThrow(messages.ILLEGAL_NAMESPACE_VALUE);
      });

      test('invalid rule: spread', () => {
        expect(() =>
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const shared = { foo: { color: 'red' } };
          const styles = stylex.create({
            ...shared,
            bar: { color: 'blue' }
          });
        `),
        ).toThrow(messages.NO_OBJECT_SPREADS);
      });

      test('valid rule: object', () => {
        expect(() => {
          transform(`
          const styles = stylex.create({
            namespace: {},
          });
        `);
        }).not.toThrow();
      });

      test('invalid dynamic rule: default object value', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            dynamic: (props = {}) => ({
              color: props.color,
            }),
          });
        `);
        }).toThrow(messages.ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS);
      });

      test('invalid dynamic rule: default string value', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            dynamic: (color = 'red') => ({
              color,
            }),
          });
        `);
        }).toThrow(messages.ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS);
      });

      test('invalid dynamic rule: destructuring ', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            dynamic: ({ color }) => ({
              color,
            }),
          });
        `);
        }).toThrow(messages.ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS);
      });

      test('invalid dynamic rule: rest param ', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            dynamic: (...rest) => ({
              color: rest[0],
            }),
          });
        `);
        }).toThrow(messages.ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS);
      });

      test('valid dynamic rule', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            dynamic: (backgroundColor) => ({
              backgroundColor,
            }),
          });
        `);
        }).not.toThrow();
      });
    });

    describe('style declarations', () => {
      /* Properties */

      test('invalid property: non-static value', () => {
        cleanExpect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              [backgroundColor]: 'red',
            }
          });
        `);
        }).toThrowErrorMatchingInlineSnapshot(`
                  "unknown file: Referenced constant is not defined.
                    3 |           const styles = stylex.create({
                    4 |             root: {
                  > 5 |               [backgroundColor]: 'red',
                      |                ^^^^^^^^^^^^^^^
                    6 |             }
                    7 |           });
                    8 |         "
              `);
      });

      /* Values */

      test('invalid value: boolean', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            default: {
              color: true,
            },
          });
        `);
        }).toThrow(messages.ILLEGAL_PROP_VALUE);
      });

      test('invalid value: non-static', () => {
        cleanExpect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                backgroundColor: backgroundColor,
              }
            });
          `);
        }).toThrowErrorMatchingInlineSnapshot(`
          "unknown file: Referenced constant is not defined.
            3 |             const styles = stylex.create({
            4 |               root: {
          > 5 |                 backgroundColor: backgroundColor,
              |                                  ^^^^^^^^^^^^^^^
            6 |               }
            7 |             });
            8 |           "
        `);

        cleanExpect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                backgroundColor: generateBg(),
              }
            });
          `);
        }).toThrowErrorMatchingInlineSnapshot(`
          "unknown file: Referenced constant is not defined.
            3 |             const styles = stylex.create({
            4 |               root: {
          > 5 |                 backgroundColor: generateBg(),
              |                                  ^^^^^^^^^^
            6 |               }
            7 |             });
            8 |           "
        `);

        cleanExpect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            import {generateBg} from './other-file';
            const styles = stylex.create({
              root: {
                backgroundColor: generateBg(),
              }
            });
          `);
        }).toThrowErrorMatchingInlineSnapshot(`
          "unknown file: Could not resolve the path to the imported file.
          Please ensure that the theme file has a .stylex.js or .stylex.ts extension and follows the
          rules for defining variables:

          https://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables

            1 |
            2 |             import * as stylex from '@stylexjs/stylex';
          > 3 |             import {generateBg} from './other-file';
              |                     ^^^^^^^^^^
            4 |             const styles = stylex.create({
            5 |               root: {
            6 |                 backgroundColor: generateBg(),"
        `);

        cleanExpect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            import generateBg from './other-file';
            const styles = stylex.create({
              root: {
                backgroundColor: generateBg(),
              }
            });
          `);
        }).toThrowErrorMatchingInlineSnapshot(`
          "unknown file: There was an error when attempting to evaluate the imported file.
          Please ensure that the imported file is self-contained and does not rely on dynamic behavior.

            1 |
            2 |             import * as stylex from '@stylexjs/stylex';
          > 3 |             import generateBg from './other-file';
              |                    ^^^^^^^^^^
            4 |             const styles = stylex.create({
            5 |               root: {
            6 |                 backgroundColor: generateBg(),"
        `);
      });

      // TODO: this needs to throw
      test.skip('invalid value: empty object', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              color: {},
            },
          });
        `);
        }).toThrow(messages.ILLEGAL_PROP_VALUE);
      });

      test('invalid value: array of objects', () => {
        cleanExpect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              transitionDuration: [[], {}],
            },
          });
        `);
        }).toThrowErrorMatchingInlineSnapshot(
          '"unknown file: A style array value can only contain strings or numbers."',
        );
      });

      test('valid value: number', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              padding: 5,
            }
          });
        `);
        }).not.toThrow();
      });

      test('valid value: string', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              backgroundColor: 'red',
            }
          });
        `);
        }).not.toThrow();
      });

      test('valid value: array of numbers', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              transitionDuration: [500],
            },
          });
        `);
        }).not.toThrow();
      });

      test('valid value: array of strings', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              transitionDuration: ['0.5s'],
            },
          });
        `);
        }).not.toThrow();
      });

      test('valid value: single-expr function call', () => {
        expect(() =>
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const generateBg = () => 'red';
          export const styles = stylex.create({
            root: {
              backgroundColor: generateBg(),
            }
          });
        `),
        ).not.toThrow();
      });

      test('valid value: single-expr function call in object', () => {
        let result;
        expect(() => {
          result = transform(`
            import * as stylex from '@stylexjs/stylex';
            const fns = {
              generateBg: () => 'red',
            };
            export const styles = stylex.create({
              root: {
                backgroundColor: fns.generateBg(),
              }
            });
          `);
        }).not.toThrow();

        expect(result).not.toBeFalsy();

        expect(result?.code).toMatchInlineSnapshot(`
          "import * as stylex from '@stylexjs/stylex';
          const fns = {
            generateBg: () => 'red'
          };
          export const styles = {
            root: {
              kWkggS: "xrkmrrc",
              $$css: true
            }
          };"
        `);

        // $FlowFixMe[prop-missing]
        expect(result?.metadata?.stylex).toMatchInlineSnapshot(`
          [
            [
              "xrkmrrc",
              {
                "ltr": ".xrkmrrc{background-color:red}",
                "rtl": null,
              },
              3000,
            ],
          ]
        `);
      });

      test('valid value: local variable', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const bg = '#eee';
          const styles = stylex.create({
            root: {
              backgroundColor: bg,
            }
          });
        `);
        }).not.toThrow();
      });

      test('valid value: pure complex expression', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const borderRadius = 2;
          const styles = stylex.create({
            root: {
              borderRadius: borderRadius * 2,
            }
          });
        `);
        }).not.toThrow();
      });

      test('valid value: template literal expressions', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const borderSize = 2;
          const styles = stylex.create({
            root: {
              borderRadius: \`\${borderSize * 2}px\`,
            }
          });
        `);
        }).not.toThrow();
      });

      /* Object values */

      // TODO: This should throw
      test.skip('invalid object value: contains disallowed key', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                'color': {
                  '::before': 'blue'
                },
              },
            });
          `);
        }).toThrow(messages.INVALID_PSEUDO_OR_AT_RULE);

        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                'color': {
                  '&:hover': 'blue'
                },
              },
            });
          `);
        }).toThrow(messages.INVALID_PSEUDO_OR_AT_RULE);

        expect(() => {
          transform(
            `
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                'color': {
                  '@media not ((not (min-width: 400px))': 'blue'
                },
              },
            });
          `,
            { enableMediaQueryOrder: true },
          );
        }).toThrow(messages.INVALID_MEDIA_QUERY_SYNTAX);

        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                'color': {
                  '(min-width:320px)': 'blue'
                },
              },
            });
          `);
        }).toThrow(messages.INVALID_PSEUDO_OR_AT_RULE);
      });

      test('valid object value: key is "default"', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                color: {
                  default: 'red'
                }
              },
            });
          `);
        }).not.toThrow();
      });

      test('valid object value: key starts with ":"', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                color: {
                  ':hover': 'green'
                }
              },
            });
          `);
        }).not.toThrow();
      });

      test('valid object value: multiple valid keys', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                color: {
                  default: 'red',
                  ':hover': 'green'
                }
              },
            });
          `);
        }).not.toThrow();
      });

      test('valid object value: nested pseudo-classes', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                ':hover': {
                  ':active': 'red'
                },
              },
            });
          `);
        }).not.toThrow(messages.ILLEGAL_NESTED_PSEUDO);
      });

      /* Custom properties */

      test('invalid CSS variable: unclosed function', () => {
        const options = { definedStylexCSSVariables: { foo: 1 } };
        expect(() => {
          transform(
            `
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                color: 'var(--foo'
              }
            });
          `,
            options,
          );
        }).toThrow(messages.LINT_UNCLOSED_FUNCTION);
      });

      test('invalid value: unclosed double-quoted string', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                fontFamily: '"Helvetica Neue'
              }
            });
          `);
        }).toThrow(messages.LINT_UNCLOSED_STRING);
      });

      test('invalid value: unclosed single-quoted string', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                fontFamily: "'Helvetica Neue"
              }
            });
          `);
        }).toThrow(messages.LINT_UNCLOSED_STRING);
      });

      test('invalid value: unclosed string in multi-value property', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                fontFamily: '"Helvetica Neue, sans-serif'
              }
            });
          `);
        }).toThrow(messages.LINT_UNCLOSED_STRING);
      });

      test('invalid value: stray apostrophe in font-family list', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                fontFamily: "'SF Pro Text', 'SF Pro Icons', Helvetica Neue', 'Helvetica', sans-serif",
              }
            });
          `);
        }).toThrow(messages.LINT_UNCLOSED_STRING);
      });

      test('valid value: properly closed strings do not throw', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                fontFamily: '"Helvetica Neue", sans-serif',
              }
            });
          `);
        }).not.toThrow();
      });

      test('valid value: properly closed single-quoted string', () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                fontFamily: "'Helvetica Neue', sans-serif",
              }
            });
          `);
        }).not.toThrow();
      });

      test('invalid CSS variable: unprefixed custom property', () => {
        const options = { definedStylexCSSVariables: { foo: 1 } };
        expect(() => {
          transform(
            `
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                color: 'var(foo'
              }
            });
          `,
            options,
          );
        }).toThrow();
      });

      test('valid CSS variable: defined custom properties', () => {
        const options = { definedStylexCSSVariables: { foo: 1, bar: 1 } };
        expect(() => {
          transform(
            `
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({
              root: {
                backgroundColor: 'var(--foo)',
                color: 'var(--bar)'
              }
            });
          `,
            options,
          );
        }).not.toThrow();
      });

      test('valid CSS variable: undefined custom properties', () => {
        const fixture = `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              color: 'var(--bar)'
            }
          });
        `;

        expect(() => transform(fixture)).not.toThrow();

        const optionsNull = { definedStylexCSSVariables: null };
        expect(() => transform(fixture, optionsNull)).not.toThrow();

        const optionsUndefined = { definedStylexCSSVariables: undefined };
        expect(() => transform(fixture, optionsUndefined)).not.toThrow();

        const optionsUnused = { definedStylexCSSVariables: { foo: 1 } };
        expect(() => transform(fixture, optionsUnused)).not.toThrow();
      });

      // TODO: Remove once we've migrated away from this syntax
      test('[legacy] pseudo-classes must start with ":" character', () => {
        expect(() => {
          transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              ':hover': {},
            },
          });
        `);
        }).not.toThrow();
      });
    });
  });

  /**
   * Various shortform properties are disallowed to simplify the way properties are merged.
   */
  describe.skip('[validation] invalid properties', () => {
    const borderValue = '1px solid red';
    const invalidPropertyDeclarations = [
      ['animation', 'anim 1s'],
      ['background', 'red'],
      ['border', borderValue],
      ['borderBlock', borderValue],
      ['borderBlockEnd', borderValue],
      ['borderBlockStart', borderValue],
      ['borderBottom', borderValue],
      ['borderImage', 'url(./img.jpg) 30 space'],
      ['borderInline', borderValue],
      ['borderInlineEnd', borderValue],
      ['borderInlineStart', borderValue],
      ['borderLeft', borderValue],
      ['borderRight', borderValue],
      ['borderTop', borderValue],
      ['flexFlow', 'row wrap'],
      ['font', '16px/16 Arial'],
      ['listStyle', 'square inside'],
      ['textDecoration', '1px solid underline'],
      ['transition', 'opacity 1s'],
    ];

    invalidPropertyDeclarations.forEach(([prop, value]) => {
      test(`invalid property: "${prop}"`, () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({ x: { ${prop}: "${value}" } });
          `);
        }).toThrow(messages.UNKNOWN_PROP_KEY);
      });
    });
  });

  describe.skip('[validation] invalid values', () => {
    const multiLength = '1px 2px';

    const invalidShortformValueDeclarations = [
      ['backgroundPosition', 'top left'],
      ['borderColor', 'red blue'],
      ['borderRadius', multiLength],
      ['borderStyle', 'solid dashed'],
      ['borderWidth', multiLength],
      ['inset', multiLength],
      ['insetBlock', multiLength],
      ['insetInline', multiLength],
      ['flex', '1 1 0'],
      ['grid', '1 1 0'],
      ['margin', multiLength],
      ['marginBlock', multiLength],
      ['marginInline', multiLength],
      ['outline', '1px solid red'],
      ['overflow', 'hidden visible'],
      ['padding', multiLength],
      ['paddingBlock', multiLength],
      ['paddingInline', multiLength],
    ];

    const invalidTransitionPropertyValueDeclarations = [
      'all',
      'bottom',
      'end',
      'height',
      'inset',
      'inset-block',
      'inset-inline',
      'inset-block-end',
      'inset-block-start',
      'inset-inline-end',
      'inset-inline-start',
      'margin',
      'left',
      'padding',
      'right',
      'start',
      'top',
      'width',
    ].map((value) => ['transitionProperty', value]);

    [
      // No !important
      ['display', 'block !important'],
      // No multi-value short-forms
      ...invalidShortformValueDeclarations,
      // No CPU intensive property transitions
      ...invalidTransitionPropertyValueDeclarations,
    ].forEach(([prop, value]) => {
      test(`invalid value: "${value}" for "${prop}"`, () => {
        expect(() => {
          transform(`
            import * as stylex from '@stylexjs/stylex';
            const styles = stylex.create({ x: { ${prop}: "${value}" } });
          `);
        }).toThrow(messages.ILLEGAL_PROP_VALUE);
      });
    });
  });

  describe('[validation] propertyValidationMode config', () => {
    test('does not throw by default for disallowed properties (silent mode)', () => {
      const consoleSpy = jest
        .spyOn(console, 'warn')
        .mockImplementation(() => {});

      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              border: '1px solid red',
            },
          });
        `);
      }).not.toThrow();

      // Should not log any warning in silent mode (default)
      expect(consoleSpy).not.toHaveBeenCalled();

      consoleSpy.mockRestore();
    });

    test('throws error when propertyValidationMode is "throw"', () => {
      expect(() => {
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              border: '1px solid red',
            },
          });
        `,
          { propertyValidationMode: 'throw' },
        );
      }).toThrow('border is not supported');
    });

    test('does not throw when propertyValidationMode is "warn"', () => {
      const consoleSpy = jest
        .spyOn(console, 'warn')
        .mockImplementation(() => {});

      expect(() => {
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              border: '1px solid red',
            },
          });
        `,
          { propertyValidationMode: 'warn' },
        );
      }).not.toThrow();

      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('border is not supported'),
      );

      consoleSpy.mockRestore();
    });

    test('does not throw when propertyValidationMode is "silent"', () => {
      const consoleSpy = jest
        .spyOn(console, 'warn')
        .mockImplementation(() => {});

      expect(() => {
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              border: '1px solid red',
            },
          });
        `,
          { propertyValidationMode: 'silent' },
        );
      }).not.toThrow();

      // Should not log any warning in silent mode
      expect(consoleSpy).not.toHaveBeenCalled();

      consoleSpy.mockRestore();
    });

    test('works with background property', () => {
      expect(() => {
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              background: 'red',
            },
          });
        `,
          { propertyValidationMode: 'silent' },
        );
      }).not.toThrow();
    });

    test('works with animation property', () => {
      expect(() => {
        transform(
          `
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.create({
            root: {
              animation: 'spin 1s',
            },
          });
        `,
          { propertyValidationMode: 'silent' },
        );
      }).not.toThrow();
    });
  });
});
