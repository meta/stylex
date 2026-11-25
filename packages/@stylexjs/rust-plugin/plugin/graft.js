/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Grafting a compiler result onto the live AST.

const { t } = require('./babel-deps');
const { equalTrees, fieldsOf, isNode, Shapes } = require('./ast-equality');

// The outer bindings a declaration statement declares, as one key, or null.
const DECLARATIONS = new Set([
  'VariableDeclaration',
  'FunctionDeclaration',
  'ClassDeclaration',
  'ExportNamedDeclaration',
  'ExportDefaultDeclaration',
  'VariableDeclarator',
]);
function declaredKey(node) {
  if (!isNode(node) || !DECLARATIONS.has(node.type)) return null;
  // Common shapes: a single named declarator, function or class, or its export.
  const declaration =
    node.type === 'ExportNamedDeclaration' ? node.declaration : node;
  if (declaration != null) {
    if (
      declaration.type === 'VariableDeclaration' &&
      declaration.declarations.length === 1 &&
      declaration.declarations[0].id.type === 'Identifier'
    )
      return declaration.declarations[0].id.name;
    if (
      declaration.type === 'VariableDeclarator' &&
      declaration.id.type === 'Identifier'
    )
      return declaration.id.name;
    if (
      (declaration.type === 'FunctionDeclaration' ||
        declaration.type === 'ClassDeclaration') &&
      declaration.id != null
    )
      return declaration.id.name;
  }
  const names = Object.keys(t.getBindingIdentifiers(node, false, true));
  return names.length === 0 ? null : names.join(',');
}

class Graft {
  constructor(shapes, record, defer = null, hold = null) {
    this.shapes = shapes;
    this.record = record;
    // Items the compile dropped that must stay until the traversal ends.
    this.defer = defer;
    this.deferred = [];
    // Nodes kept in place until visited; their replacements wait in `held`.
    this.hold = hold;
    this.held = [];
    this.edits = [];
    // Whether the live tree changed at all (held replacements wait outside).
    this.changed = false;
    this.adopted = new Set();
    // [replaced node, replacement] pairs.
    this.replaced = [];
    // Unmatched list runs, as { removed, added } (with `record`).
    this.regions = [];
    // The list each deferred item stays in.
    this.deferredIn = new Map();
    // Commented items removed outright, with their neighbours in the result.
    this.removed = [];
  }

  adopt(node, previous = null) {
    this.adopted.add(node);
    if (isNode(previous)) this.replaced.push([previous, node]);
    return node;
  }

  node(previous, next) {
    if (previous === next) return previous;
    if (!isNode(previous) || !isNode(next) || previous.type !== next.type)
      return isNode(next) ? this.replace(previous, next) : next;
    if (!this.shapes.sameScalars(previous, next))
      return this.replace(previous, next);
    // Compare children directly, so an edited subtree is never hashed whole.
    this.children(previous, next);
    return previous;
  }

  replace(previous, next) {
    this.adopt(next, previous);
    if (isNode(previous) && this.hold != null && this.hold(previous)) {
      this.held.push([previous, next]);
      return previous;
    }
    return next;
  }

  children(previous, next) {
    const fields = new Set([...fieldsOf(previous), ...fieldsOf(next)]);
    for (const field of fields) {
      const before = previous[field];
      const after = next[field];
      if (Array.isArray(before) || Array.isArray(after)) {
        this.list(previous, field, before || [], after || []);
      } else if (isNode(before) && isNode(after)) {
        const result = this.node(before, after);
        if (result !== before) this.set(previous, field, result);
      } else if (isNode(before) || isNode(after)) {
        this.set(
          previous,
          field,
          isNode(after) ? this.adopt(after, before) : after,
        );
      }
    }
  }

  set(parent, field, value) {
    this.changed = true;
    if (this.record)
      this.edits.push({ parent, field, before: parent[field], after: value });
    parent[field] = value;
  }

