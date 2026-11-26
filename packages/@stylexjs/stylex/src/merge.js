/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow strict
 */

'use strict';

type StyleObject = { +[string]: mixed };
type InlineStyle = { [string]: string | number };
type MergeResult = [string, InlineStyle | null, string];

type TerminalShape = {
  +keys: $ReadOnlyArray<number>,
  +values: $ReadOnlyArray<string>,
  +debug: string | null,
  mask: number,
  chunk: string,
};

type PathNode = {
  next: WeakMap<StyleObject, PathNode> | void,
  value: MergeResult | void,
};

// Every property name gets a small integer id. A merge marks the ids it has
// seen with the current generation, so checking a key is one array read.
const ids: Map<string, number> = new Map();
let marks: Int32Array = new Int32Array(512);
let generation = 0;

function idFor(key: string): number {
  let id = ids.get(key);
  if (id === undefined) {
    id = ids.size;
    ids.set(key, id);
    if (id >= marks.length) {
      const grown = new Int32Array(marks.length * 2);
      grown.set(marks);
      marks = grown;
    }
  }
  return id;
}

const shapes: WeakMap<StyleObject, TerminalShape> = new WeakMap();

// Only strings from the lowest-priority style can contribute to the output.
function terminalShape(style: StyleObject): TerminalShape {
  const keys: Array<number> = [];
  const values: Array<string> = [];
  let debug: string | null = null;
  for (const key in style) {
    const value = style[key];
    if (key === '$$css') {
      if (value !== true) {
        debug = String(value);
      }
    } else if (typeof value === 'string') {
      keys.push(idFor(key));
      values.push(value);
    } else if (value !== null) {
      warnInvalid(key, value);
    }
  }
  const shape = { keys, values, debug, mask: -1, chunk: '' };
  shapes.set(style, shape);
  return shape;
}

function terminalChunk(shape: TerminalShape, current: number): string {
  const { keys, values } = shape;
  if (keys.length <= 31) {
    let mask = 0;
    for (let i = 0; i < keys.length; i++) {
      if (marks[keys[i]] === current) {
        mask |= 1 << i;
      }
    }
    if (mask === shape.mask) {
      return shape.chunk;
    }
    shape.mask = mask;
  }
  let chunk = '';
  for (let i = 0; i < keys.length; i++) {
    if (marks[keys[i]] !== current) {
      chunk = chunk ? chunk + ' ' + values[i] : values[i];
    }
  }
  shape.chunk = chunk;
  return chunk;
}

function warnInvalid(key: string, value: mixed): void {
  console.error(
    `stylex: ${key} typeof ${String(value)} is not "string" or "null".`,
  );
}

function isCompiled(style: StyleObject): boolean {
  return style.$$css != null;
}

const singles: WeakMap<StyleObject, MergeResult> = new WeakMap();
const pairs: WeakMap<StyleObject, WeakMap<StyleObject, MergeResult> | null> =
  new WeakMap();

function mergeSingle(style: StyleObject): MergeResult {
  let className = '';
  let debug = '';
  for (const key in style) {
    const value = style[key];
    if (key === '$$css') {
      if (value !== true) {
        debug = String(value);
      }
    } else if (typeof value === 'string') {
      className = className ? className + ' ' + value : value;
    } else if (value !== null) {
      warnInvalid(key, value);
    }
  }
  const result: MergeResult = [className, null, debug];
  singles.set(style, result);
  return result;
}

function mergePair(base: StyleObject, override: StyleObject): MergeResult {
  let firstKey: string | void;
  let secondKey: string | void;
  let moreKeys: Set<string> | null = null;
  let suffix = '';
  let prefix = '';
  let debug = '';
  for (const key in override) {
    const value = override[key];
    if (key === '$$css') {
      if (value !== true) {
        debug = String(value);
      }
    } else if (typeof value === 'string' || value === null) {
      if (firstKey === undefined) {
        firstKey = key;
      } else if (secondKey === undefined) {
        secondKey = key;
      } else {
        if (moreKeys === null) {
          moreKeys = new Set();
        }
        moreKeys.add(key);
      }
      if (value !== null) {
        suffix = suffix ? suffix + ' ' + value : value;
      }
    } else {
      warnInvalid(key, value);
    }
  }
  for (const key in base) {
    const value = base[key];
    if (key === '$$css') {
      if (value !== true) {
        debug = debug ? String(value) + '; ' + debug : String(value);
      }
    } else if (typeof value === 'string') {
      if (
        key !== firstKey &&
        key !== secondKey &&
        (moreKeys === null || !moreKeys.has(key))
      ) {
        prefix = prefix ? prefix + ' ' + value : value;
      }
    } else if (value !== null) {
      warnInvalid(key, value);
    }
  }
  const className = prefix && suffix ? prefix + ' ' + suffix : prefix || suffix;
  return [className, null, debug];
}

