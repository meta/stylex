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

function transform(source: string, opts: any = {}) {
  return transformSync(source, {
    filename: opts.filename || 'TestTheme.stylex.js',
    parserOpts: {
      flow: 'all',
    },
    plugins: [
      [
        stylexPlugin,
        {
          unstable_moduleResolution: { type: 'commonJS' },
          ...opts,
        },
      ],
    ],
  });
}

describe('@stylexjs/babel-plugin', () => {
  describe('[validation] stylex.defineVars()', () => {
    test('invalid export: not bound', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const styles = stylex.defineVars({});
        `);
      }).toThrow(messages.nonExportNamedDeclaration('defineVars'));

      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          stylex.defineVars({});
        `);
      }).toThrow(messages.unboundCallValue('defineVars'));
    });

    test('invalid media query syntax', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            color: {
              default: 'red',
              '@media not ((not (min-width: 400px))': 'blue',
            },
          });
        `);
      }).toThrow(messages.INVALID_MEDIA_QUERY_SYNTAX);
    });

    test('invalid argument: none', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars();
        `);
      }).toThrow(messages.illegalArgumentLength('defineVars', 1));
    });

    test('invalid argument: too many', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({}, {});
        `);
      }).toThrow(messages.illegalArgumentLength('defineVars', 1));
    });

    test('invalid argument: number', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars(1);
        `);
      }).toThrow(messages.nonStyleObject('defineVars'));
    });

    test('invalid argument: string', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars('1');
        `);
      }).toThrow(messages.nonStyleObject('defineVars'));
    });

    test('invalid argument: non-static', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars(genStyles());
        `);
      }).toThrow(messages.nonStaticValue('defineVars'));
    });

    test('valid argument: object', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({});
        `);
      }).not.toThrow();
    });

    test('valid export: separate const and export statement', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const vars = stylex.defineVars({});
          export { vars };
        `);
      }).not.toThrow();
    });

    test('invalid export: re-export from another file does not count', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const vars = stylex.defineVars({});
          export { vars } from './other.stylex.js';
        `);
      }).toThrow(messages.nonExportNamedDeclaration('defineVars'));
    });

    test('invalid export: renamed re-export from another file does not count', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const vars = stylex.defineVars({});
          export { vars as otherVars } from './other.stylex.js';
        `);
      }).toThrow(messages.nonExportNamedDeclaration('defineVars'));
    });

    test('invalid export: default export does not count', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const vars = stylex.defineVars({});
          export default vars;
        `);
      }).toThrow(messages.nonExportNamedDeclaration('defineVars'));
    });

    test('invalid export: renamed export with as syntax', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          const vars = stylex.defineVars({});
          export { vars as themeVars };
        `);
      }).toThrow(messages.nonExportNamedDeclaration('defineVars'));
    });

    /* Properties */

    test('invalid key: non-static', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            [labelColor]: 'red',
          });
        `);
      }).toThrow(messages.nonStaticValue('defineVars'));
    });

    /* Values */

    test('invalid value: non-static', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            labelColor: labelColor,
          });
        `);
      }).toThrow(messages.nonStaticValue('defineVars'));

      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            labelColor: labelColor(),
          });
        `);
      }).toThrow(messages.nonStaticValue('defineVars'));
    });

    test('valid value: number', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            cornerRadius: 5,
          });
        `);
      }).not.toThrow();
    });

    test('valid value: string', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            labelColor: 'red',
          });
        `);
      }).not.toThrow();
    });

    test('valid value: keyframes', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const vars = stylex.defineVars({
            fadeIn: stylex.keyframes({
              '0%': { opacity: 0 },
              '100%': { opacity: 1}
            }),
          });
        `);
      }).not.toThrow();
    });

    test('valid value: same-group function reference', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: () => \`color-mix(\${colors.text}, transparent 50%)\`,
          });
        `);
      }).not.toThrow();
    });

    test('invalid function value: parameterized', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: (value) => value,
          });
        `);
      }).toThrow(messages.invalidDefineVarsFunctionValue());
    });

    test('invalid function value: non-static body', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: () => getColor(colors.text),
          });
        `);
      }).toThrow(messages.nonStaticValue('defineVars'));
    });

    test('valid function value: returns stylex.types', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: () => stylex.types.color('red'),
          });
        `);
      }).not.toThrow();
    });

    test('valid function value: returns object', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            textMuted: () => ({
              default: 'red',
              '@media (prefers-color-scheme: dark)': 'blue',
            }),
          });
        `);
      }).not.toThrow();
    });

    test('invalid same-group reference: unknown key', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: () => colors.missing,
          });
        `);
      }).toThrow(messages.unknownDefineVarsReference('textMuted', 'missing'));
    });

    test('invalid same-group reference: direct cycle', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            text: 'black',
            textMuted: () => colors.textMuted,
          });
        `);
      }).toThrow(messages.cyclicDefineVarsReference('textMuted -> textMuted'));
    });

    test('invalid same-group reference: indirect cycle', () => {
      expect(() => {
        transform(`
          import * as stylex from '@stylexjs/stylex';
          export const colors = stylex.defineVars({
            a: () => colors.b,
            b: () => colors.c,
            c: () => colors.a,
          });
        `);
      }).toThrow(messages.cyclicDefineVarsReference('a -> b -> c -> a'));
    });
  });
});
