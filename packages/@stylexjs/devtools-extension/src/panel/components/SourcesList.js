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
import type { StylexSource } from '../../types';
import { SourceRow } from './SourceRow';

export function SourcesList({
  revision,
  sources,
}: {
  revision: number,
  sources: $ReadOnlyArray<StylexSource>,
}): React.Node {
  return (
    <div {...stylex.props(styles.sourcesList)}>
      {sources.map((src, index) => (
        <SourceRow
          key={src.raw ?? `${src.file}:${String(src.line ?? '')}:${index}`}
          revision={revision}
          source={src}
        />
      ))}
    </div>
  );
}

const styles = stylex.create({
  sourcesList: {
    width: '100%',
    display: 'flex',
    flexDirection: 'column',
    gap: 6,
  },
});