  list(parent, field, before, after) {
    const shapes = this.shapes;
    const pairs = matchItems(before, after, shapes);
    const result = [];
    let declared = null;
    const removed = [];
    const drop = (item) => {
      if (this.defer == null || !isNode(item) || !this.defer(item)) {
        if (isNode(item) && (item.leadingComments || item.trailingComments))
          removed.push({ item, at: result.length });
        return;
      }
      // A declaration re-emitted elsewhere in this list was rewritten, not removed.
      if (declared == null) {
        declared = new Map();
        // Outer bindings only: `function C(s)` does not declare the program's `s`.
        for (const other of after)
          if (isNode(other))
            for (const name of Object.keys(
              t.getBindingIdentifiers(other, false, true),
            ))
              if (!declared.has(name)) declared.set(name, other);
      }
      const rewritten = Object.keys(t.getBindingIdentifiers(item, false, true))
        .map((name) => declared.get(name))
        .find((other) => other != null);
      if (rewritten != null) {
        this.replaced.push([item, rewritten]);
        return;
      }
      this.deferred.push(item);
      this.deferredIn.set(item, { parent, field });
      result.push(item);
    };
    let i = 0;
    let j = 0;
    const gap = (iEnd, jEnd) => {
      // With `record`, a gap's dropped and adopted items form one region.
      const region = this.record ? { removed: [], added: [] } : null;
      const adopted = (next) => {
        if (region != null && isNode(next)) region.added.push(next);
        return isNode(next) ? this.adopt(next) : next;
      };
      const dropped = (item) => {
        if (region != null && isNode(item)) region.removed.push(item);
        drop(item);
      };
      // Unmatched runs: pair same-typed items in order and graft within them.
      const ahead = new Map();
      for (let k = j; k < jEnd; k++) {
        const type = isNode(after[k]) ? after[k].type : null;
        ahead.set(type, (ahead.get(type) || 0) + 1);
      }
      // Declared names of the items ahead in the run, counted on first need.
      let aheadKeys = null;
      const aheadOf = (key) => {
        if (aheadKeys == null) {
          aheadKeys = new Map();
          for (let k = j; k < jEnd; k++) {
            const other = declaredKey(after[k]);
            if (other != null)
              aheadKeys.set(other, (aheadKeys.get(other) || 0) + 1);
          }
        }
        return aheadKeys.get(key) || 0;
      };
      const passed = (next) => {
        const type = isNode(next) ? next.type : null;
        ahead.set(type, ahead.get(type) - 1);
        if (aheadKeys != null) {
          const key = declaredKey(next);
          if (key != null) aheadKeys.set(key, aheadKeys.get(key) - 1);
        }
      };
      // Declarations of different bindings never pair.
      let remaining = null;
      const remainingOf = (key) => {
        if (remaining == null) {
          remaining = new Map();
          for (let k = i; k < iEnd; k++) {
            const other = declaredKey(before[k]);
            if (other != null)
              remaining.set(other, (remaining.get(other) || 0) + 1);
          }
        }
        return remaining.get(key) || 0;
      };
      // An item leaving the run: it no longer counts as remaining.
      const left = (item) => {
        if (remaining != null) {
          const key = declaredKey(item);
          if (key != null) remaining.set(key, remaining.get(key) - 1);
        }
        return item;
      };
      while (i < iEnd && j < jEnd) {
        const previous = before[i];
        const next = after[j];
        const sameType =
          isNode(previous) && isNode(next) && previous.type === next.type;
        const previousKey = declaredKey(previous);
        const nextKey = sameType ? declaredKey(next) : null;
        if (previousKey != null && aheadOf(previousKey) === 0) {
          // No compiled declaration of these bindings ahead: removed.
          dropped(left(before[i++]));
        } else if (sameType && previousKey !== nextKey) {
          // Gone if the compiled declaration's source is later in the run; else new.
          if (
            nextKey != null &&
            previousKey == null &&
            remainingOf(nextKey) > 0
          )
            dropped(left(before[i++]));
          else {
            passed(after[j++]);
            result.push(adopted(next));
          }
        } else if (sameType) {
          passed(after[j++]);
          result.push(this.node(left(previous), next));
          i++;
        } else if (isNode(previous) && ahead.get(previous.type) > 0) {
          passed(after[j++]);
          result.push(adopted(next));
        } else if (
          isNode(previous) &&
          isNode(next) &&
          this.hold != null &&
          this.hold(previous)
        ) {
          // A definition replaced in place stays until visited, like any other.
          passed(after[j++]);
          result.push(this.replace(left(previous), next));
          i++;
        } else dropped(left(before[i++]));
      }
      for (; j < jEnd; j++) result.push(adopted(after[j]));
      while (i < iEnd) dropped(left(before[i++]));
      if (
        region != null &&
        region.removed.length > 0 &&
        region.added.length > 0
      )
        this.regions.push(region);
    };
    for (const [bi, ai] of pairs) {
      gap(bi, ai);
      result.push(before[bi]);
      i = bi + 1;
      j = ai + 1;
    }
    gap(before.length, after.length);
    for (const { item, at } of removed)
      this.removed.push({
        item,
        previous: isNode(result[at - 1]) ? result[at - 1] : null,
        next: isNode(result[at]) ? result[at] : null,
      });
    const unchanged =
      result.length === before.length &&
      result.every((item, index) => item === before[index]);
    if (unchanged) return;
    this.changed = true;
    if (this.record)
      this.edits.push({
        parent,
        field,
        list: true,
        before: before.slice(),
        after: result.slice(),
      });
    if (Array.isArray(parent[field]))
      parent[field].splice(0, parent[field].length, ...result);
    else parent[field] = result;
  }
}

