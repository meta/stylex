/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import * as stylex from '@stylexjs/stylex';
import { Link } from 'lucide-react';
import type { ComponentPropsWithoutRef, ReactElement } from 'react';
import { headingMarker } from './mdx.stylex';
import { vars } from '@/theming/vars.stylex';

type Types = 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';
type HeadingProps<T extends Types> = Omit<
  ComponentPropsWithoutRef<T>,
  'as' | 'className' | 'style'
> & {
  as?: T;
  xstyle?: stylex.StyleXStyles;
};

export default function Heading<T extends Types = 'h1'>({
  as,
  xstyle,
  ...props
}: HeadingProps<T>): ReactElement {
  const As = as ?? 'h1';

  if (!props.id) return <As {...stylex.props(xstyle)} {...props} />;

  return (
    <As
      {...stylex.props(
        styles.heading,
        sizes[As as keyof typeof sizes] ?? {},
        stylex.defaultMarker(),
        headingMarker,
        xstyle,
      )}
      {...props}
    >
      <a data-card="" href={`#${props.id}`} {...stylex.props(styles.anchor)}>
        {props.children}
      </a>
      <Link aria-hidden {...stylex.props(styles.icon)} />
    </As>
  );
}

const styles = stylex.create({
  heading: {
    display: 'flex',
    flexDirection: 'row',
    gap: 8,
    alignItems: 'center',
    marginTop: '1em',
    scrollMarginTop: '7rem',
    // marginBottom: '0.5em',
  },
  anchor: {
    color: 'inherit',
    textDecoration: 'none',
  },
  icon: {
    flexShrink: 0,
    width: 14,
    height: 14,
    color: vars['--color-fd-muted-foreground'],
    opacity: {
      default: 0,
      [stylex.when.ancestor(':hover', headingMarker)]: 1,
    },
    transitionTimingFunction: 'ease',
    transitionDuration: '0.15s',
    transitionProperty: 'opacity',
  },
});

// const TEXT_XS = '0.75rem';
// const TEXT_XS_LH = 'calc(1 / 0.75)';
// const TEXT_SM = '0.875rem';
// const TEXT_SM_LH = 'calc(1.25 / 0.875)';
// const TEXT_LG = '1.125rem';
// const TEXT_LG_LH = 'calc(1.75 / 1.125)';
// const TEXT_2XL = '1.5rem';
// const TEXT_2XL_LH = 'calc(2.5 / 1.5)';
const TEXT_3XL = '1.875rem';
// const TEXT_3XL_LH = 'calc(3.5 / 1.875)';

const sizes = stylex.create({
  h1: {
    fontSize: TEXT_3XL,
    fontWeight: 800,
    lineHeight: 1.1111111,
    // marginTop: 0,
    // marginBottom: '0.8888889em',
  },
  h2: {
    fontSize: '1.4em',
    fontWeight: 600,
    lineHeight: 1.3333333,
    // marginTop: '1.5em',
    // marginBottom: '0.5em',
  },
  h3: {
    fontSize: '1.2em',
    fontWeight: 600,
    lineHeight: 1.6,
    // marginTop: '1.6em',
    // marginBottom: '0.6em',
  },
  h4: {
    fontSize: '1em',
    fontWeight: 600,
    lineHeight: 1.5,
    // marginTop: '1.5em',
    // marginBottom: '0.5em',
  },
  h5: {
    fontSize: '0.875em',
    fontWeight: 500,
    lineHeight: 1.5,
    // marginTop: '1.5em',
    // marginBottom: '0.5em',
  },
  h6: {},
});
