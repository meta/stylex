/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.disableAutomock();

const { RuleTester: ESLintTester } = require('eslint');
const rule = require('../src/stylex-no-unused');

const eslintTester = new ESLintTester({
  parser: require.resolve('hermes-eslint'),
  parserOptions: {
    ecmaVersion: 6,
    sourceType: 'module',
  },
});

eslintTester.run('stylex-no-unused', rule.default, {
  valid: [
    {
      // all style used; identifier and literal
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
        });
        const sizeStyles = stylex.create({
          [8]: {
            height: 8,
            width: 8,
          },
          [10]: {
            height: 10,
            width: 10,
          },
          [12]: {
            height: 12,
            width: 12,
          },
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.main, styles.dynamic('red'), sizeStyles[8])}>
            </div>
          )
        }
      `,
    },
    {
      // stylex not default export
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
        });
        export const sizeStyles = stylex.create({
          [8]: {
            height: 8,
            width: 8,
          },
          [10]: {
            height: 10,
            width: 10,
          },
          [12]: {
            height: 12,
            width: 12,
          },
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.main, styles.dynamic('red'))}>
            </div>
          )
        }
      `,
    },
    {
      // indirect usage of style
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            display: 'flex',
          },
          dynamic: (color) => ({
            backgroundColor: color,
          })
        });
        const sizeStyles = stylex.create({
          [8]: {
            height: 8,
            width: 8,
          },
          [10]: {
            height: 10,
            width: 10,
          },
          [12]: {
            height: 12,
            width: 12,
          },
        });
        const widthStyles = stylex.create({
            widthModeConstrained: {
              width: 'auto',
            },
            widthModeFlexible: {
              width: '100%',
            },
        })
        // style used as export
        function getWidthStyles() {
          return widthStyles;
        }
        export default function TestComponent({ width: number}) {
          // style used as variable
          const red = styles.dynamic('red');
          const display = width > 10 ? sizeStyles[12] :  sizeStyles[8]
          return(
            <div {...stylex.props(styles.main, red, display)}>
            </div>
          )
        }
      `,
    },
    {
      // styles default export
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
      });
      export default styles;
    `,
    },
    {
      // styles named export specifier
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        main: {
          display: 'flex',
        },
        dynamic: (color) => ({
          backgroundColor: color,
        })
      });
      export { styles };
    `,
    },
    {
      // styles renamed export specifier
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        main: {
          display: 'flex',
        },
      });
      export { styles as sharedStyles };
    `,
    },
    {
      // styles export specifier alongside other exports
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        main: {
          display: 'flex',
        },
      });
      const theme = 'dark';
      export { theme, styles };
    `,
    },
    {
      // styles named default export specifier
      code: `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({
        main: {
          display: 'flex',
        },
      });
      export { styles as default };
    `,
    },
    {
      // styles named default inline export
      code: `
      import * as stylex from '@stylexjs/stylex';
      export default styles = stylex.create({
        maxDimensionsModal: {
          maxWidth: '90%',
          maxHeight: '90%',
        },
        halfWindowWidth: {
          width: '50vw',
        },
      })
    `,
    },
    {
      // styles anonymous default inline export
      code: `
      import * as stylex from '@stylexjs/stylex';
      export default stylex.create({
        maxDimensionsModal: {
          maxWidth: '90%',
          maxHeight: '90%',
        },
        halfWindowWidth: {
          width: '50vw',
        },
      })
    `,
    },
    {
      // Meta-only use of default import
      code: `
      import stylex from 'stylex';
      export default stylex.create({
        maxDimensionsModal: {
          maxWidth: '90%',
          maxHeight: '90%',
        }
      })
    `,
    },
    {
      // importSources with custom import
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
        });
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            color: 'red',
          },
        });
        export default function Component() {
          return <div {...css.props(styles.main)} />;
        }
      `,
    },
  ],
  invalid: [
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
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.dynamic('red'))}>
            </div>
          )
        }
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          dynamic: (color) => ({
            backgroundColor: color,
          })
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.dynamic('red'))}>
            </div>
          )
        }
      `,
      errors: [
        {
          message: 'Unused style detected: styles.main',
        },
      ],
    },
    {
      // Import form: import * as stylex from '@stylexjs/stylex';
      code: `
        import * as customStylex from '@stylexjs/stylex';
        const styles = customStylex.create({
          main: {
            display: 'flex',
          },
          dynamic: (color) => ({
            backgroundColor: color,
          })
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.dynamic('red'))}>
            </div>
          )
        }`,
      output: `
        import * as customStylex from '@stylexjs/stylex';
        const styles = customStylex.create({
          dynamic: (color) => ({
            backgroundColor: color,
          })
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.dynamic('red'))}>
            </div>
          )
        }`,
      errors: [
        {
          message: 'Unused style detected: styles.main',
        },
      ],
    },
    {
      // Import form: import {create as c} from '@stylexjs/stylex';
      code: `
        import {create as c} from '@stylexjs/stylex';
        const styles = c({
          main: {
            display: 'flex',
          },
          dynamic: (color) => ({
            backgroundColor: color,
          })
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.dynamic('red'))}>
            </div>
          )
        }`,
      output: `
        import {create as c} from '@stylexjs/stylex';
        const styles = c({
          dynamic: (color) => ({
            backgroundColor: color,
          })
        });
        export default function TestComponent() {
          return(
            <div {...stylex.props(styles.dynamic('red'))}>
            </div>
          )
        }`,
      errors: [
        {
          message: 'Unused style detected: styles.main',
        },
      ],
    },
    {
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
          unused: {
            fontSize: '16px',
          },
        });
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
      output: `
        import * as stylex from 'custom-stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
        });
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
      errors: [
        {
          message: 'Unused style detected: styles.unused',
        },
      ],
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            color: 'red',
          },
          unused: {
            fontSize: '16px',
          },
        });
        export default function Component() {
          return <div {...css.props(styles.main)} />;
        }
      `,
      output: `
        import { css } from 'a';
        const styles = css.create({
          main: {
            color: 'red',
          },
        });
        export default function Component() {
          return <div {...css.props(styles.main)} />;
        }
      `,
      errors: [
        {
          message: 'Unused style detected: styles.unused',
        },
      ],
    },
    {
      // re-export of another module's binding does not exempt local styles
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
          unused: {
            fontSize: '16px',
          },
        });
        export { styles } from './other-styles';
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
        });
        export { styles } from './other-styles';
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
      errors: [
        {
          message: 'Unused style detected: styles.unused',
        },
      ],
    },
    {
      // type-only export does not export the styles object itself
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
          unused: {
            fontSize: '16px',
          },
        });
        export type { styles };
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
        });
        export type { styles };
        export default function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
      errors: [
        {
          message: 'Unused style detected: styles.unused',
        },
      ],
    },
  ],
});

// `export { type styles }` is TypeScript-only syntax, so the inline type
// specifier cases need the TypeScript parser.
const tsEslintTester = new ESLintTester({
  parser: require.resolve('@typescript-eslint/parser'),
  parserOptions: {
    ecmaVersion: 6,
    sourceType: 'module',
  },
});

tsEslintTester.run('stylex-no-unused (typescript)', rule.default, {
  valid: [
    {
      // styles named export specifier
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            display: 'flex',
          },
        });
        export { styles };
      `,
    },
    {
      // value export specifier alongside an inline type export specifier
      code: `
        import * as stylex from '@stylexjs/stylex';
        type Theme = 'dark';
        const styles = stylex.create({
          main: {
            display: 'flex',
          },
        });
        export { type Theme, styles };
      `,
    },
  ],
  invalid: [
    {
      // inline type export specifier does not exempt local styles
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
          unused: {
            fontSize: '16px',
          },
        });
        export { type styles };
        stylex.props(styles.main);
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
        });
        export { type styles };
        stylex.props(styles.main);
      `,
      errors: [
        {
          message: 'Unused style detected: styles.unused',
        },
      ],
    },
    {
      // type-only export does not exempt local styles
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
          unused: {
            fontSize: '16px',
          },
        });
        export type { styles };
        stylex.props(styles.main);
      `,
      output: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: {
            color: 'red',
          },
        });
        export type { styles };
        stylex.props(styles.main);
      `,
      errors: [
        {
          message: 'Unused style detected: styles.unused',
        },
      ],
    },
  ],
});