// A base seen only once needs no child WeakMap. Its next pair lookup creates
// the child map, independently of calls with other bases.
function cachePair(
  base: StyleObject,
  override: StyleObject,
  entries: WeakMap<StyleObject, MergeResult> | null | void,
): MergeResult {
  const result = mergePair(base, override);
  if (entries === undefined) {
    pairs.set(base, null);
  } else {
    let children = entries;
    if (children === null) {
      children = new WeakMap();
      pairs.set(base, children);
    }
    children.set(override, result);
  }
  return result;
}

// Deeper inputs still merge with the iterative stack.
const maxCacheDepth = 32;
const rootChildren: WeakMap<StyleObject, PathNode> = new WeakMap();
const resultRoot: PathNode = { next: rootChildren, value: undefined };

function childOf(node: PathNode | void, item: mixed): PathNode | void {
  if (
    node === undefined ||
    item == null ||
    typeof item !== 'object' ||
    Array.isArray(item)
  ) {
    return undefined;
  }
  const children = node.next;
  return children === undefined ? undefined : children.get(item);
}

/**
 * Merges StyleX styles into [className, inlineStyle, debugString].
 * Later styles win. This is a faster replacement for styleq with the same
 * output, including its cache of previously merged sequences. Like that
 * cache, it assumes compiled style objects are never mutated.
 */
export function merge(styles: $ReadOnlyArray<mixed>): MergeResult {
  const length = styles.length;
  if (length === 0) {
    return ['', null, ''];
  }
  const first = styles[0];
  if (length === 1 || (length === 2 && !styles[1])) {
    if (first == null || typeof first !== 'object') {
      return ['', null, ''];
    }
    if (!Array.isArray(first)) {
      const cached = singles.get(first);
      if (cached !== undefined) {
        return cached;
      }
      if (isCompiled(first)) {
        return mergeSingle(first);
      }
    }
  } else if (length === 2) {
    const last = styles[1];
    if (
      first != null &&
      typeof first === 'object' &&
      !Array.isArray(first) &&
      last != null &&
      typeof last === 'object' &&
      !Array.isArray(last)
    ) {
      const entries = pairs.get(first);
      if (entries != null) {
        const cached = entries.get(last);
        if (cached !== undefined) {
          return cached;
        }
      }
      if (isCompiled(first) && isCompiled(last)) {
        return cachePair(first, last, entries);
      }
    }
  } else if (length === 3 && !Array.isArray(styles[2])) {
    const hit = childOf(
      childOf(childOf(resultRoot, first), styles[1]),
      styles[2],
    );
    if (hit !== undefined && hit.value !== undefined) {
      return hit.value;
    }
  }
  // A sole array argument already contains the styles in application order.
  const list = length === 1 && Array.isArray(first) ? first : styles;
  const found = lookup(resultRoot, list, 0);
  if (found !== undefined && found.value !== undefined) {
    return found.value;
  }
  return mergeMany(list, found);
}

