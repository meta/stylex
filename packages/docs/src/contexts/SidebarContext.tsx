/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use client';

import { usePathname } from 'fumadocs-core/framework';
import { createContext, useId, useMemo, useState } from 'react';

export const SidebarContext = createContext<
  [
    null | boolean,
    (_val: ((_old: null | boolean) => null | boolean) | null | boolean) => void,
    string,
  ]
>([true, () => {}, '']);

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(null);
  const sidebarId = useId();

  const pathname = usePathname();
  const [prevPathname, setPrevPathname] = useState(pathname);
  if (pathname !== prevPathname) {
    setPrevPathname(pathname);
    if (open === true) setOpen(null);
  }

  const value = useMemo(() => [open, setOpen, sidebarId], [open, sidebarId]);

  return <SidebarContext value={value as any}>{children}</SidebarContext>;
}
