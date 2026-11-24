'use strict';

// Greedy tree minimizer over fuzz specs. Tries removing list elements,
// simplifying value nodes and dropping options, keeping any edit after which
// the predicate still holds. Works on JSON specs from gen.cjs.

const LIST_KEYS = new Set(['items', 'uses', 'consts', 'namespaces', 'entries', 'args', 'frames', 'overrides', 'typeBits', 'decor']);

function clone(x) {
  return JSON.parse(JSON.stringify(x));
}

function getAt(obj, p) {
  let o = obj;
  for (const k of p) o = o[k];
  return o;
}

function setAt(obj, p, v) {
  const parent = getAt(obj, p.slice(0, -1));
  parent[p[p.length - 1]] = v;
}

function* edits(spec) {
  if (spec.kind === 'corpus') {
    for (const k of Object.keys(spec.options)) yield (s) => { delete s.options[k]; };
    if (JSON.stringify(spec.processConfig) !== '{"useLayers":false}') yield (s) => { s.processConfig = { useLayers: false }; };
    return;
  }
  // spec-level simplifications first: they shrink the most
  if (spec.mutation) yield (s) => { s.mutation = null; };
  if (spec.usesFirst) yield (s) => { s.usesFirst = false; };
  if (spec.debugFilePathFn) yield (s) => { s.debugFilePathFn = false; };
  if (spec.uses.length) yield (s) => { s.uses = []; };
  if (spec.items.length > 1) {
    for (let i = 0; i < spec.items.length; i++) yield (s) => { s.items.splice(i, 1); };
  }
  for (const k of Object.keys(spec.options)) yield (s) => { delete s.options[k]; };
  if (JSON.stringify(spec.processConfig) !== '{"useLayers":false}') yield (s) => { s.processConfig = { useLayers: false }; };
  if (spec.uses.some((u) => u.inJsx)) yield (s) => { for (const u of s.uses) u.inJsx = false; };
  if (spec.uses.some((u) => u.componentSx || u.classNameMerge || u.topLevel)) yield (s) => { for (const u of s.uses) { u.componentSx = false; u.classNameMerge = false; u.topLevel = false; } };
  if (spec.items.some((it) => (it.namespaces || []).some((ns) => ns.spreadBase))) yield (s) => { for (const it of s.items) for (const ns of it.namespaces || []) ns.spreadBase = false; };
  if (spec.syntax !== 'plain') yield (s) => { s.syntax = 'plain'; };
  if (spec.syntax === 'flow' || spec.syntax === 'tsx') yield (s) => { s.syntax = 'js'; };
  if (spec.importForm !== 'ns') yield (s) => { s.importForm = 'ns'; s.customSource = null; };
  if (spec.filename) yield (s) => { s.filename = null; };
  if (spec.fileKind === 'tokensUser') yield (s) => { s.fileKind = 'component'; };
  for (let i = 0; i < spec.items.length; i++) {
    for (const flag of ['exported', 'quotedKeys', 'inFunction', 'paren', 'exportDefault']) {
      if (spec.items[i][flag]) yield (s) => { s.items[i][flag] = false; };
    }
  }

  // list element removal and value simplification, anywhere in the tree
  const lists = [];
  const values = [];
  const walk = (node, p) => {
    if (Array.isArray(node)) {
      node.forEach((x, i) => walk(x, [...p, i]));
      return;
    }
    if (!node || typeof node !== 'object') return;
    if (typeof node.t === 'string' && p.length) values.push([p, node]);
    for (const [k, v] of Object.entries(node)) {
      if (LIST_KEYS.has(k) && Array.isArray(v)) lists.push([[...p, k], v.length]);
      walk(v, [...p, k]);
    }
  };
  walk(spec, []);
  // bigger lists first, from the end (later items depend on earlier ones less often)
  lists.sort((a, b) => b[1] - a[1]);
  for (const [p, n] of lists) {
    if (p.length === 1 && p[0] === 'items') continue; // handled above
    for (let i = n - 1; i >= 0; i--) {
      yield (s) => {
        const arr = getAt(s, p);
        if (Array.isArray(arr) && i < arr.length) arr.splice(i, 1);
      };
    }
  }
  for (const [p, node] of values) {
    const repl = [];
    if (p.includes('args')) {
      // props()/attrs() arguments: simplify toward the plain forms
      if (node.t === 'and') repl.push(node.arg);
      if (node.t === 'tern') repl.push(node.a, node.b);
      if (node.t === 'arr' && node.items[0]) repl.push(node.items[0]);
      if (node.t !== 'lit' || node.v !== null) repl.push({ t: 'lit', v: null });
      for (const r of repl) yield (s) => { setAt(s, p, clone(r)); };
      continue;
    }
    switch (node.t) {
      case 'cond': {
        const d = node.entries.find(([k]) => k === 'default');
        if (d) repl.push(d[1]);
        if (node.entries[0]) repl.push(node.entries[0][1]);
        break;
      }
      case 'arr':
      case 'ftw':
        if (node.items[0]) repl.push(node.items[0]);
        break;
      case 'type':
        repl.push(node.value);
        break;
      case 'obj':
        break;
      case 'lit':
        if (node.v !== 'red' && typeof node.v === 'string') repl.push({ t: 'lit', v: 'red' });
        break;
      default:
        repl.push({ t: 'lit', v: 'red' });
    }
    for (const r of repl) yield (s) => { setAt(s, p, clone(r)); };
  }
  // condition keys: computed keys (when.*, constants) become plain strings
  for (const [p, node] of values) {
    if (node.t !== 'cond') continue;
    for (let i = 0; i < node.entries.length; i++) {
      if (typeof node.entries[i][0] !== 'object') continue;
      yield (s) => { getAt(s, p).entries[i][0] = ':hover'; };
    }
  }
}

async function minimize(spec, predicate, { budget = 300 } = {}) {
  let cur = clone(spec);
  let evals = 0;
  let progress = true;
  while (progress && evals < budget) {
    progress = false;
    for (const edit of edits(cur)) {
      if (evals >= budget) break;
      const next = clone(cur);
      try {
        edit(next);
      } catch {
        continue;
      }
      if (JSON.stringify(next) === JSON.stringify(cur)) continue;
      evals++;
      if (await predicate(next)) {
        cur = next;
        progress = true;
        break;
      }
    }
  }
  return { spec: cur, evals };
}

module.exports = { minimize };
