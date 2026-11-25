/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Structural equality of ASTs, ignoring positions, comments and parser extras.
// A hash match is always confirmed structurally.

const { babelParser, t } = require('./babel-deps');

const COMMENT_FIELDS = ['leadingComments', 'innerComments', 'trailingComments'];

const IGNORED_FIELDS = new Set([
  'type',
  'loc',
  'start',
  'end',
  'range',
  'extra',
  'leadingComments',
  'trailingComments',
  'innerComments',
  'comments',
  'tokens',
  'errors',
  'sourceFile',
  'parent',
]);

function isNode(value) {
  return (
    value != null &&
    typeof value === 'object' &&
    typeof value.type === 'string' &&
    !Array.isArray(value)
  );
}

const typeFields = new Map();
function fieldsOf(node) {
  let names = typeFields.get(node.type);
  if (names !== undefined) return names;
  const fields = t.NODE_FIELDS[node.type];
  if (fields == null)
    // A node type Babel does not define: compare its own data fields.
    return Object.keys(node).filter(
      (key) => key[0] !== '_' && !IGNORED_FIELDS.has(key),
    );
  names = Object.keys(fields).filter((key) => !IGNORED_FIELDS.has(key));
  typeFields.set(node.type, names);
  return names;
}

// Scalars that describe the same program compare equal.
function absent(value) {
  return (
    value == null ||
    value === false ||
    (Array.isArray(value) && value.length === 0)
  );
}

function sameScalar(a, b) {
  if (absent(a) && absent(b)) return true;
  if (a === b) return true;
  if (typeof a === 'object' && typeof b === 'object' && a && b)
    return JSON.stringify(a) === JSON.stringify(b);
  return Number.isNaN(a) && Number.isNaN(b);
}

function mix(hash, value) {
  return Math.imul(hash ^ value, 0x01000193) >>> 0;
}

// Strings hash by an interned id, which V8 handles far faster than mixing
// characters. Ids are only stable within one `Shapes` table.
function hashString(hash, text, strings) {
  let id = strings.get(text);
  if (id === undefined) {
    id = strings.size + 1;
    strings.set(text, id);
  }
  return mix(hash, id);
}

const SCALAR_TAGS = { string: 1, number: 2, boolean: 3, bigint: 4 };

function hashScalar(hash, value, strings) {
  if (absent(value)) return hash;
  if (typeof value === 'object')
    return hashString(mix(hash, 6), JSON.stringify(value), strings);
  return hashString(
    mix(hash, SCALAR_TAGS[typeof value] || 5),
    String(value),
    strings,
  );
}

function commentsEqual(a, b) {
  for (const field of COMMENT_FIELDS) {
    const left = a[field] || [];
    const right = b[field] || [];
    if (left.length !== right.length) return false;
    for (let index = 0; index < left.length; index++)
      if (
        left[index].type !== right[index].type ||
        left[index].value !== right[index].value
      )
        return false;
  }
  return true;
}

// Exact structural equality; with `comments`, attached comment text too.
// Nodes of `b` in `opaque` only need a counterpart of the same type.
function equalTrees(a, b, comments, opaque = null) {
  if (a === b) return true;
  if (opaque != null && isNode(b) && opaque.has(b))
    return isNode(a) && a.type === b.type;
  if (!isNode(a) || !isNode(b)) return sameScalar(a, b);
  if (a.type !== b.type) return false;
  if (comments && !commentsEqual(a, b)) return false;
  for (const field of fieldsOf(a)) {
    const left = a[field];
    const right = b[field];
    if (Array.isArray(left) || Array.isArray(right)) {
      const l = left || [];
      const r = right || [];
      if (l.length !== r.length) return false;
      for (let index = 0; index < l.length; index++)
        if (!equalTrees(l[index], r[index], comments, opaque)) return false;
    } else if (isNode(left) || isNode(right)) {
      if (!equalTrees(left, right, comments, opaque)) return false;
    } else if (!sameScalar(left, right)) return false;
  }
  return true;
}

// Compact records of a subtree's structure, as `equalTrees` compares it, except that type-only
// syntax is left out: the TypeScript and Flow plugins strip `as const`, `satisfies T` and
// annotations in the same pass, which changes no value StyleX reads.
// Scalars are a numeric tag then the value, so they never align with a type.
const ABSENT = 0;
const SCALAR = 1;
const OBJECT = 2;
// Matches any subtree: used for definitions StyleX replaces at their visit.
const OPAQUE = 3;

// Expressions whose value is their operand's.
const TYPE_WRAPPERS = new Set([
  'TSAsExpression',
  'TSSatisfiesExpression',
  'TSNonNullExpression',
  'TSTypeAssertion',
  'TSInstantiationExpression',
  'TypeCastExpression',
  'ParenthesizedExpression',
]);
// Fields that hold only types.
const TYPE_FIELDS = new Set([
  'typeAnnotation',
  'typeParameters',
  'typeArguments',
  'returnType',
  'predicate',
]);
function valueNode(node) {
  while (isNode(node) && TYPE_WRAPPERS.has(node.type)) node = node.expression;
  return node;
}
function valueFieldsOf(node) {
  return fieldsOf(node).filter((field) => !TYPE_FIELDS.has(field));
}

