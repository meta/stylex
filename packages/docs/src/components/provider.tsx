/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use client';

import { RootProvider } from 'fumadocs-ui/provider/waku';
import type { ReactNode } from 'react';
import { SearchDialog } from './search-dialog';

export function Provider({ children }: { children: ReactNode }) {
  return (
    <RootProvider search={{ SearchDialog }} theme={{ defaultTheme: 'dark' }}>
      {children}
    </RootProvider>
  );
}
