/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import { useLayoutEffect, useRef, useState } from 'react';

const DEFAULT_LINE_LENGTH = 80;
const MIN_LINE_LENGTH = 24;
const DEFAULT_FONT_SIZE = 12;
const MONOSPACE_GLYPH_RATIO = 0.62;

export function estimateValueLineLength(
  availableWidth: number,
  fontSize: number,
  letterSpacing: number = 0,
): number {
  const glyphWidth = Math.max(
    1,
    fontSize * MONOSPACE_GLYPH_RATIO + letterSpacing,
  );
  return Math.max(MIN_LINE_LENGTH, Math.floor(availableWidth / glyphWidth));
}

export function useValueLineLength(enabled: boolean): {
  buttonRef: { current: ?HTMLButtonElement },
  maxLineLength: number,
} {
  const buttonRef = useRef<?HTMLButtonElement>(null);
  const [maxLineLength, setMaxLineLength] = useState(DEFAULT_LINE_LENGTH);

  useLayoutEffect(() => {
    if (!enabled) return;
    const button = buttonRef.current;
    const container = button?.parentElement;
    if (button == null || container == null) return;

    const updateLineLength = () => {
      const availableWidth = container.getBoundingClientRect().width;
      if (availableWidth <= 0) return;

      const computedStyle = window.getComputedStyle(button);
      const parsedFontSize = Number.parseFloat(computedStyle.fontSize);
      const parsedLetterSpacing = Number.parseFloat(
        computedStyle.letterSpacing,
      );
      const nextLineLength = estimateValueLineLength(
        availableWidth,
        Number.isFinite(parsedFontSize) ? parsedFontSize : DEFAULT_FONT_SIZE,
        Number.isFinite(parsedLetterSpacing) ? parsedLetterSpacing : 0,
      );
      setMaxLineLength((current) =>
        current === nextLineLength ? current : nextLineLength,
      );
    };

    updateLineLength();
    const ResizeObserverImpl = (globalThis as any).ResizeObserver;
    if (typeof ResizeObserverImpl !== 'function') return;

    const observer = new ResizeObserverImpl(updateLineLength);
    observer.observe(container);
    return () => observer.disconnect();
  }, [enabled]);

  return { buttonRef, maxLineLength };
}
