/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import * as React from 'react';
import { useEffect, useState } from 'react';
import * as stylex from '@stylexjs/stylex';

import type { SourcePreview, StylexSource } from '../../types';
import { devtoolsBridge } from '../../devtools/bridge';
import { copyText } from '../../utils/clipboard';
import {
  formatCopyableSourceLocation,
  formatSourceLocation,
} from '../../utils/sourceLocation';
import { colors } from '../theme.stylex';
import { EyeIcon } from './EyeIcon';

export function SourceRow({
  revision,
  source,
}: {
  revision: number,
  source: StylexSource,
}): React.Node {
  const [previewOpen, setPreviewOpen] = useState(false);
  const [preview, setPreview] = useState<?SourcePreview>(null);
  const [error, setError] = useState<?string>(null);
  const [copyStatus, setCopyStatus] = useState<'idle' | 'copied' | 'failed'>(
    'idle',
  );

  useEffect(() => {
    if (!previewOpen || !devtoolsBridge.capabilities.sourcePreview) return;
    let cancelled = false;
    setPreview(null);
    setError(null);
    devtoolsBridge.getSourcePreview(source.file, source.line).then(
      (nextPreview) => {
        if (!cancelled) setPreview(nextPreview);
      },
      (caught) => {
        if (!cancelled) {
          setError(
            caught instanceof Error
              ? caught.message
              : 'Could not load the source preview.',
          );
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [previewOpen, revision, source.file, source.line]);

  useEffect(() => {
    if (copyStatus === 'idle') return;
    const timeoutId = window.setTimeout(() => setCopyStatus('idle'), 1600);
    return () => window.clearTimeout(timeoutId);
  }, [copyStatus]);

  const location = formatSourceLocation(source);
  const copyLocation = formatCopyableSourceLocation(source);
  const sourceControl = devtoolsBridge.capabilities.openSource ? (
    <button
      {...stylex.props(styles.pathButton)}
      onClick={() => {
        setError(null);
        devtoolsBridge
          .openSource(source.file, source.line)
          .catch((caught) =>
            setError(
              caught instanceof Error
                ? caught.message
                : 'Could not open source.',
            ),
          );
      }}
      title={source.raw}
      type="button"
    >
      {location}
    </button>
  ) : (
    <button
      {...stylex.props(styles.pathButton, styles.copyPathButton)}
      aria-label={
        copyStatus === 'copied'
          ? `Copied source location ${copyLocation}`
          : copyStatus === 'failed'
            ? `Copy failed for source location ${copyLocation}`
            : `Copy source location ${copyLocation}`
      }
      onClick={async () => {
        setCopyStatus('idle');
        const copied = await copyText(copyLocation);
        setCopyStatus(copied ? 'copied' : 'failed');
      }}
      title={
        copyStatus === 'copied'
          ? `Copied: ${copyLocation}`
          : copyStatus === 'failed'
            ? `Could not copy: ${copyLocation}`
            : `Copy: ${copyLocation}`
      }
      type="button"
    >
      {location}
    </button>
  );

  return (
    <div {...stylex.props(styles.root)}>
      <div {...stylex.props(styles.row)}>
        {devtoolsBridge.capabilities.sourcePreview ? (
          <button
            {...stylex.props(
              styles.iconButton,
              previewOpen && styles.iconButtonActive,
            )}
            aria-label={previewOpen ? 'Hide source preview' : 'Preview source'}
            onClick={() => setPreviewOpen((open) => !open)}
            title={previewOpen ? 'Hide source preview' : 'Preview source'}
            type="button"
          >
            <EyeIcon xstyle={styles.icon} />
          </button>
        ) : null}
        {sourceControl}
        {!devtoolsBridge.capabilities.openSource && copyStatus !== 'idle' ? (
          <span
            {...stylex.props(
              styles.copyStatus,
              copyStatus === 'failed' && styles.copyStatusFailed,
            )}
            role="status"
          >
            {copyStatus === 'copied' ? 'Copied' : 'Copy failed'}
          </span>
        ) : null}
      </div>
      {error != null ? (
        <div {...stylex.props(styles.error)} role="alert">
          {error}
        </div>
      ) : null}
      {previewOpen ? (
        <pre {...stylex.props(styles.preview)}>
          {preview?.snippet ?? (error == null ? 'Loading…' : '')}
        </pre>
      ) : null}
    </div>
  );
}

const styles = stylex.create({
  root: { display: 'grid', gap: 5, minWidth: 0 },
  row: { alignItems: 'center', display: 'flex', gap: 5, minWidth: 0 },
  icon: { height: 16, width: 16 },
  iconButton: {
    appearance: 'none',
    backgroundColor: { default: 'transparent', ':hover': colors.bgRaised },
    borderStyle: 'none',
    color: colors.textPrimary,
    cursor: 'pointer',
    display: 'flex',
    flexShrink: 0,
    padding: 4,
  },
  iconButtonActive: { color: colors.textAccent },
  pathButton: {
    appearance: 'none',
    backgroundColor: 'transparent',
    borderStyle: 'none',
    color: {
      default: colors.textPrimary,
      ':focus-visible': colors.textAccent,
      ':hover': colors.textAccent,
    },
    cursor: 'pointer',
    flexBasis: 0,
    flexGrow: 1,
    flexShrink: 1,
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    minWidth: 0,
    overflowWrap: 'anywhere',
    padding: 0,
    textAlign: 'left',
  },
  copyPathButton: { flexBasis: 'auto', flexGrow: 0 },
  copyStatus: {
    color: colors.secondaryAccent,
    flexShrink: 0,
    fontSize: 11,
    whiteSpace: 'nowrap',
  },
  copyStatusFailed: { color: 'light-dark(#b42318, #ff8a80)' },
  error: { color: 'light-dark(#b42318, #ff8a80)', overflowWrap: 'anywhere' },
  preview: {
    backgroundColor: colors.bgRaised,
    borderColor: colors.border,
    borderRadius: 6,
    borderStyle: 'solid',
    borderWidth: 1,
    fontFamily:
      'ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace',
    margin: 0,
    minHeight: 32,
    overflow: 'auto',
    padding: 8,
    whiteSpace: 'pre',
  },
});
