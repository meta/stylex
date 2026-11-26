/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

export const canUseDOM: boolean = !!(
  typeof window !== 'undefined' &&
  window.document &&
  window.document.createElement
);

/**
 * Adds :not(#\#) to selectors to increase their specificity.
 * This is used to polyfill @layer
 */
export function addSpecificityLevel(cssText: string, index: number): string {
  if (cssText.startsWith('@keyframes')) {
    return cssText;
  }

  const pseudoSelector = Array.from({ length: index })
    .map(() => ':not(#\\#)')
    .join('');

  if (index === 0) {
    return cssText;
  }

  // Radius fallbacks contain several nested rules in one insertable block, so
  // every selector in the block needs the specificity bump.
  if (cssText.includes('@media all{')) {
    let result = '';
    let cursor = 0;
    while (cursor < cssText.length) {
      const openCurly = cssText.indexOf('{', cursor);
      if (openCurly === -1) {
        result += cssText.slice(cursor);
        break;
      }

      const prelude = cssText.slice(cursor, openCurly);
      const boundary = Math.max(
        prelude.lastIndexOf('{'),
        prelude.lastIndexOf('}'),
      );
      const beforeSelector = prelude.slice(0, boundary + 1);
      const selectorText = prelude.slice(boundary + 1);

      if (selectorText.trimStart().startsWith('@')) {
        result += `${prelude}{`;
      } else {
        const pseudoIndex = selectorText.includes('::')
          ? selectorText.indexOf('::')
          : selectorText.length;
        result +=
          beforeSelector +
          selectorText.slice(0, pseudoIndex) +
          pseudoSelector +
          selectorText.slice(pseudoIndex) +
          '{';
      }

      cursor = openCurly + 1;
    }
    return result;
  }

  const lastOpenCurly = cssText.includes('::')
    ? cssText.indexOf('::')
    : cssText.lastIndexOf('{');
  let beforeCurly = cssText.slice(0, lastOpenCurly);
  const afterCurly = cssText.slice(lastOpenCurly);

  if (index > 0) {
    beforeCurly = beforeCurly.trimEnd();
  }

  return `${beforeCurly}${pseudoSelector}${afterCurly}`;
}