function shapeRecord(node, record = [], opaque = null) {
  if (opaque != null && opaque.has(node)) {
    record.push(OPAQUE);
    return record;
  }
  node = valueNode(node);
  if (!isNode(node)) {
    recordScalar(node, record);
    return record;
  }
  if (opaque != null && opaque.has(node)) {
    record.push(OPAQUE);
    return record;
  }
  record.push(node.type);
  for (const field of valueFieldsOf(node)) {
    const value = node[field];
    if (Array.isArray(value)) {
      record.push(value.length);
      for (const item of value) shapeRecord(item, record, opaque);
    } else if (isNode(value)) shapeRecord(value, record, opaque);
    else recordScalar(value, record);
  }
  return record;
}

// Scalars that describe the same program record the same (see `sameScalar`).
function recordScalar(value, record) {
  if (absent(value)) record.push(ABSENT);
  else if (typeof value === 'object')
    record.push(OBJECT, JSON.stringify(value));
  else record.push(SCALAR, value);
}

// Whether another plugin may edit a definition before StyleX visits it.
function othersMayEditDefinitions(state) {
  const opts = state.file.opts;
  const passes = [
    opts.plugins || [],
    ...(opts.presets || []).map((p) => p.plugins),
  ];
  const pass = passes.find((plugins) =>
    plugins.some((plugin) => plugin.key === state.key),
  );
  if (pass == null) return true;
  return pass.some((plugin) => {
    if (plugin.key === state.key) return false;
    const visitor = plugin.visitor || {};
    return Object.keys(visitor).some(
      (key) =>
        key !== '_exploded' &&
        key !== '_verified' &&
        (key !== 'Program' || visitor.Program.enter != null),
    );
  });
}

// The plugins of the pass StyleX runs in, and StyleX's place in it.
function passOf(state) {
  const opts = state.file.opts;
  const passes = [
    opts.plugins || [],
    ...(opts.presets || []).map((p) => p.plugins),
  ];
  const index = passes.findIndex((plugins) =>
    plugins.some((plugin) => plugin.key === state.key),
  );
  return index < 0 ? null : { first: index === 0, plugins: passes[index] };
}

const visitorKeys = (plugin) =>
  Object.keys(plugin.visitor || {}).filter(
    (key) => key !== '_exploded' && key !== '_verified',
  );

// Whether no other plugin of the pass has a visitor or a `pre` hook.
function aloneInPass(state) {
  // A visitor wrapper runs code around every visitor, StyleX's included.
  if (state.file.opts.wrapPluginVisitorMethod != null) return false;
  const pass = passOf(state);
  return (
    pass != null &&
    pass.plugins.every(
      (plugin) =>
        plugin.key === state.key ||
        (plugin.pre == null && visitorKeys(plugin).length === 0),
    )
  );
}

// Prototype of Babel-parsed nodes (Babel clones input ASTs to plain objects).
const PARSED = Object.getPrototypeOf(babelParser.parse(''));

// Whether the program is still Babel's untouched first-pass parse of the file.
function parsedAndUntouched(state, program) {
  const opts = state.file.opts;
  // With `cloneInputAst` off, plugins receive the caller's own nodes.
  if (opts.cloneInputAst !== true) return false;
  // A visitor wrapper can change the program before StyleX's Program.enter.
  if (opts.wrapPluginVisitorMethod != null) return false;
  if (
    state.file.ast.program !== program.node ||
    Object.getPrototypeOf(state.file.ast) !== PARSED ||
    Object.getPrototypeOf(program.node) !== PARSED
  )
    return false;
  // A parser override in any pass parses instead of Babel.
  const passes = [
    opts.plugins || [],
    ...(opts.presets || []).map((preset) => preset.plugins),
  ];
  if (
    passes.some((plugins) =>
      plugins.some((plugin) => typeof plugin.parserOverride === 'function'),
    )
  )
    return false;
  const pass = passOf(state);
  if (pass == null || !pass.first) return false;
  let before = true;
  for (const plugin of pass.plugins) {
    if (plugin.key === state.key) {
      before = false;
      continue;
    }
    if (plugin.pre != null) return false;
    const visitor = plugin.visitor || {};
    if (before && (visitor.enter != null || visitor.Program?.enter != null))
      return false;
  }
  return true;
}

// Parser options under which JSX parses without the `jsx` plugin.
function impliesJsx(parserOpts) {
  return (parserOpts?.plugins || []).some(
    (plugin) =>
      Array.isArray(plugin) &&
      plugin[0] === 'typescript' &&
      plugin[1]?.isTSX === true,
  );
}

