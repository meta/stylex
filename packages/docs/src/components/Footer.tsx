/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import * as React from 'react';
import * as stylex from '@stylexjs/stylex';
import { GithubIcon, TwitterIcon } from 'lucide-react';
import Bluesky from './icons/Bluesky';
import MetaOpenSource from './icons/MetaOpenSource';
import Link from 'fumadocs-core/link';
import { vars } from '@/theming/vars.stylex';

function ExternalLinkIcon(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path d="M15 3h6v6" />
      <path d="M10 14 21 3" />
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    </svg>
  );
}

const footerLinks = {
  develop: [
    { label: 'Learn', href: '/docs/learn' },
    { label: 'API', href: '/docs/api' },
  ],
  explore: [
    { label: 'Playground', href: '/playground' },
    { label: 'Blog', href: '/blog' },
  ],
  participate: [
    {
      label: 'GitHub',
      href: 'https://github.com/facebook/stylex',
      external: true,
    },
    { label: 'Acknowledgements', href: '/docs/acknowledgements' },
  ],
  legal: [
    {
      label: 'Privacy',
      href: 'https://opensource.fb.com/legal/privacy/',
      external: true,
    },
    {
      label: 'Terms',
      href: 'https://opensource.fb.com/legal/terms/',
      external: true,
    },
  ],
};

export default function Footer({
  noBorderTop = false,
}: {
  noBorderTop?: boolean;
}) {
  return (
    <footer
      {...stylex.props(styles.footer, noBorderTop && styles.footerNoBorder)}
    >
      <div {...stylex.props(styles.container)}>
        <div {...stylex.props(styles.grid)}>
          <div>
            <h4 {...stylex.props(styles.heading)}>Develop</h4>
            <ul {...stylex.props(styles.list)}>
              {footerLinks.develop.map((link) => (
                <li key={link.label}>
                  <Link href={link.href} {...stylex.props(styles.link)}>
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h4 {...stylex.props(styles.heading)}>Explore</h4>
            <ul {...stylex.props(styles.list)}>
              {footerLinks.explore.map((link) => (
                <li key={link.label}>
                  <Link href={link.href} {...stylex.props(styles.link)}>
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h4 {...stylex.props(styles.heading)}>Participate</h4>
            <ul {...stylex.props(styles.list)}>
              {footerLinks.participate.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    {...stylex.props(
                      styles.link,
                      link.external && styles.externalLink,
                    )}
                    {...(link.external
                      ? { target: '_blank', rel: 'noopener noreferrer' }
                      : {})}
                  >
                    {link.label}
                    {link.external && (
                      <ExternalLinkIcon
                        {...stylex.props(styles.externalIcon)}
                      />
                    )}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <h4 {...stylex.props(styles.heading)}>Legal</h4>
            <ul {...stylex.props(styles.list)}>
              {footerLinks.legal.map((link) => (
                <li key={link.label}>
                  <Link
                    href={link.href}
                    {...stylex.props(
                      styles.link,
                      link.external && styles.externalLink,
                    )}
                    {...(link.external
                      ? { target: '_blank', rel: 'noopener noreferrer' }
                      : {})}
                  >
                    {link.label}
                    {link.external && (
                      <ExternalLinkIcon
                        {...stylex.props(styles.externalIcon)}
                      />
                    )}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <Link
          href="https://opensource.fb.com"
          rel="noopener noreferrer"
          target="_blank"
          {...stylex.props(styles.metaOpenSourceLink)}
        >
          <MetaOpenSource xstyle={styles.metaOpenSource} />
        </Link>

        <div {...stylex.props(styles.bottom)}>
          <span {...stylex.props(styles.copyright)}>
            Copyright © {new Date().getFullYear()} Meta Platforms, Inc.
          </span>

          <div {...stylex.props(styles.bottomSpacer)} />

          <div {...stylex.props(styles.socialLinks)}>
            <Link
              href="https://github.com/facebook/stylex"
              rel="noopener noreferrer"
              target="_blank"
              {...stylex.props(styles.socialLink)}
            >
              <GithubIcon {...stylex.props(styles.socialIcon)} />
            </Link>
            <Link
              href="https://x.com/stylexjs"
              rel="noopener noreferrer"
              target="_blank"
              {...stylex.props(styles.socialLink)}
            >
              <TwitterIcon {...stylex.props(styles.socialIcon)} />
            </Link>
            <Link
              href="https://bsky.app/profile/stylexjs.bsky.social"
              rel="noopener noreferrer"
              target="_blank"
              {...stylex.props(styles.socialLink)}
            >
              <Bluesky xstyle={styles.socialIcon} />
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}

const styles = stylex.create({
  footer: {
    backgroundColor: vars['--color-fd-background'],
    borderTopColor: vars['--color-fd-border'],
    borderTopStyle: 'solid',
    borderTopWidth: 1,
    transitionDuration: '300ms',
    transitionProperty: 'background-color, border-color',
  },
  footerNoBorder: {
    borderTopColor: null,
    borderTopStyle: null,
    borderTopWidth: null,
  },
  container: {
    display: 'flex',
    flexDirection: 'column',
    gap: 32,
    alignItems: 'center',
    justifyContent: 'center',
    width: '100%',
    padding: 32,
    paddingTop: 48,
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: {
      default: 'repeat(2, 1fr)',
      '@media (min-width: 768px)': 'repeat(4, 1fr)',
    },
    gap: 32,
    width: '100%',
    maxWidth: 1280,
  },
  heading: {
    marginBottom: 16,
    fontSize: '0.875rem',
    fontWeight: 600,
    color: vars['--color-fd-foreground'],
    textAlign: 'center',
  },
  list: {
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
    alignItems: 'center',
    padding: 0,
    margin: 0,
    listStyle: 'none',
  },
  link: {
    fontSize: '0.875rem',
    color: {
      default: vars['--color-fd-muted-foreground'],
      ':hover': vars['--color-fd-foreground'],
    },
    textDecoration: 'none',
    transitionDuration: '150ms',
    transitionProperty: 'color',
  },
  externalLink: {
    display: 'inline-flex',
    gap: 4,
    alignItems: 'center',
  },
  externalIcon: {
    width: 12,
    height: 12,
  },
  bottom: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: 8,
    alignItems: 'center',
    justifyContent: 'center',
    // justifyContent: 'space-between',
    width: '100%',
    maxWidth: 1080,
  },
  bottomSpacer: {
    flexGrow: 1,
    minWidth: 32,
  },
  metaOpenSourceLink: {
    marginTop: 16,
    opacity: {
      default: 0.5,
      ':focus-visible': 1,
      ':hover': 1,
    },
    transitionTimingFunction: 'ease-in-out',
    transitionDuration: '150ms',
    transitionProperty: 'opacity',
  },
  metaOpenSource: {
    height: 68,
  },
  copyright: {
    fontSize: '0.875rem',
    color: vars['--color-fd-muted-foreground'],
  },
  socialLinks: {
    display: 'flex',
    gap: 16,
    alignItems: 'center',
  },
  socialLink: {
    padding: 8,
    color: {
      default: vars['--color-fd-muted-foreground'],
      ':hover': vars['--color-fd-foreground'],
    },
    transitionDuration: '150ms',
    transitionProperty: 'color',
  },
  socialIcon: {
    width: 20,
    height: 20,
  },
});
