/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import type { ReactNode } from 'react';
import { Provider } from '@/components/provider';
import '@/styles/globals.css';
import DevStyleXHMR from '@/components/DevStyleXHMR';
import { SidebarProvider } from '@/contexts/SidebarContext';
import coverImageUrl from '@/static/img/stylex-cover-photo.png';

const faviconUrl = '/favicon.svg';

const DEFAULT_TITLE = 'StyleX — The styling system for ambitious interfaces';
const DEFAULT_DESCRIPTION = 'The styling system that powers Meta.';

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <head>
        <meta charSet="utf-8" />
        <meta content="width=device-width, initial-scale=1" name="viewport" />
        <meta content={DEFAULT_DESCRIPTION} name="description" />
        <meta content={DEFAULT_TITLE} property="og:title" />
        <meta content={DEFAULT_DESCRIPTION} property="og:description" />
        <meta content="website" property="og:type" />
        <meta content={coverImageUrl} property="og:image" />
        <meta content="summary_large_image" name="twitter:card" />
        <meta content={DEFAULT_TITLE} name="twitter:title" />
        <meta content={DEFAULT_DESCRIPTION} name="twitter:description" />
        <meta content={coverImageUrl} name="twitter:image" />
        <link href={faviconUrl} rel="icon" sizes="any" />
        <link href={faviconUrl} rel="icon" type="image/svg+xml" />
        <link href={faviconUrl} rel="shortcut icon" />
      </head>
      <DevStyleXHMR />
      <Provider>
        <SidebarProvider>{children}</SidebarProvider>
      </Provider>
    </>
  );
}
