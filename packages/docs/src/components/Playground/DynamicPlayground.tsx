/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use client'; // This must be a client component

import { lazy, Suspense, useEffect, useState } from 'react';
import { QueryParamProvider } from 'use-query-params';
import { WindowHistoryAdapter } from 'use-query-params/adapters/window';
import * as stylex from '@stylexjs/stylex';
import { vars, playgroundVars } from '@/theming/vars.stylex';

const shimmer = stylex.keyframes({
  '0%': { backgroundPosition: '200% 0' },
  '100%': { backgroundPosition: '-200% 0' },
});

export function ClientOnly({
  children,
  fallback = null,
}: {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}) {
  const [hasMounted, setHasMounted] = useState(false);

  useEffect(() => {
    setHasMounted(true);
  }, []);

  if (!hasMounted) {
    // Render a fallback or null on the server and during initial client render
    return fallback;
    // You can also add a loading spinner or other placeholder here
    // return <div>Loading...</div>;
  }

  // Once mounted on the client, render the children
  return children;
}

const LazyPlayground = lazy(() => import('./index'));

export function Playground() {
  return (
    <ClientOnly fallback={<PlaygroundPlaceholder />}>
      <Suspense fallback={<PlaygroundPlaceholder />}>
        <QueryParamProvider adapter={WindowHistoryAdapter}>
          {/* TODO: Use concurrent mode to suspend while monaco boots up */}
          <LazyPlayground />
        </QueryParamProvider>
      </Suspense>
    </ClientOnly>
  );
}

function PlaygroundPlaceholder() {
  return (
    <div
      aria-busy="true"
      aria-label="Loading playground"
      role="status"
      {...stylex.props(styles.placeholder)}
    >
      <div {...stylex.props(styles.frame)}>
        <div {...stylex.props(styles.row)}>
          <div {...stylex.props(styles.column)}>
            <SkeletonEditorPanel lineCount={9} tabCount={3} />
            <SkeletonEditorPanel lineCount={4} tabCount={2} />
          </div>
          <div {...stylex.props(styles.column)}>
            <SkeletonPreviewPanel />
          </div>
        </div>
      </div>
    </div>
  );
}

function SkeletonEditorPanel({
  tabCount,
  lineCount,
}: {
  tabCount: number;
  lineCount: number;
}) {
  const lineWidths = [
    '80%',
    '55%',
    '70%',
    '40%',
    '85%',
    '60%',
    '50%',
    '75%',
    '35%',
  ];
  return (
    <div {...stylex.props(styles.panel)}>
      <div {...stylex.props(styles.panelHeader)}>
        {Array.from({ length: tabCount }).map((_, i) => (
          <div key={i} {...stylex.props(styles.tabPill, styles.shimmer)} />
        ))}
      </div>
      <div {...stylex.props(styles.panelBody)}>
        {Array.from({ length: lineCount }).map((_, i) => (
          <div
            key={i}
            style={{ width: lineWidths[i % lineWidths.length] }}
            {...stylex.props(styles.codeLine, styles.shimmer)}
          />
        ))}
      </div>
    </div>
  );
}

function SkeletonPreviewPanel() {
  return (
    <div {...stylex.props(styles.panel, styles.previewPanel)}>
      <div {...stylex.props(styles.panelHeader)}>
        <div {...stylex.props(styles.headerLabel, styles.shimmer)} />
      </div>
      <div {...stylex.props(styles.previewBody)}>
        <div {...stylex.props(styles.previewBlock, styles.shimmer)} />
      </div>
    </div>
  );
}

const styles = stylex.create({
  placeholder: {
    boxSizing: 'border-box',
    width: '100%',
    height: `calc(100dvh - ${vars['--fd-nav-height']})`,
    padding: 10,
    containerType: 'inline-size',
  },
  frame: {
    boxSizing: 'border-box',
    width: '100%',
    height: '100%',
    overflow: 'hidden',
    borderColor: playgroundVars['--pg-border'],
    borderStyle: 'solid',
    borderWidth: 1,
    borderRadius: 20,
  },
  row: {
    display: 'flex',
    flexDirection: { default: 'row', '@container (width < 768px)': 'column' },
    width: '100%',
    height: '100%',
  },
  column: {
    display: 'flex',
    flexGrow: 1,
    flexBasis: 0,
    flexDirection: 'column',
    minWidth: 0,
    minHeight: 0,
  },
  panel: {
    position: 'relative',
    display: 'flex',
    flexGrow: 1,
    flexShrink: 0,
    flexBasis: 42,
    flexDirection: 'column',
    overflow: 'hidden',
    backgroundColor: playgroundVars['--pg-panel-surface'],
    boxShadow: `0 0 0 1px ${playgroundVars['--pg-panel-shadow']}`,
  },
  previewPanel: {
    backgroundColor: playgroundVars['--pg-preview'],
  },
  panelHeader: {
    display: 'flex',
    flexShrink: 0,
    gap: 10,
    alignItems: 'center',
    width: '100%',
    height: 44,
    paddingInline: 16,
    backgroundColor: playgroundVars['--pg-header-surface'],
    borderBottomColor: vars['--color-fd-border'],
    borderBottomStyle: 'solid',
    borderBottomWidth: 1,
  },
  tabPill: {
    width: 64,
    height: 16,
    borderRadius: 4,
  },
  headerLabel: {
    width: 72,
    height: 14,
    borderRadius: 4,
  },
  panelBody: {
    display: 'flex',
    flexGrow: 1,
    flexDirection: 'column',
    gap: 12,
    padding: 18,
    overflow: 'hidden',
  },
  codeLine: {
    height: 10,
    borderRadius: 4,
  },
  previewBody: {
    display: 'flex',
    flexGrow: 1,
    padding: 18,
  },
  previewBlock: {
    flexGrow: 1,
    width: '100%',
    borderRadius: 8,
  },
  shimmer: {
    backgroundColor: vars['--color-fd-muted'],
    backgroundImage: `linear-gradient(90deg, transparent 0%, ${vars['--color-fd-accent']} 50%, transparent 100%)`,
    backgroundRepeat: 'no-repeat',
    backgroundSize: '200% 100%',
    animationName: shimmer,
    animationDuration: '1.6s',
    animationTimingFunction: 'linear',
    animationIterationCount: 'infinite',
  },
});
