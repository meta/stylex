/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.disableAutomock();

const { RuleTester: ESLintTester } = require('eslint');
const rule = require('../src/stylex-enforce-module-scope');

const eslintTester = new ESLintTester({
  parser: require.resolve('hermes-eslint'),
  parserOptions: {
    // Class fields and static blocks need ES2022
    ecmaVersion: 2022,
    sourceType: 'module',
  },
});

const error = (api) => ({
  message: `stylex.${api}() must be called at module scope, not inside a function, component, or class.`,
});

eslintTester.run('stylex-enforce-module-scope', rule.default, {
  valid: [
    `
      import * as stylex from '@stylexjs/stylex';
      const styles = stylex.create({ foo: { color: 'red' } });
      const fade = stylex.keyframes({ from: { opacity: 0 } });
      export const vars = stylex.defineVars({ color: 'red' });
      function Foo() {
        return stylex.props(styles.foo);
      }
    `,
    `
      import { create, props } from '@stylexjs/stylex';
      const styles = create({ foo: { color: 'red' } });
      function Foo() {
        return props(styles.foo);
      }
    `,
    // Blocks at module scope are allowed
    `
      import * as stylex from '@stylexjs/stylex';
      if (true) {
        var styles = stylex.create({ foo: { color: 'red' } });
      }
    `,
    // Non-stylex calls are ignored
    `
      import * as other from 'other';
      function Foo() {
        const styles = other.create({});
      }
    `,
    `
      function Foo() {
        const styles = create({});
      }
    `,
    // Custom import source
    {
      code: `
        import { css } from 'custom';
        const styles = css.create({ foo: { color: 'red' } });
      `,
      options: [{ validImports: [{ from: 'custom', as: 'css' }] }],
    },
  ],
  invalid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        function Foo() {
          const styles = stylex.create({ foo: { color: 'red' } });
        }
      `,
      errors: [error('create')],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const Foo = ({ on }) => {
          const styles = on
            ? stylex.create({ foo: { color: 'blue' } })
            : stylex.create({ foo: { color: 'lime' } });
        };
      `,
      errors: [error('create'), error('create')],
    },
    {
      code: `
        import stylex from '@stylexjs/stylex';
        function useTheme() {
          if (true) {
            return stylex.createTheme(vars, { color: 'blue' });
          }
        }
      `,
      errors: [error('createTheme')],
    },
    {
      code: `
        import { keyframes as kf, defineVars } from '@stylexjs/stylex';
        function Foo() {
          const fade = kf({ from: { opacity: 0 } });
          const vars = defineVars({ color: 'red' });
        }
      `,
      errors: [error('keyframes'), error('defineVars')],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        class Foo {
          styles = stylex.create({ foo: { color: 'red' } });
          static {
            stylex.defineConsts({ a: 1 });
          }
          method() {
            stylex.positionTry({ top: 0 });
          }
        }
      `,
      errors: [error('create'), error('defineConsts'), error('positionTry')],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = (() => stylex.create({ foo: { color: 'red' } }))();
        items.map(() => stylex.keyframes({ from: { opacity: 0 } }));
      `,
      errors: [error('create'), error('keyframes')],
    },
    {
      code: `
        import { css } from 'custom';
        function Foo() {
          css.create({ foo: { color: 'red' } });
        }
      `,
      options: [{ validImports: [{ from: 'custom', as: 'css' }] }],
      errors: [error('create')],
    },
  ],
});

const tsRuleTester = new ESLintTester({
  parser: require.resolve('@typescript-eslint/parser'),
  parserOptions: { sourceType: 'module' },
});

tsRuleTester.run('stylex-enforce-module-scope (TypeScript)', rule.default, {
  valid: [
    `
      import * as stylex from '@stylexjs/stylex';
      namespace Foo {
        export const styles = stylex.create({ foo: { color: 'red' } });
        namespace Bar {
          export const styles = stylex.create({ foo: { color: 'blue' } });
        }
      }
    `,
  ],
  invalid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        namespace Foo {
          export function Component() {
            const styles = stylex.create({ foo: { color: 'red' } });
          }
        }
      `,
      errors: [error('create')],
    },
  ],
});
