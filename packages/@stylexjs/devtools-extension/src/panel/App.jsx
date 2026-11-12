/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as React from 'react';
import * as stylex from '@stylexjs/stylex';

import { Button } from './components/Button';
import { CopyMetadataButton } from './components/CopyMetadataButton';
import Logo from './components/Logo';
import { MatchedStyles } from './components/MatchedStyles';
import { OverridesSection } from './components/OverridesSection';
import { Section } from './components/Section';
import { SourcesList } from './components/SourcesList';
import { useDebugData } from './hooks/useDebugData';
import { colors } from './theme.stylex';

export function App(): React.Node {
  const { data, error, loading, mutate, refresh, revision } = useDebugData();

  return (
    <div {...stylex.props(styles.root)}>
      <header {...stylex.props(styles.header)}>
        <Logo xstyle={styles.logo} />
        <div {...stylex.props(styles.actions)}>
          {data?.selectionState === 'element' ? (
            <>
              <span {...stylex.props(styles.tag)}>{data.element.tagName}</span>
              <CopyMetadataButton data={data} />
            </>
          ) : null}
          <Button onClick={refresh}>Refresh</Button>
        </div>
      </header>

      {error != null ? (
        <div {...stylex.props(styles.error)} role="alert">
          {error}
        </div>
      ) : null}

      {loading && data == null ? <EmptyState>Loading…</EmptyState> : null}
      {!loading && data == null ? (
        <EmptyState>StyleX inspection is unavailable.</EmptyState>
      ) : null}
      {data?.selectionState === 'none' ? (
        <EmptyState>No element selected.</EmptyState>
      ) : null}
      {data?.selectionState === 'non-element' ? (
        <EmptyState>The selected node is not an element.</EmptyState>
      ) : null}

      {data?.selectionState === 'element' ? (
        <>
          {data.warnings.length > 0 ? (
            <div {...stylex.props(styles.warnings)}>
              {data.warnings.map((warning) => (
                <div key={warning.code}>{warning.message}</div>
              ))}
            </div>
          ) : null}
          {data.sources.length > 0 ? (
            <Section title="Sources">
              <SourcesList revision={revision} sources={data.sources} />
            </Section>
          ) : null}
          <Section title="Matched Styles">
            <MatchedStyles
              data={data}
              key={`matched:${data.selectionId}`}
              onMutate={mutate}
            />
            <OverridesSection
              data={data}
              key={`overrides:${data.selectionId}`}
              onMutate={mutate}
            />
          </Section>
        </>
      ) : null}
    </div>
  );
}

function EmptyState({ children }: { children: React.Node }): React.Node {
  return <div {...stylex.props(styles.emptyState)}>{children}</div>;
}

const styles = stylex.create({
  root: {
    backgroundColor: colors.bg,
    boxSizing: 'border-box',
    color: colors.textPrimary,
    display: 'flex',
    flexDirection: 'column',
    fontFamily:
      'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif',
    fontSize: 12,
    minHeight: '100%',
  },
  header: {
    alignItems: 'center',
    borderBottomColor: colors.border,
    borderBottomStyle: 'solid',
    borderBottomWidth: 1,
    display: 'flex',
    justifyContent: 'space-between',
    minHeight: 44,
    padding: 8,
  },
  logo: { color: colors.textPrimary, height: 28 },
  actions: {
    alignItems: 'center',
    display: 'flex',
    flexWrap: 'wrap',
    gap: 7,
    justifyContent: 'flex-end',
    minWidth: 0,
  },
  tag: {
    backgroundColor: colors.bgRaised,
    borderColor: colors.border,
    borderRadius: 7,
    borderStyle: 'solid',
    borderWidth: 1,
    color: colors.textMuted,
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    maxWidth: 100,
    overflow: 'hidden',
    paddingBlock: 2,
    paddingInline: 6,
    textOverflow: 'ellipsis',
  },
  error: {
    color: 'light-dark(#b42318, #ff8a80)',
    overflowWrap: 'anywhere',
    paddingBlock: 7,
    paddingInline: 8,
  },
  warnings: {
    color: 'light-dark(#7a4d00, #f0c36d)',
    display: 'grid',
    gap: 3,
    paddingBlock: 7,
    paddingInline: 8,
  },
  emptyState: {
    alignItems: 'center',
    color: colors.textMuted,
    display: 'flex',
    flex: 1,
    justifyContent: 'center',
    minHeight: 120,
    padding: 16,
    textAlign: 'center',
  },
});
