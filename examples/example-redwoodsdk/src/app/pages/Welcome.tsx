/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as stylex from '@stylexjs/stylex';
import { Button } from '@stylexjs/shared-ui';
import { tokens } from '@stylexjs/shared-ui/tokens.stylex';
import { Copy } from '../components/Copy';

export const Welcome = () => {
  return (
    <div {...stylex.props(styles.container)}>
      <header {...stylex.props(styles.header)}>
        <h1 {...stylex.props(styles.title)}>Welcome to RedwoodSDK x StyleX</h1>
        <p {...stylex.props(styles.subtitle)}>
          You’ve just installed the starter project. Here’s what to do next.
        </p>
      </header>

      <main>
        <section {...stylex.props(styles.section)}>
          <h2 {...stylex.props(styles.sectionTitle)}>Next steps</h2>
          <ol {...stylex.props(styles.list)}>
            <li {...stylex.props(styles.listItem)}>
              Read the{' '}
              <a
                href="https://docs.rwsdk.com/getting-started/quick-start/"
                rel="noreferrer"
                target="_blank"
                {...stylex.props(styles.link)}
              >
                Quick Start
              </a>{' '}
              to learn the basics.
            </li>
            <li {...stylex.props(styles.listItem)}>
              Explore React Server Components and Server Functions in the{' '}
              <a
                href="https://docs.rwsdk.com/"
                rel="noreferrer"
                target="_blank"
                {...stylex.props(styles.link)}
              >
                Docs
              </a>
              .
            </li>
            <li {...stylex.props(styles.listItem)}>
              Join the community to ask questions and share what you’re
              building.
            </li>
          </ol>
        </section>

        <section {...stylex.props(styles.section)}>
          <h2 {...stylex.props(styles.sectionTitle)}>Deploy to Cloudflare</h2>
          <p>
            RedwoodSDK runs on Cloudflare Workers. Here’s the quickest way to
            deploy.
          </p>
          <div {...stylex.props(styles.codeBlock)}>
            <span {...stylex.props(styles.codePrompt)}>$</span>
            <code {...stylex.props(styles.code)}>pnpm release</code>
            <Copy textToCopy="pnpm release" />
          </div>
          <p>
            Need more detail? Read the{' '}
            <a
              href="https://docs.rwsdk.com/core/hosting/"
              rel="noreferrer"
              target="_blank"
              {...stylex.props(styles.link)}
            >
              Cloudflare deployment guide
            </a>
            .
          </p>
        </section>
        <section>
          <Button xstyle={styles.btn}>button from @stylexjs/shared-ui</Button>
        </section>
      </main>
    </div>
  );
};

const styles = stylex.create({
  container: {
    maxWidth: 1100,
    marginInline: 'auto',
    paddingBlock: 80,
    paddingInline: 32,
    fontFamily:
      "Noto Sans, system-ui, -apple-system, Segoe UI, Roboto, Ubuntu, Cantarell, 'Helvetica Neue', Arial, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol'",
    color: '#1a1a1a',
    minHeight: '100vh',
  },
  header: { marginBottom: 64 },
  title: {
    fontFamily: 'Playfair Display, serif',
    fontSize: 64,
    fontWeight: 700,
    lineHeight: 0.9,
    margin: 0,
  },
  subtitle: {
    fontFamily:
      "Noto Sans, system-ui, -apple-system, Segoe UI, Roboto, Ubuntu, Cantarell, 'Helvetica Neue', Arial, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol'",
    fontSize: 26,
    marginTop: 12,
  },
  section: { marginBottom: 64 },
  sectionTitle: {
    fontFamily: 'Playfair Display, serif',
    fontSize: 40,
    fontWeight: 700,
    marginBottom: 16,
  },
  list: {
    listStylePosition: 'inside',
    listStyleType: 'decimal',
    paddingLeft: 0,
    fontSize: 20,
    lineHeight: 1.6,
  },
  listItem: { marginBottom: 12 },
  link: {
    color: {
      default: '#f47238',
      ':hover': '#ffad48',
    },
    fontWeight: 700,
    textDecorationLine: 'none',
  },
  codeBlock: {
    backgroundColor: '#1b1b1b',
    color: '#ffad48',
    padding: 16,
    borderRadius: 8,
    fontFamily:
      'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    fontSize: 20,
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    marginTop: 16,
  },
  codePrompt: { color: '#f47238' },
  code: { flexGrow: 1 },
  btn: {
    backgroundColor: {
      default: `color-mix(in oklab, ${tokens.primaryColor} 50%, transparent)`,
      ':hover': `color-mix(in oklab, ${tokens.primaryColor} 95%, transparent)`,
    },
    transform: {
      default: null,
      ':active': 'scale(0.97)',
    },
    transitionProperty: 'transform',
    transitionDuration: {
      default: '0.3s',
      ':active': '0.05s',
    },
  },
});