// Grafts a compiled program onto a live Babel program in place, replacing only
// changed subtrees. `adopted` gets the replacements; `edits` feed `revert`.
function graftProgram(
  program,
  compiled,
  { record = false, defer = null, hold = null } = {},
) {
  const graft = new Graft(new Shapes(), record, defer, hold);
  graft.children(program, compiled);
  return {
    adopted: graft.adopted,
    edits: graft.edits,
    replaced: graft.replaced,
    deferred: graft.deferred,
    held: graft.held,
    regions: graft.regions,
    removed: graft.removed,
    deferredIn: graft.deferredIn,
    changed: graft.changed,
  };
}

// Index pairs of equal items in two lists, in order.
function matchItems(before, after, shapes) {
  const pairs = [];
  let start = 0;
  while (
    start < before.length &&
    start < after.length &&
    equalTrees(before[start], after[start], false)
  ) {
    pairs.push([start, start]);
    start++;
  }
  let endBefore = before.length;
  let endAfter = after.length;
  const suffix = [];
  while (
    endBefore > start &&
    endAfter > start &&
    equalTrees(before[endBefore - 1], after[endAfter - 1], false)
  ) {
    endBefore--;
    endAfter--;
    suffix.push([endBefore, endAfter]);
  }
  const n = endBefore - start;
  const m = endAfter - start;
  let aligned;
  if (n > 0 && m > 0 && n * m <= 64) {
    // A short changed run: compare items directly instead of hashing subtrees.
    const shallow = (item) => shapes.shallowKey(item);
    const keysAfter = after.slice(start, endAfter).map(shallow);
    const equal = [];
    for (let x = 0; x < n; x++) {
      const key = shallow(before[start + x]);
      equal.push(
        keysAfter.map(
          (other, y) =>
            other === key &&
            equalTrees(before[start + x], after[start + y], false),
        ),
      );
    }
    const table = new Uint32Array((n + 1) * (m + 1));
    for (let x = n - 1; x >= 0; x--)
      for (let y = m - 1; y >= 0; y--)
        table[x * (m + 1) + y] = equal[x][y]
          ? table[(x + 1) * (m + 1) + y + 1] + 1
          : Math.max(table[(x + 1) * (m + 1) + y], table[x * (m + 1) + y + 1]);
    let x = 0;
    let y = 0;
    while (x < n && y < m) {
      if (equal[x][y]) {
        pairs.push([start + x, start + y]);
        x++;
        y++;
      } else if (table[(x + 1) * (m + 1) + y] >= table[x * (m + 1) + y + 1])
        x++;
      else y++;
    }
  } else if (
    n > 0 &&
    n === m &&
    (aligned = positional(before, after, start, n, shapes)) != null
  ) {
    // Replaced in place: the aligned pairs are the whole common subsequence.
    for (const x of aligned) pairs.push([x, x]);
  } else if (n > 0 && m > 0) {
    const middleBefore = before.slice(start, endBefore);
    const middleAfter = after.slice(start, endAfter);
    const [keysBefore, keysAfter] = shapes.listKeys(middleBefore, middleAfter);
    for (const [x, y] of matchLists(keysBefore, keysAfter))
      if (shapes.same(middleBefore[x], middleAfter[y]))
        pairs.push([start + x, start + y]);
  }
  return pairs.concat(suffix.reverse());
}

