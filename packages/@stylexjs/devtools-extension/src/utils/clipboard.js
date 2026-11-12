/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

export async function copyText(text: string): Promise<boolean> {
  try {
    const clipboard = navigator.clipboard;
    const writeText = (clipboard as any)?.writeText;
    if (clipboard != null && typeof writeText === 'function') {
      await writeText.call(clipboard, text);
      return true;
    }
  } catch {}

  let textarea = null;
  try {
    textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    const body = document.body;
    if (body == null) return false;
    body.appendChild(textarea);
    textarea.focus();
    textarea.select();
    return document.execCommand('copy');
  } catch {
    return false;
  } finally {
    textarea?.remove();
  }
}
