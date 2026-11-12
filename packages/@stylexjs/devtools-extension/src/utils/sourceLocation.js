/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

import type { StylexSource } from '../types';

const URL_PREFIX = /^[a-z][a-z\d+.-]*:\/\//i;
const WINDOWS_DRIVE_PREFIX = /^[a-z]:[\\/]/i;

export function formatSourceLocation(source: StylexSource): string {
  return source.line == null ? source.file : `${source.file}:${source.line}`;
}

export function formatCopyableSourceLocation(source: StylexSource): string {
  const file = stripProjectPrefix(source.file);
  return formatSourceLocation({ ...source, file });
}

function stripProjectPrefix(file: string): string {
  if (URL_PREFIX.test(file) || WINDOWS_DRIVE_PREFIX.test(file)) {
    return file;
  }

  const separator = file.indexOf(':');
  if (separator === -1) {
    return file;
  }

  const path = file.slice(separator + 1);
  return path === '' ? file : path;
}
