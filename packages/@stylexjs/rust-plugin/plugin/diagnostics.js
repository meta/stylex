/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Diagnostics.

const { codeFrameColumns } = require('@babel/code-frame');
const { generate, originalPositionFor, t, TraceMap } = require('./babel-deps');

const DIAGNOSTIC_PREFIX = 'STYLEX_DIAGNOSTIC:';

function sourcePosition(source, offset) {
  // SWC reports UTF-8 offsets. Babel's columns count UTF-16 code units.
  const prefix = Buffer.from(source, 'utf8')
    .subarray(0, offset)
    .toString('utf8');
  const lines = prefix.split(/\r\n|\n|\r|\u2028|\u2029/);
  return { line: lines.length, column: lines[lines.length - 1].length + 1 };
}

function diagnosticFromError(error) {
  if (error != null && error.stylexDiagnostic != null)
    return error.stylexDiagnostic;
  if (error != null && error.diagnostic != null) return error.diagnostic;
  const message = error != null ? error.message : null;
  if (typeof message !== 'string' || !message.startsWith(DIAGNOSTIC_PREFIX))
    return null;
  try {
    const diagnostic = JSON.parse(message.slice(DIAGNOSTIC_PREFIX.length));
    if (
      typeof diagnostic.message === 'string' &&
      ['Error', 'SyntaxError'].includes(diagnostic.name)
    )
      return diagnostic;
  } catch (_) {
    // A malformed native envelope is a native error, not a StyleX diagnostic.
  }
  return null;
}

// A position in Babel-generated text, traced back to the input file.
function tracedPosition(map, position) {
  const traced = originalPositionFor(map, {
    line: position.line,
    column: position.column - 1,
  });
  return traced.line == null
    ? null
    : { line: traced.line, column: traced.column + 1 };
}

function diagnosticError(diagnostic, input, state) {
  let message = diagnostic.message;
  const range = diagnostic.range;
  if (
    range != null &&
    Number.isInteger(range.start) &&
    Number.isInteger(range.end)
  ) {
    let source = input.code;
    let start = sourcePosition(source, range.start);
    let end = sourcePosition(source, range.end);
    if (input.map != null) {
      // The compiler read Babel-generated text; frame the user's file.
      const map = new TraceMap(input.map);
      const originalStart = tracedPosition(map, start);
      if (originalStart != null) {
        const covered = Buffer.from(source, 'utf8')
          .subarray(range.start, range.end)
          .toString('utf8');
        const startNode = originalNodeAt(state, originalStart, covered);
        source = state.file.code;
        start = originalStart;
        end =
          startNode != null && startNode.loc != null
            ? {
                line: startNode.loc.end.line,
                column: startNode.loc.end.column + 1,
              }
            : start;
      }
    }
    // Like Babel's buildCodeFrameError, a multiline node frames only its start.
    message +=
      '\n' +
      codeFrameColumns(
        source,
        {
          start,
          end: start.line === end.line ? end : undefined,
        },
        { highlightCode: state.file.opts.highlightCode !== false },
      );
  }
  const Constructor = diagnostic.name === 'SyntaxError' ? SyntaxError : Error;
  const error = new Constructor(message);
  error.code =
    diagnostic.kind === 'parse' ? 'BABEL_PARSE_ERROR' : 'BABEL_TRANSFORM_ERROR';
  return error;
}

// The outermost input node at `position` whose Babel printing spans `covered`.
function originalNodeAt(state, position, covered) {
  const candidates = [];
  t.traverseFast(state.file.ast.program, (node) => {
    const loc = node.loc;
    if (
      loc != null &&
      loc.start.line === position.line &&
      loc.start.column === position.column - 1
    )
      candidates.push(node);
  });
  const text = covered.replace(/\s+/g, '');
  return (
    candidates.find(
      (node) => generate(node).code.replace(/\s+/g, '') === text,
    ) ??
    candidates[0] ??
    null
  );
}

function metadataRules(rules) {
  return rules.map(([name, fields, priority]) => {
    const output =
      fields.constKey != null && fields.constVal != null
        ? {
            constKey: fields.constKey,
            constVal: fields.constVal,
            ltr: fields.ltr,
            rtl: fields.rtl ?? null,
          }
        : { ltr: fields.ltr, rtl: fields.rtl ?? null };
    return [name, output, priority];
  });
}

function logDiagnostic(method, message) {
  if (method === 'warn') {
    console.warn(message);
    return;
  }
  // Current Babel uses two console arguments for its package-prefixed errors.
  const match = /^\[([^\]]+)\] ([\s\S]*)$/.exec(message);
  if (match != null) console[method](`[${match[1]}]`, match[2]);
  else console[method](message);
}

// Items of `next` that are not accounted for by `previous` (as a multiset).
function multisetDifference(next, previous, key = JSON.stringify) {
  const counts = new Map();
  for (const item of previous) {
    const k = key(item);
    counts.set(k, (counts.get(k) || 0) + 1);
  }
  return next.filter((item) => {
    const k = key(item);
    const count = counts.get(k) || 0;
    if (count === 0) return true;
    counts.set(k, count - 1);
    return false;
  });
}

module.exports = {
  diagnosticError,
  diagnosticFromError,
  logDiagnostic,
  metadataRules,
  multisetDifference,
};