function matchesShapeRecord(node, record) {
  let index = 0;
  const scalar = (value) => {
    const tag = record[index++];
    if (absent(value)) return tag === ABSENT;
    if (typeof value === 'object')
      return tag === OBJECT && record[index++] === JSON.stringify(value);
    if (tag !== SCALAR) return false;
    const recorded = record[index++];
    return (
      recorded === value ||
      (typeof value === 'number' &&
        Number.isNaN(value) &&
        Number.isNaN(recorded))
    );
  };
  const walk = (current) => {
    if (record[index] === OPAQUE) {
      index++;
      return true;
    }
    current = valueNode(current);
    if (!isNode(current)) return scalar(current);
    if (record[index++] !== current.type) return false;
    for (const field of valueFieldsOf(current)) {
      const value = current[field];
      if (Array.isArray(value)) {
        if (record[index++] !== value.length) return false;
        for (const item of value) if (!walk(item)) return false;
      } else if (isNode(value)) {
        if (!walk(value)) return false;
      } else if (!scalar(value)) return false;
    }
    return true;
  };
  return walk(node) && index === record.length;
}

class Shapes {
  constructor() {
    this.hashes = new WeakMap();
    this.strings = new Map();
  }

  hash(node) {
    let hash = this.hashes.get(node);
    if (hash !== undefined) return hash;
    const strings = this.strings;
    hash = hashString(0x811c9dc5, node.type, strings);
    const fields = fieldsOf(node);
    for (let position = 0; position < fields.length; position++) {
      const value = node[fields[position]];
      if (absent(value)) continue;
      hash = mix(hash, position + 1);
      if (Array.isArray(value)) {
        hash = mix(hash, value.length);
        for (const item of value)
          hash = isNode(item)
            ? mix(hash, this.hash(item))
            : hashScalar(hash, item, strings);
      } else if (isNode(value)) hash = mix(hash, this.hash(value));
      else hash = hashScalar(hash, value, strings);
    }
    this.hashes.set(node, hash);
    return hash;
  }

  itemKey(item) {
    return isNode(item) ? this.hash(item) : hashScalar(1, item, this.strings);
  }

  // A key from the node's type and own scalars only; equal subtrees share it.
  shallowKey(item) {
    if (!isNode(item)) return hashScalar(1, item, this.strings);
    const strings = this.strings;
    let hash = hashString(0x811c9dc5, item.type, strings);
    const fields = fieldsOf(item);
    for (let position = 0; position < fields.length; position++) {
      const value = item[fields[position]];
      if (absent(value) || Array.isArray(value) || isNode(value)) continue;
      hash = hashScalar(mix(hash, position + 1), value, strings);
    }
    return hash;
  }

  // A key equal for equal subtrees from the first `depth` levels only.
  boundedKey(item, depth) {
    if (!isNode(item)) return hashScalar(1, item, this.strings);
    const strings = this.strings;
    let hash = hashString(0x811c9dc5, item.type, strings);
    const fields = fieldsOf(item);
    for (let position = 0; position < fields.length; position++) {
      const value = item[fields[position]];
      if (absent(value)) continue;
      hash = mix(hash, position + 1);
      if (Array.isArray(value)) {
        hash = mix(hash, value.length);
        if (depth > 1)
          for (const child of value)
            hash = mix(hash, this.boundedKey(child, depth - 1));
      } else if (isNode(value)) {
        if (depth > 1) hash = mix(hash, this.boundedKey(value, depth - 1));
      } else hash = hashScalar(hash, value, strings);
    }
    return hash;
  }

  // Full keys only where the other list may hold an equal item.
  listKeys(before, after) {
    const shallowBefore = before.map((item) => this.boundedKey(item, 4));
    const shallowAfter = after.map((item) => this.boundedKey(item, 4));
    const inAfter = new Set(shallowAfter);
    const inBefore = new Set(shallowBefore);
    return [
      before.map((item, index) =>
        inAfter.has(shallowBefore[index]) ? this.itemKey(item) : `b${index}`,
      ),
      after.map((item, index) =>
        inBefore.has(shallowAfter[index]) ? this.itemKey(item) : `a${index}`,
      ),
    ];
  }

  // Exact equality, confirmed structurally after a hash match.
  same(a, b) {
    if (a === b) return true;
    if (!isNode(a) || !isNode(b)) return sameScalar(a, b);
    return this.hash(a) === this.hash(b) && equalTrees(a, b, false);
  }

  // Only the nodes' own non-node fields.
  sameScalars(a, b) {
    for (const field of new Set([...fieldsOf(a), ...fieldsOf(b)])) {
      const left = a[field];
      const right = b[field];
      if (
        Array.isArray(left) ||
        isNode(left) ||
        Array.isArray(right) ||
        isNode(right)
      )
        continue;
      if (!sameScalar(left, right)) return false;
    }
    return true;
  }
}

module.exports = {
  aloneInPass,
  COMMENT_FIELDS,
  equalTrees,
  fieldsOf,
  impliesJsx,
  isNode,
  matchesShapeRecord,
  othersMayEditDefinitions,
  parsedAndUntouched,
  shapeRecord,
  Shapes,
};