function equalItem(a, b, shapes) {
  return (
    shapes.shallowKey(a) === shapes.shallowKey(b) && equalTrees(a, b, false)
  );
}

// Equal positions of two same-length runs if that matching is optimal, or null.
function positional(before, after, start, length, shapes) {
  const equal = [];
  const unmatchedBefore = new Set();
  const unmatchedAfter = [];
  for (let x = start; x < start + length; x++) {
    if (equalItem(before[x], after[x], shapes)) equal.push(x);
    else {
      unmatchedBefore.add(shapes.shallowKey(before[x]));
      unmatchedAfter.push(shapes.shallowKey(after[x]));
    }
  }
  return unmatchedAfter.some((key) => unmatchedBefore.has(key)) ? null : equal;
}

// Longest common subsequence of two key lists, as index pairs.
function matchLists(a, b) {
  const pairs = [];
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) {
    pairs.push([start, start]);
    start++;
  }
  let endA = a.length;
  let endB = b.length;
  const suffix = [];
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
    endA--;
    endB--;
    suffix.push([endA, endB]);
  }
  const n = endA - start;
  const m = endB - start;
  if (n > 0 && m > 0) {
    if (n * m <= 1 << 20) {
      const table = new Uint32Array((n + 1) * (m + 1));
      for (let x = n - 1; x >= 0; x--)
        for (let y = m - 1; y >= 0; y--)
          table[x * (m + 1) + y] =
            a[start + x] === b[start + y]
              ? table[(x + 1) * (m + 1) + y + 1] + 1
              : Math.max(
                  table[(x + 1) * (m + 1) + y],
                  table[x * (m + 1) + y + 1],
                );
      let x = 0;
      let y = 0;
      while (x < n && y < m) {
        if (a[start + x] === b[start + y]) {
          pairs.push([start + x, start + y]);
          x++;
          y++;
        } else if (table[(x + 1) * (m + 1) + y] >= table[x * (m + 1) + y + 1])
          x++;
        else y++;
      }
    } else {
      // Very long changed runs: match each key to its next occurrence.
      const positions = new Map();
      for (let y = start; y < endB; y++) {
        if (!positions.has(b[y])) positions.set(b[y], []);
        positions.get(b[y]).push(y);
      }
      const next = new Map();
      let last = start - 1;
      for (let x = start; x < endA; x++) {
        const candidates = positions.get(a[x]);
        if (candidates == null) continue;
        let cursor = next.get(a[x]) || 0;
        while (cursor < candidates.length && candidates[cursor] <= last)
          cursor++;
        if (cursor < candidates.length) {
          last = candidates[cursor++];
          pairs.push([x, last]);
        }
        next.set(a[x], cursor);
      }
    }
  }
  return pairs.concat(suffix.reverse());
}

// Undoes recorded edits that are still in place, newest first.
// Items other plugins added since keep their relative positions.
function revert(edits) {
  const restored = new Set();
  for (let index = edits.length - 1; index >= 0; index--) {
    const edit = edits[index];
    const { parent, field } = edit;
    if (!edit.list) {
      if (parent[field] === edit.after) {
        parent[field] = edit.before;
        if (isNode(edit.before)) restored.add(edit.before);
      }
      continue;
    }
    const current = parent[field];
    if (!Array.isArray(current)) continue;
    const kept = new Set(edit.before);
    const ours = new Set(edit.after);
    const inserted = new Set(edit.after.filter((item) => !kept.has(item)));
    const present = new Set(current);
    // Only items this graft removed come back; later plugins' removals stand.
    const removed = edit.before.filter(
      (item) => !present.has(item) && !ours.has(item),
    );
    const order = new Map(
      edit.before.map((item, position) => [item, position]),
    );
    const result = [];
    let next = 0;
    const flush = (limit) => {
      while (next < removed.length && order.get(removed[next]) < limit) {
        result.push(removed[next]);
        if (isNode(removed[next])) restored.add(removed[next]);
        next++;
      }
    };
    for (const item of current) {
      if (inserted.has(item)) continue;
      if (order.has(item)) flush(order.get(item));
      result.push(item);
    }
    flush(Infinity);
    current.splice(0, current.length, ...result);
  }
  return restored;
}

module.exports = {
  graftProgram,
  revert,
};
