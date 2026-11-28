/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use client';

import { SidebarContext } from '@/contexts/SidebarContext';
import { vars } from '@/theming/vars.stylex';
import { SidebarIcon } from 'lucide-react';
import * as stylex from '@stylexjs/stylex';
import { useMediaQuery } from 'fumadocs-core/utils/use-media-query';
import { use } from 'react';

export default function SidebarToggle() {
  const [open, setOpen, sidebarId] = use(SidebarContext);
  const isMobile = useMediaQuery('(max-width: 767.9px)');
  const expanded = open ?? (isMobile === null ? undefined : !isMobile);

  return (
    <button
      aria-controls={sidebarId}
      aria-expanded={expanded}
      aria-label="Toggle sidebar"
      onClick={() => {
        setOpen((old) => {
          if (old === null) {
            return window.matchMedia('(max-width: 767.9px)').matches;
          }
          return !old;
        });
      }}
      type="button"
      {...stylex.props(styles.button)}
    >
      <SidebarIcon size={20} />
    </button>
  );
}

const styles = stylex.create({
  button: {
    width: 56,
    height: 56,
    marginInline: (20 - 56) / 2,
    color: {
      default: vars['--color-fd-foreground'],
      ':focus-visible': vars['--color-fd-primary'],
      ':hover': vars['--color-fd-primary'],
    },
    backgroundColor: 'transparent',
    scale: {
      default: null,
      ':active': 0.95,
    },
    transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
    transitionDuration: '0.3s',
    transitionProperty: 'color, scale',
  },
});
