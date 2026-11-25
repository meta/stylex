/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Locations of grafted nodes, traced to the Babel input file, and the comments
// they carry.

const { originalPositionFor, t, TraceMap } = require('./babel-deps');
const { COMMENT_FIELDS, fieldsOf, isNode } = require('./ast-equality');

function lineStarts(source) {
  const starts = [0];
  const breaks = /\r\n|\n|\r|\u2028|\u2029/g;
  for (let match; (match = breaks.exec(source)); )
    starts.push(match.index + match[0].length);
  return starts;
}

function locator(nativeMap, priorMap, source) {
  const native = new TraceMap(nativeMap);
  const prior = priorMap == null ? null : new TraceMap(priorMap);
  const starts = lineStarts(source);
  function trace(position) {
    let result = originalPositionFor(native, {
      line: position.line,
      column: position.column,
    });
    if (result.line == null) return null;
    if (prior != null)
      result = originalPositionFor(prior, {
        line: result.line,
        column: result.column,
      });
    if (result.line == null || starts[result.line - 1] == null) return null;
    return {
      line: result.line,
      column: result.column,
      index: starts[result.line - 1] + result.column,
    };
  }
  return function remap(node) {
    if (node.loc == null) return;
    const start = trace(node.loc.start);
    const end = trace(node.loc.end);
    if (start == null || end == null || end.index < start.index) {
      node.loc = null;
      delete node.start;
      delete node.end;
      return;
    }
    // No filename: the generator then uses its sourceFileName as the source.
    node.loc = { start, end, identifierName: node.loc.identifierName };
    node.start = start.index;
    node.end = end.index;
  };
}

// Remaps grafted subtrees and drops comment copies unchanged nodes still carry.
// Returns every attached comment, for `file.ast.comments`.
function settle(program, adopted, remap, visit, previousOf) {
  const kept = new Set();
  const keptStarts = new Set();
  const walk = (node) => {
    if (adopted.has(node)) return;
    if (visit != null) visit(node);
    for (const field of COMMENT_FIELDS) {
      const comments = node[field];
      if (comments != null)
        for (const comment of comments) {
          kept.add(comment);
          if (comment.start != null) keptStarts.add(comment.start);
        }
    }
    for (const field of fieldsOf(node)) {
      const value = node[field];
      if (Array.isArray(value)) {
        for (const item of value) if (isNode(item)) walk(item);
      } else if (isNode(value)) walk(value);
    }
  };
  walk(program);
  const all = new Set(kept);
  for (const root of adopted) {
    // Span the replaced node, since a traced end can collapse onto the start.
    const previous = previousOf.get(root);
    const spans = previous?.loc != null && previous.start != null;
    if (spans) {
      root.loc = {
        start: previous.loc.start,
        end: previous.loc.end,
        identifierName: root.loc?.identifierName,
      };
      root.start = previous.start;
      root.end = previous.end;
    }
    t.traverseFast(root, (node) => {
      if (visit != null) visit(node);
      if (!spans || node !== root) remap(node);
      for (const field of COMMENT_FIELDS) {
        if (node[field] == null) continue;
        const comments = [];
        for (const comment of node[field]) {
          if (!all.has(comment) && !kept.has(comment)) {
            remap(comment);
            if (comment.start != null && keptStarts.has(comment.start))
              continue;
            all.add(comment);
          }
          if (all.has(comment)) comments.push(comment);
        }
        if (comments.length > 0) node[field] = comments;
        else delete node[field];
      }
    });
  }
  return [...all].sort((x, y) => (x.start ?? Infinity) - (y.start ?? Infinity));
}

// Drops copies of kept statements' comments from adopted program statements.
function dropCommentCopies(body, adopted) {
  for (let index = 0; index < body.length; index++) {
    const statement = body[index];
    if (!adopted.has(statement)) continue;
    let previous = index - 1;
    while (previous >= 0 && adopted.has(body[previous])) previous--;
    let next = index + 1;
    while (next < body.length && adopted.has(body[next])) next++;
    const owned = [];
    for (const neighbour of [body[previous], body[next]])
      if (neighbour != null)
        for (const field of COMMENT_FIELDS)
          for (const comment of neighbour[field] || []) owned.push(comment);
    if (owned.length === 0) continue;
    for (const field of ['leadingComments', 'trailingComments']) {
      const comments = statement[field];
      if (comments == null) continue;
      const left = comments.filter(
        (comment) =>
          owned.includes(comment) ||
          !owned.some(
            (other) =>
              other.type === comment.type && other.value === comment.value,
          ),
      );
      if (left.length > 0) statement[field] = left;
      else delete statement[field];
    }
  }
}

// Removed statements share comments with siblings like Babel's `path.remove()`.
function shareRemovedComments(list, nodes) {
  for (let index = 0; index < list.length; index++) {
    const node = list[index];
    if (!nodes.has(node)) continue;
    const leading = node.leadingComments;
    const trailing = node.trailingComments;
    if (leading == null && trailing == null) continue;
    let back = index - 1;
    while (back >= 0 && nodes.has(list[back])) back--;
    shareComments(
      node,
      isNode(list[back]) ? list[back] : null,
      isNode(list[index + 1]) ? list[index + 1] : null,
    );
  }
}

// Babel's `shareCommentsWithSiblings`, for a node leaving between two others.
function shareComments(node, previous, next) {
  const without = (comments, existing) =>
    existing == null
      ? comments
      : comments.filter((comment) => !existing.includes(comment));
  const leading = node.leadingComments;
  const trailing = node.trailingComments;
  if (previous != null) {
    if (leading != null)
      t.addComments(
        previous,
        'trailing',
        without(leading, previous.trailingComments),
      );
    if (trailing != null && next == null)
      t.addComments(previous, 'trailing', trailing);
  }
  if (next != null) {
    if (trailing != null)
      t.addComments(next, 'leading', without(trailing, next.leadingComments));
    if (leading != null && previous == null)
      t.addComments(next, 'leading', leading);
  }
}

module.exports = {
  dropCommentCopies,
  locator,
  settle,
  shareComments,
  shareRemovedComments,
};
