/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.disableAutomock();

const { RuleTester: ESLintTester } = require('eslint');
const rule = require('../src/stylex-sort-keys');

const eslintTester = new ESLintTester({
  parser: require.resolve('hermes-eslint'),
  parserOptions: {
    ecmaVersion: 6,
    sourceType: 'module',
  },
});

eslintTester.run('stylex-sort-keys', rule.default, {
  valid: [
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        main: {
          borderColor: {
            default: 'green',
            ':hover': 'red',
            '@media (min-width: 1540px)': 1366,
          },
          borderRadius: 10,
          display: 'flex',
        },
        dynamic: (color) => ({
          backgroundColor: color,
        })
      })
    `,
    },
    {
      options: [{ order: 'clean' }],
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        main: {
          display: 'flex',
          borderColor: {
            default: 'green',
            ':hover': 'red',
            '@media (min-width: 1540px)': 1366,
          },
          borderRadius: 10,
        },
        dynamic: (color) => ({
          backgroundColor: color,
        })
      })
    `,
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          width: {
            default: '100%',
            '@supports (width: 100dvw)': {
              default: '100dvw',
              '@media (max-width: 1000px)': '100px',
            },
            ':hover': {
              color: 'red',
            }
          },
        },
      });`,
    },
    {
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const obj = { fontSize: '12px' };
      const styles = cr({
        button: {
          alignItems: 'center',
          display: 'flex',
          ...obj,
          borderColor: 'black',
          alignSelf: 'center',
        }
      });
    `,
    },
    {
      options: [{ order: 'clean' }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const obj = { fontSize: '12px' };
      const styles = cr({
        button: {
          display: 'flex',
          alignItems: 'center',
          ...obj,
          alignSelf: 'center',
          borderColor: 'black',
        }
      });
    `,
    },
    {
      options: [{ order: 'recess' }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const obj = { fontSize: '12px' };
      const styles = cr({
        button: {
          display: 'flex',
          alignItems: 'center',
          ...obj,
          alignSelf: 'center',
          borderColor: 'black',
        }
      });
    `,
    },
    {
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          marginBlock: 6,
          marginInline: 8,
        }
      });
    `,
    },
    {
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          margin: 16,
          marginInline: 8,
          marginBlockEnd: 6,
          marginLeft: 4,
        }
      });
    `,
    },
    {
      options: [{ allowLineSeparatedGroups: true }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          alignItems: 'center',
          display: 'flex',

          borderColor: 'black',
          alignSelf: 'center',
        }
      });
    `,
    },
    {
      options: [{ order: 'clean', allowLineSeparatedGroups: true }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          display: 'flex',
          alignItems: 'center',

          alignSelf: 'center',
          borderColor: 'black',
        }
      });
    `,
    },
    {
      options: [{ order: 'recess', allowLineSeparatedGroups: true }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          display: 'flex',
          alignItems: 'center',

          alignSelf: 'center',
          borderColor: 'black',
        }
      });
    `,
    },
    {
      options: [{ minKeys: 5 }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          flex: 1,
          display: 'flex',
          borderColor: 'black',
          alignItems: 'center',
        }
      });
    `,
    },
    {
      options: [{ validImports: ['a'] }],
      code: `
      import { create as cr } from 'a';
      const styles = cr({
        button: {
          borderColor: 'black',
          display: 'flex',
        }
      });
    `,
    },
    {
      options: [{ validImports: ['a'], order: 'clean' }],
      code: `
      import { create as cr } from 'a';
      const styles = cr({
        button: {
          display: 'flex',
          borderColor: 'black',
        }
      });
    `,
    },
    {
      options: [{ validImports: ['a'], order: 'recess' }],
      code: `
      import { create as cr } from 'a';
      const styles = cr({
        button: {
          display: 'flex',
          borderColor: 'black',
        }
      });
    `,
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
      import { css } from 'a';
      const styles = css.create({
        button: {
          borderColor: 'black',
          display: 'flex',
        }
      });
      `,
    },
    {
      options: [
        {
          validImports: [{ from: 'a', as: 'css' }],
          order: 'clean',
        },
      ],
      code: `
      import { css } from 'a';
      const styles = css.create({
        button: {
          display: 'flex',
          borderColor: 'black',
        }
      });
      `,
    },
    {
      code: `
        import { keyframes } from 'stylex';
        const someAnimation = keyframes({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            borderColor: 'green',
            display: 'flex',
          },
        });
      `,
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import { keyframes } from 'stylex';
        const someAnimation = keyframes({
          '0%': {
            display: 'none',
            borderColor: 'red',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
    },
    {
      options: [{ order: 'recess' }],
      code: `
        import { keyframes } from 'stylex';
        const someAnimation = keyframes({
          '0%': {
            display: 'none',
            borderColor: 'red',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const someAnimation = stylex.keyframes({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            borderColor: 'green',
            display: 'flex',
          },
        });
      `,
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const someAnimation = stylex.keyframes({
          '0%': {
            display: 'none',
            borderColor: 'red',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        nav: {
          paddingBlock: 0,
          maxWidth: {
            default: "1080px",
            "@media (min-width: 2000px)": "calc((1080 / 24) * 1rem)"
          },
        },
      });`,
    },
    {
      // With `enableMediaQueryOrder`, authored media query order is preserved,
      // so a max-width cascade written largest-first is reported as valid.
      options: [{ enableMediaQueryOrder: true }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          container: {
            padding: {
              default: '2rem',
              '@media (max-width: 768px)': '1.5rem',
              '@media (max-width: 640px)': '1rem',
            },
          },
        });
      `,
    },
    {
      // min-width breakpoints authored smallest-first are also left untouched.
      options: [{ enableMediaQueryOrder: true }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          container: {
            fontSize: {
              default: '1rem',
              '@media (min-width: 640px)': '1.25rem',
              '@media (min-width: 1024px)': '1.5rem',
            },
          },
        });
      `,
    },
  ],
  invalid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            animationDuration: '100ms',
            padding: 10,
            fontSize: 12,
          }
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            padding: 10,
            animationDuration: '100ms',
            fontSize: 12,
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "padding" should be above "animationDuration"',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            fontSize: 12,
            animationDuration: '100ms',
            padding: 10,
          }
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            padding: 10,
            animationDuration: '100ms',
            fontSize: 12,
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "animationDuration" should be above "fontSize"',
        },
        {
          message:
            'StyleX property key "padding" should be above "animationDuration"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            padding: 10,
            animationDuration: '100ms',
            fontSize: 12,
          }
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            padding: 10,
            fontSize: 12,
            animationDuration: '100ms',
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "fontSize" should be above "animationDuration"',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const obj = { fontSize: '12px' };
        const styles = stylex.create({
          button: {
            alignItems: 'center',
            display: 'flex',
            ...obj,
            alignSelf: 'center',
            borderColor: 'red', // ok
          }
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const obj = { fontSize: '12px' };
        const styles = stylex.create({
          button: {
            alignItems: 'center',
            display: 'flex',
            ...obj,
            borderColor: 'red', // ok
            alignSelf: 'center',
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "borderColor" should be above "alignSelf"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const obj = { fontSize: '12px' };
        const styles = stylex.create({
          button: {
            alignItems: 'center',
            display: 'flex',
            ...obj,
            borderColor: 'red', // ok
            alignSelf: 'center',
          }
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const obj = { fontSize: '12px' };
        const styles = stylex.create({
          button: {
            display: 'flex',
            alignItems: 'center',
            ...obj,
            alignSelf: 'center',
            borderColor: 'red', // ok
          }
        });
      `,
      errors: [
        {
          message: 'StyleX property key "display" should be above "alignItems"',
        },
        {
          message:
            'StyleX property key "alignSelf" should be above "borderColor"',
        },
      ],
    },
    {
      code: `
        import { create } from 'stylex';
        const styles = create({
          button: {
            alignItems: 'center',
            display: 'flex',
            borderColor: 'red',
          }
        });
      `,
      output: `
        import { create } from 'stylex';
        const styles = create({
          button: {
            borderColor: 'red',
            alignItems: 'center',
            display: 'flex',
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "borderColor" should be above "display"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import { create } from 'stylex';
        const styles = create({
          button: {
            alignItems: 'center',
            display: 'flex',
            borderColor: 'red',
          }
        });
      `,
      output: `
        import { create } from 'stylex';
        const styles = create({
          button: {
            display: 'flex',
            alignItems: 'center',
            borderColor: 'red',
          }
        });
      `,
      errors: [
        {
          message: 'StyleX property key "display" should be above "alignItems"',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const someAnimation = stylex.keyframes({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const someAnimation = stylex.keyframes({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            borderColor: 'green',
            display: 'flex',
          },
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "borderColor" should be above "display"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const someAnimation = stylex.keyframes({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const someAnimation = stylex.keyframes({
          '0%': {
            display: 'none',
            borderColor: 'red',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "display" should be above "borderColor"',
        },
      ],
    },
    {
      code: `
        import { keyframes as kf } from 'stylex';
        const someAnimation = kf({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
      output: `
        import { keyframes as kf } from 'stylex';
        const someAnimation = kf({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            borderColor: 'green',
            display: 'flex',
          },
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "borderColor" should be above "display"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
        import { keyframes as kf } from 'stylex';
        const someAnimation = kf({
          '0%': {
            borderColor: 'red',
            display: 'none',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
      output: `
        import { keyframes as kf } from 'stylex';
        const someAnimation = kf({
          '0%': {
            display: 'none',
            borderColor: 'red',
          },
          '100%': {
            display: 'flex',
            borderColor: 'green',
          },
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "display" should be above "borderColor"',
        },
      ],
    },
    {
      code: `
      import { create } from 'stylex';
      const styles = create({
        main: {
          display: 'flex',
          borderColor: {
            default: 'green',
            '@media (min-width: 1540px)': 1366,
            ':hover': 'red',
          },
          borderRadius: 10,
        },
      });`,
      output: `
      import { create } from 'stylex';
      const styles = create({
        main: {
          borderColor: {
            default: 'green',
            '@media (min-width: 1540px)': 1366,
            ':hover': 'red',
          },
          borderRadius: 10,
          display: 'flex',
        },
      });`,
      errors: [
        {
          message:
            'StyleX property key "borderColor" should be above "display"',
        },
        {
          message:
            'StyleX property key ":hover" should be above "@media (min-width: 1540px)"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
      import { create } from 'stylex';
      const styles = create({
        main: {
          borderColor: {
            default: 'green',
            '@media (min-width: 1540px)': 1366,
            ':hover': 'red',
          },
          display: 'flex',
          borderRadius: 10,
        },
      });`,
      output: `
      import { create } from 'stylex';
      const styles = create({
        main: {
          display: 'flex',
          borderColor: {
            default: 'green',
            '@media (min-width: 1540px)': 1366,
            ':hover': 'red',
          },
          borderRadius: 10,
        },
      });`,
      errors: [
        {
          message:
            'StyleX property key ":hover" should be above "@media (min-width: 1540px)"',
        },
        {
          message:
            'StyleX property key "display" should be above "borderColor"',
        },
      ],
    },
    {
      code: `
      import { create } from 'stylex';
      const styles = create({
        main: {
          backgroundColor: {
            // a
            ':hover': 'blue', // a
            // b
            default: 'red', // b
          },
        },
      });`,
      output: `
      import { create } from 'stylex';
      const styles = create({
        main: {
          backgroundColor: {
            // b
            default: 'red', // b
            // a
            ':hover': 'blue', // a
          },
        },
      });`,
      errors: [
        {
          message: 'StyleX property key "default" should be above ":hover"',
        },
      ],
    },
    {
      code: `
      import { create } from 'stylex';
      const styles = create({
        foo: {
          display: 'flex',
          backgroundColor: {
            // foo
            default: 'red',
            // bar
            /* Block comment */
            ':hover': 'brown',
          },
        }
      });
      `,
      output: `
      import { create } from 'stylex';
      const styles = create({
        foo: {
          backgroundColor: {
            // foo
            default: 'red',
            // bar
            /* Block comment */
            ':hover': 'brown',
          },
          display: 'flex',
        }
      });
      `,
      errors: [
        {
          message:
            'StyleX property key "backgroundColor" should be above "display"',
        },
      ],
    },
    {
      options: [{ order: 'clean' }],
      code: `
      import { create } from 'stylex';
      const styles = create({
        foo: {
          backgroundColor: {
            // foo
            default: 'red',
            // bar
            /* Block comment */
            ':hover': 'brown',
          },
          display: 'flex',
        }
      });
      `,
      output: `
      import { create } from 'stylex';
      const styles = create({
        foo: {
          display: 'flex',
          backgroundColor: {
            // foo
            default: 'red',
            // bar
            /* Block comment */
            ':hover': 'brown',
          },
        }
      });
      `,
      errors: [
        {
          message:
            'StyleX property key "display" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          // zee
          backgroundColor: 'red', // foo
          // bar
          alignItems: 'center' // eee
        }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          // bar
          alignItems: 'center', // eee
          // zee
          backgroundColor: 'red', // foo
        }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: { backgroundColor: 'red', alignItems: 'center', }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: { alignItems: 'center', backgroundColor: 'red', }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: { // foo
          // foo
          backgroundColor: 'red', // bar
          // bar
          alignItems: 'center' // baz
          // qux
        }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: { // foo
          // bar
          alignItems: 'center', // baz
          // foo
          backgroundColor: 'red', // bar
          // qux
        }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          /*
          *
          * foo
          * bar
          * baz
          *
          */
          backgroundColor: 'red',
          alignItems: 'center'
        }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          alignItems: 'center',
          /*
          *
          * foo
          * bar
          * baz
          *
          */
          backgroundColor: 'red',
        }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          backgroundColor: 'red',             //       foo
          alignItems: 'center'       // baz
        }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          alignItems: 'center',       // baz
          backgroundColor: 'red',             //       foo
        }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          /*
          * foo
          */ backgroundColor: 'red',
          alignItems: 'center'
        }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          alignItems: 'center',
          /*
          * foo
          */ backgroundColor: 'red',
        }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          // foo

          backgroundColor: 'red',
          alignItems: 'center'
        }
      })
      `,
      output: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        foo: {
          // foo

          alignItems: 'center',
          backgroundColor: 'red',
        }
      })
      `,
      errors: [
        {
          message:
            'StyleX property key "alignItems" should be above "backgroundColor"',
        },
      ],
    },
    {
      options: [{ allowLineSeparatedGroups: true }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          alignItems: 'center',
          display: 'flex',
          // foo

          // bar
          alignSelf: 'center',
          borderColor: 'black',
        }
      });
      `,
      output: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          alignItems: 'center',
          display: 'flex',
          // foo

          borderColor: 'black',
          // bar
          alignSelf: 'center',
        }
      });
      `,
      errors: [
        {
          message:
            'StyleX property key "borderColor" should be above "alignSelf"',
        },
      ],
    },
    {
      options: [{ order: 'clean', allowLineSeparatedGroups: true }],
      code: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          alignItems: 'center',
          display: 'flex',
          // foo

          // bar
          borderColor: 'black',
          alignSelf: 'center',
        }
      });
      `,
      output: `
      import { create as cr } from '@stylexjs/stylex';
      const styles = cr({
        button: {
          display: 'flex',
          alignItems: 'center',
          // foo

          alignSelf: 'center',
          // bar
          borderColor: 'black',
        }
      });
      `,
      errors: [
        {
          message: 'StyleX property key "display" should be above "alignItems"',
        },
        {
          message:
            'StyleX property key "alignSelf" should be above "borderColor"',
        },
      ],
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            animationDuration: '100ms',
            padding: 10,
            fontSize: 12,
          }
        });
      `,
      output: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            padding: 10,
            animationDuration: '100ms',
            fontSize: 12,
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "padding" should be above "animationDuration"',
        },
      ],
    },
    {
      options: [
        {
          validImports: [{ from: 'a', as: 'css' }],
          order: 'clean',
        },
      ],
      code: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            padding: 10,
            animationDuration: '100ms',
            fontSize: 12,
          }
        });
      `,
      output: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            padding: 10,
            fontSize: 12,
            animationDuration: '100ms',
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "fontSize" should be above "animationDuration"',
        },
      ],
    },
    {
      code: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            ':hover': 10,
            default: 20,
          },
        },
      });
      `,
      output: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            default: 20,
            ':hover': 10,
          },
        },
      });
      `,
      errors: [
        {
          message: 'StyleX property key "default" should be above ":hover"',
        },
      ],
    },
    {
      code: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            ':focus': 10,
            ':hover': 20,
          },
        },
      });
      `,
      output: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            ':hover': 20,
            ':focus': 10,
          },
        },
      });
      `,
      errors: [
        {
          message: 'StyleX property key ":hover" should be above ":focus"',
        },
      ],
    },
    {
      code: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          width: {
            [when.siblingAfter(':active')]: 30,
            [when.descendant(':focus')]: 20,
          },
          display: 'flex',
        },
      });
      `,
      output: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            [when.siblingAfter(':active')]: 30,
            [when.descendant(':focus')]: 20,
          },
        },
      });
      `,
      errors: [
        {
          message:
            'StyleX property key ":when:descendant:focus" should be above ":when:siblingAfter:active"',
        },
        {
          message: 'StyleX property key "display" should be above "width"',
        },
      ],
    },
    {
      code: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            [stylex.when.siblingAfter(':active')]: 30,
            [when.descendant(':focus')]: 20,
          },
        },
      });
      `,
      output: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            [when.descendant(':focus')]: 20,
            [stylex.when.siblingAfter(':active')]: 30,
          },
        },
      });
      `,
      errors: [
        {
          message:
            'StyleX property key ":when:descendant:focus" should be above ":when:siblingAfter:active"',
        },
      ],
    },
    {
      code: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            [when[api](\`:focus\`)]: 20,
            [when[api](\`:active\`)]: 30,
          },
        },
      });
      `,
      output: `
      import { create, when } from '@stylexjs/stylex';
      const styles = create({
        base: {
          display: 'flex',
          width: {
            [when[api](\`:active\`)]: 30,
            [when[api](\`:focus\`)]: 20,
          },
        },
      });
      `,
      errors: [
        {
          message:
            'StyleX property key ":when:api:active" should be above ":when:api:focus"',
        },
      ],
    },
    {
      // Default behavior is unchanged: without the option, max-width media
      // queries are still reordered alphabetically (the cascade-breaking bug).
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          container: {
            padding: {
              default: '2rem',
              '@media (max-width: 768px)': '1.5rem',
              '@media (max-width: 640px)': '1rem',
            },
          },
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          container: {
            padding: {
              default: '2rem',
              '@media (max-width: 640px)': '1rem',
              '@media (max-width: 768px)': '1.5rem',
            },
          },
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "@media (max-width: 640px)" should be above "@media (max-width: 768px)"',
        },
      ],
    },
    {
      // With `enableMediaQueryOrder`, regular property keys are still sorted;
      // only media query ordering is left to the author.
      options: [{ enableMediaQueryOrder: true }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            animationDuration: '100ms',
            padding: 10,
            fontSize: 12,
          }
        });
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            padding: 10,
            animationDuration: '100ms',
            fontSize: 12,
          }
        });
      `,
      errors: [
        {
          message:
            'StyleX property key "padding" should be above "animationDuration"',
        },
      ],
    },
  ],
});