function mergeMany(
  styles: $ReadOnlyArray<mixed>,
  found: PathNode | void,
): MergeResult {
  if (++generation === 0x7fffffff) {
    generation = 1;
    marks.fill(0);
  }
  const current = generation;
  let className = '';
  let inlineStyle: InlineStyle | null = null;
  let debugString = '';
  let compiledOnly = true;
  let stack: $ReadOnlyArray<mixed> = styles;
  let owned: Array<mixed> | null = null;
  let remaining = styles.length;
  while (remaining > 0) {
    const item = stack[--remaining];
    if (item == null || typeof item !== 'object') {
      continue;
    }
    if (Array.isArray(item)) {
      if (owned === null) {
        owned = styles.slice(0, remaining);
        stack = owned;
      }
      for (let i = 0; i < item.length; i++) {
        owned[remaining++] = item[i];
      }
      continue;
    }
    const style: StyleObject = item;
    if (isCompiled(style)) {
      let chunk = '';
      let debug: string | null = null;
      let shape: TerminalShape | void;
      if (remaining === 0 && !compiledOnly) {
        shape = shapes.get(style);
        if (shape === undefined) {
          shape = terminalShape(style);
        }
      }
      if (shape !== undefined) {
        debug = shape.debug;
        chunk = terminalChunk(shape, current);
      } else {
        for (const key in style) {
          const value = style[key];
          if (key === '$$css') {
            if (value !== true) {
              debug = String(value);
            }
          } else if (typeof value === 'string' || value === null) {
            if (remaining === 0 && value === null) {
              continue;
            }
            const id = idFor(key);
            if (marks[id] === current) {
              continue;
            }
            marks[id] = current;
            if (value !== null) {
              chunk = chunk ? chunk + ' ' + value : value;
            }
          } else {
            warnInvalid(key, value);
          }
        }
      }
      if (debug !== null) {
        debugString = debugString ? debug + '; ' + debugString : debug;
      }
      if (chunk) {
        className = className ? chunk + ' ' + className : chunk;
      }
    } else {
      compiledOnly = false;
      let subStyle: InlineStyle | null = null;
      for (const prop in style) {
        const value = style[prop];
        if (value === undefined) {
          continue;
        }
        const id = idFor(prop);
        if (marks[id] === current) {
          continue;
        }
        marks[id] = current;
        if (value !== null) {
          if (subStyle === null) {
            const empty: InlineStyle = {};
            subStyle = empty;
          }
          // $FlowFixMe[incompatible-type] inline values are passed through as-is
          subStyle[prop] = value;
        }
      }
      if (subStyle !== null) {
        if (inlineStyle !== null) {
          for (const prop in inlineStyle) {
            subStyle[prop] = inlineStyle[prop];
          }
        }
        inlineStyle = subStyle;
      }
    }
  }
  const result: MergeResult = [className, inlineStyle, debugString];
  if (compiledOnly) {
    // Register a new flat base first. Build paths when that base is reused.
    const base = styles[0];
    if (
      base != null &&
      typeof base === 'object' &&
      !Array.isArray(base) &&
      !rootChildren.has(base)
    ) {
      rootChildren.set(base, { next: undefined, value: undefined });
      return result;
    }
    const node = found !== undefined ? found : storePath(resultRoot, styles, 0);
    if (node !== undefined) {
      node.value = result;
    }
  }
  return result;
}

function lookup(
  node: PathNode,
  styles: $ReadOnlyArray<mixed>,
  depth: number,
): PathNode | void {
  let current: PathNode = node;
  for (let i = 0; i < styles.length; i++) {
    let item = styles[i];
    if (Array.isArray(item) && item.length === 1) {
      item = item[0];
    }
    if (item == null || typeof item !== 'object') {
      continue;
    }
    let next: PathNode | void;
    if (Array.isArray(item)) {
      if (depth === maxCacheDepth) {
        return undefined;
      }
      next = lookup(current, item, depth + 1);
    } else {
      const children = current.next;
      next = children === undefined ? undefined : children.get(item);
    }
    if (next === undefined) {
      return undefined;
    }
    current = next;
  }
  return current;
}

function storePath(
  node: PathNode,
  styles: $ReadOnlyArray<mixed>,
  depth: number,
): PathNode | void {
  let current: PathNode = node;
  for (let i = 0; i < styles.length; i++) {
    const item = styles[i];
    if (item == null || typeof item !== 'object') {
      continue;
    }
    if (Array.isArray(item)) {
      if (depth === maxCacheDepth) {
        return undefined;
      }
      const next = storePath(current, item, depth + 1);
      if (next === undefined) {
        return undefined;
      }
      current = next;
    } else {
      let children = current.next;
      if (children === undefined) {
        children = new WeakMap();
        current.next = children;
      }
      let child = children.get(item);
      if (child === undefined) {
        child = { next: undefined, value: undefined };
        children.set(item, child);
      }
      current = child;
    }
  }
  return current;
}
