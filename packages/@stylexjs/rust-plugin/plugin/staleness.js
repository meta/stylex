/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Compiled consumers whose definitions another plugin edited afterwards.

const { t } = require('./babel-deps');
const { isNode, matchesShapeRecord } = require('./ast-equality');
const { revert } = require('./graft');

function unchangedInputs(record, s) {
  return (
    matchesShapeRecord(record.call, record.shape) &&
    record.dependencies.every((declaration) =>
      matchesShapeRecord(declaration, s.dependencyShapes.get(declaration)),
    )
  );
}

// Undoes the stale entry output before other plugins visit it; exit recompiles.
function markStale(s) {
  if (s.stale) return;
  s.stale = true;
  s.staleConsumers = staleConsumers(s);
  for (const node of revert(s.edits)) s.restored.add(node);
  s.edits = [];
}

// Compiled consumers (props, attrs) that read a definition whose input changed.
function staleConsumers(s) {
  if ((s.consumerCalls || []).length === 0 || s.references == null) return [];
  // The declarator each recorded definition initializes.
  const calls = new Map(s.inputs.map((record) => [record.call, record]));
  const declaratorOf = new Map();
  t.traverseFast(s.program.node, (node) => {
    if (node.type !== 'VariableDeclarator' || node.init == null) return;
    t.traverseFast(node.init, (inner) => {
      if (calls.has(inner)) declaratorOf.set(calls.get(inner), node);
    });
  });
  // The program-level declarations a subtree references, by Babel's scope.
  const referenced = (root) => {
    const found = new Set();
    t.traverseFast(root, (node) => {
      const declaration = s.references.get(node);
      if (declaration != null) found.add(declaration);
    });
    return found;
  };
  // Changed definitions and declarations, then every definition referencing one.
  const affected = new Map();
  for (const record of s.inputs) {
    if (unchangedInputs(record, s)) continue;
    const declarator = declaratorOf.get(record);
    if (!matchesShapeRecord(record.call, record.shape) && declarator != null)
      affected.set(declarator, record);
    for (const dependency of record.dependencies)
      if (!matchesShapeRecord(dependency, s.dependencyShapes.get(dependency)))
        affected.set(dependency, record);
  }
  for (let grown = true; grown; ) {
    grown = false;
    for (const record of s.inputs) {
      const declarator = declaratorOf.get(record);
      if (declarator == null || affected.has(declarator)) continue;
      for (const declaration of referenced(record.call))
        if (affected.has(declaration)) {
          affected.set(declarator, affected.get(declaration));
          grown = true;
          break;
        }
    }
  }
  const result = [];
  for (const { call, nodes, declarations } of s.consumerCalls) {
    if (call.start == null) continue;
    for (const declaration of declarations)
      if (affected.has(declaration)) {
        result.push({ call, nodes, definition: affected.get(declaration) });
        break;
      }
  }
  return result;
}

function locationOf(node) {
  const start = node?.loc?.start;
  return start == null
    ? 'an unknown location'
    : `${start.line}:${start.column + 1}`;
}

// Throws if other plugins already transformed a stale consumer's output.
function assertNoStaleConsumers(program, state, s) {
  const stale = s.staleConsumers || [];
  if (stale.length === 0 || s.outputs == null) return;
  // Every node of a stale consumer as written; its output lies at or inside them.
  const consumerOf = new Map();
  for (const entry of stale)
    for (const node of entry.nodes)
      if (!consumerOf.has(node)) consumerOf.set(node, entry);
  const ownerIn = (items) => {
    for (const item of items) {
      if (!isNode(item)) continue;
      let found = null;
      t.traverseFast(item, (node) => {
        found ??= consumerOf.get(node) ?? null;
      });
      if (found != null) return found;
    }
    return null;
  };
  const owners = new Map();
  const own = (items, owner) => {
    for (const item of items)
      if (isNode(item))
        t.traverseFast(item, (node) => {
          owners.set(node, owner);
        });
  };
  for (const { removed, added } of s.outputs) {
    const owner = ownerIn(removed);
    if (owner != null) own(added, owner);
  }
  if (owners.size === 0) return;
  let leaked = null;
  t.traverseFast(program.node, (node) => {
    if (leaked == null && owners.has(node)) leaked = owners.get(node);
  });
  if (leaked == null) return;
  const { call, definition } = leaked;
  throw state.file.buildCodeFrameError(
    call,
    `A plugin rewrote the StyleX definition at ${locationOf(
      definition.call,
    )} (or a value it reads) during the traversal, after its consumer at ${locationOf(
      call,
    )} was compiled and transformed by other plugins, so the consumer would keep class names without CSS. Move the definition above its consumers, or run that plugin in an earlier pass.`,
    Error,
  );
}

// Checks every recorded input at the first visit, after all Program.enter hooks.
function checkProgram(s) {
  if (s.checkedProgram || !s.recordShapes) return;
  s.checkedProgram = true;
  if (s.inputs.some((record) => !unchangedInputs(record, s))) markStale(s);
}

module.exports = {
  assertNoStaleConsumers,
  checkProgram,
  markStale,
  unchangedInputs,
};
