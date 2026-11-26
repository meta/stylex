/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

jest.disableAutomock();

const { RuleTester: ESLintTester } = require('eslint');
const rule = require('../src/stylex-no-conflicting-props');

const eslintTester = new ESLintTester({
  parser: require.resolve('hermes-eslint'),
  parserOptions: {
    ecmaVersion: 6,
    sourceType: 'module',
    ecmaFeatures: {
      jsx: true,
    },
  },
});

eslintTester.run('stylex-no-conflicting-props', rule.default, {
  valid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        function Component() {
          return <div className="foo" />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        function Component() {
          return <div style={{ color: 'red' }} />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        function Component() {
          return <div sx className="foo" />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        function Component() {
          return <div sx="foo" className="foo" />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} data-testid="test" />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...otherProps} className="foo" />;
        }
      `,
    },
    {
      code: `
        import { props, create } from '@stylexjs/stylex';
        const styles = create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...props(styles.main)} />;
        }
      `,
    },
    {
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} />;
        }
      `,
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        const styles = css.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...css.props(styles.main)} />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div sx={styles.main} data-testid="x" />;
        }
      `,
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <CustomComponent sx={styles.main} className="foo" />;
        }
      `,
    },
    {
      options: [{ sxPropName: false }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div sx={styles.main} className="foo" />;
        }
      `,
    },
  ],
  invalid: [
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} style={{ margin: 10 }} />;
        }
      `,
      errors: [
        {
          message:
            'The `style` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div className="foo" {...stylex.props(styles.main)} />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div style={{ margin: 10 }} {...stylex.props(styles.main)} />;
        }
      `,
      errors: [
        {
          message:
            'The `style` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import { props, create } from '@stylexjs/stylex';
        const styles = create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...props(styles.main)} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import { props as p, create } from '@stylexjs/stylex';
        const styles = create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...p(styles.main)} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      options: [{ validImports: ['custom-stylex'] }],
      code: `
        import * as stylex from 'custom-stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      options: [{ validImports: [{ from: 'a', as: 'css' }] }],
      code: `
        import { css } from 'a';
        const styles = css.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...css.props(styles.main)} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} className="foo" style={{ margin: 10 }} />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
        {
          message:
            'The `style` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} {...{ className: 'foo' }} />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div {...stylex.props(styles.main)} {...{ style: { margin: 10 } }} />;
        }
      `,
      errors: [
        {
          message:
            'The `style` prop should not be used when spreading `stylex.props()` to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div sx={styles.main} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used with the `sx` StyleX prop to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div className="foo" sx={styles.main} />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used with the `sx` StyleX prop to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div sx={styles.main} style={{ margin: 10 }} />;
        }
      `,
      errors: [
        {
          message:
            'The `style` prop should not be used with the `sx` StyleX prop to avoid conflicts.',
        },
      ],
    },
    {
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div sx={styles.main} {...{ className: 'foo' }} />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used with the `sx` StyleX prop to avoid conflicts.',
        },
      ],
    },
    {
      options: [{ sxPropName: 'css' }],
      code: `
        import * as stylex from '@stylexjs/stylex';
        const styles = stylex.create({
          main: { color: 'red' },
        });
        function Component() {
          return <div css={styles.main} className="foo" />;
        }
      `,
      errors: [
        {
          message:
            'The `className` prop should not be used with the `css` StyleX prop to avoid conflicts.',
        },
      ],
    },
  ],
});
