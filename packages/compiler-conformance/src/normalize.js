/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const path = require('node:path');

const FIXTURE_ROOT_TOKEN = '<FIXTURE_ROOT>';
const REPO_ROOT_TOKEN = '<REPO_ROOT>';

// The bracketed tag implementations put in front of non-fatal diagnostics,
// e.g. `[@stylexjs/babel-plugin] `. The tag names the implementation, so it is
// collapsed to a single neutral tag before comparing. Tags never contain
// whitespace, which keeps a message that merely opens with a bracket — say
// `[1, 2] is not a valid value` — from being mistaken for one.
const IMPLEMENTATION_TAG = /^\[[^\]\s]+\]\s*/;

// A source excerpt appended to a diagnostic by `@babel/code-frame` and its
// equivalents:
//
//   ` 3 | export const styles = stylex.create(1);`
//   `   |                       ^^^^^^^^^^^^^^^^`
//
// The exact rendering is implementation-specific, so it is dropped.
const CODE_FRAME_LINE = /^\s*(?:>\s*)?(?:\d+\s*)?\|(?:\s|$)/;

// Terminal escape sequences. A diagnostic is syntax-highlighted whenever the
// implementation thinks the environment supports color, and that decision is
// made from the environment rather than from the code under test: Babel's code
// frame colorizes whenever `CI` is set, so the very same error arrives plain on
// a developer's machine and colorized on CI. Color is presentation, so it is
// removed before anything else — otherwise the escapes also hide the code frame
// from the matcher above, and hide absolute paths from the path tokenizer.
// eslint-disable-next-line no-control-regex -- ESC is what this matches.
const ANSI_ESCAPE = /\u001B\[[0-9;]*[A-Za-z]/g;

function splitJoin(value, search, replacement) {
  return search === '' ? value : value.split(search).join(replacement);
}

function toPosixPath(value) {
  return value.split(path.win32.sep).join(path.posix.sep);
}

/**
 * Replaces absolute filesystem paths with stable tokens. The fixture directory
 * is replaced first because it is nested inside the repository root.
 */
function normalizePaths(value, context) {
  let output = String(value).split('\r\n').join('\n');
  const roots = [
    [context.fixtureDir, FIXTURE_ROOT_TOKEN],
    [context.repoRoot, REPO_ROOT_TOKEN],
  ];
  for (const [root, token] of roots) {
    if (root == null) {
      continue;
    }
    const absolute = path.resolve(root);
    output = splitJoin(output, absolute, token);
    output = splitJoin(output, toPosixPath(absolute), token);
  }
  return output;
}

function stripAnsi(value) {
  return String(value).replace(ANSI_ESCAPE, '');
}

function stripCodeFrame(message) {
  const lines = message.split('\n');
  const frameStart = lines.findIndex((line) => CODE_FRAME_LINE.test(line));
  const kept = frameStart === -1 ? lines : lines.slice(0, frameStart);
  return kept.join('\n');
}

/**
 * Normalizes a single warning, error or fatal message so that the same
 * behavior reported by two implementations compares equal:
 *
 * - terminal color escapes are removed
 * - absolute paths become `<FIXTURE_ROOT>` / `<REPO_ROOT>`
 * - a leading `<FIXTURE_ROOT>/<entry>: ` location prefix is dropped
 * - a trailing source excerpt (code frame) is dropped
 * - a leading `[implementation]` tag becomes `[stylex]`
 */
function normalizeDiagnostic(message, context) {
  let output = stripCodeFrame(normalizePaths(stripAnsi(message), context));

  const locationPrefix = `${FIXTURE_ROOT_TOKEN}/${context.entry}: `;
  if (output.startsWith(locationPrefix)) {
    output = output.slice(locationPrefix.length);
  }

  return output.replace(IMPLEMENTATION_TAG, '[stylex] ').trim();
}

/**
 * Normalizes generated CSS. Rule order and rule text are part of the contract,
 * so only line endings and trailing whitespace are touched.
 */
function normalizeCss(css, context) {
  return normalizePaths(css ?? '', context)
    .split('\n')
    .map((line) => line.replace(/\s+$/, ''))
    .join('\n')
    .trim();
}

/**
 * Normalizes structured data (StyleX metadata). Object keys are sorted because
 * their order carries no meaning in JSON; array order is preserved because rule
 * order is part of the contract.
 */
function normalizeData(value, context) {
  if (Array.isArray(value)) {
    return value.map((item) => normalizeData(item, context));
  }
  if (value != null && typeof value === 'object') {
    const entries = Object.keys(value)
      .sort()
      .map((key) => [key, normalizeData(value[key], context)]);
    return Object.fromEntries(entries);
  }
  return typeof value === 'string' ? normalizePaths(value, context) : value;
}

module.exports = {
  FIXTURE_ROOT_TOKEN,
  REPO_ROOT_TOKEN,
  normalizeCss,
  normalizeData,
  normalizeDiagnostic,
  normalizePaths,
};
