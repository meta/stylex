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

type Shape = {
  +ids: $ReadOnlyArray<number>,
  +values: $ReadOnlyArray<string | null>,
  +className: string,
  +debug: string,
};

// [className, ids it defines, debug string after it, next level of the cache]
type CacheEntry = [string, $ReadOnlyArray<number>, string, Cache];
type Cache = WeakMap<StyleObject, CacheEntry>;

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

const shapes: WeakMap<StyleObject, Shape> = new WeakMap();

function shapeOf(style: StyleObject): Shape {
  const cached = shapes.get(style);
  if (cached !== undefined) {
    return cached;
  }
  const shapeIds: Array<number> = [];
  const values: Array<string | null> = [];
  let className = '';
  let debug = '';
  for (const key in style) {
    const value = style[key];
    if (key === '$$css') {
      if (value !== true) {
        debug = String(value);
      }
    } else if (typeof value === 'string' || value === null) {
      shapeIds.push(idFor(key));
      values.push(value);
      if (value !== null) {
        className = className ? className + ' ' + value : value;
      }
    } else {
      warnInvalid(key, value);
    }
  }
  const shape = { ids: shapeIds, values, className, debug };
  shapes.set(style, shape);
  return shape;
}

function warnInvalid(key: string, value: mixed): void {
  console.error(
    `stylex: ${key} typeof ${String(value)} is not "string" or "null".`,
  );
}

function isCompiled(style: StyleObject): boolean {
  return style.$$css != null;
}

const rootCache: Cache = new WeakMap();

/**
 * Merges StyleX styles into [className, inlineStyle, debugString].
 * Later styles win. This is a faster replacement for styleq with the same
 * output, including its cache of previously merged sequences. Like that
 * cache, it assumes compiled style objects are never mutated.
 */
export function merge(styles: $ReadOnlyArray<mixed>): MergeResult {
  if (styles.length === 0) {
    return ['', null, ''];
  }
  if (styles.length === 1) {
    const only = styles[0];
    if (only == null || typeof only !== 'object') {
      return ['', null, ''];
    }
    if (!Array.isArray(only) && isCompiled(only)) {
      const shape = shapeOf(only);
      return [shape.className, null, shape.debug];
    }
  }

  generation++;
  if (generation === 0x7fffffff) {
    generation = 1;
    marks.fill(0);
  }
  const current = generation;

  let className = '';
  let inlineStyle: InlineStyle | null = null;
  let debugString = '';
  let nextCache: Cache | null = rootCache;
  // ids from cache hits, marked only if a later style needs them
  let pending: Array<$ReadOnlyArray<number>> | null = null;

  const stack: Array<mixed> = [...styles];
  while (stack.length > 0) {
    const item = stack.pop();
    if (item == null || typeof item !== 'object') {
      continue;
    }
    if (Array.isArray(item)) {
      for (let i = 0; i < item.length; i++) {
        stack.push(item[i]);
      }
      continue;
    }
    const style: StyleObject = item;

    if (isCompiled(style)) {
      let chunk = '';
      const entry = nextCache != null ? nextCache.get(style) : undefined;
      if (entry !== undefined) {
        chunk = entry[0];
        debugString = entry[2];
        if (pending === null) {
          pending = [entry[1]];
        } else {
          pending.push(entry[1]);
        }
        nextCache = entry[3];
      } else {
        if (pending !== null) {
          markAll(pending, current);
          pending = null;
        }
        // Shapes are only stored by single-style calls. Storing one for every
        // merged object costs more than it saves when objects are reused less.
        const shape = shapes.get(style);
        const added: Array<number> | null = nextCache != null ? [] : null;
        if (shape !== undefined) {
          for (let i = 0; i < shape.ids.length; i++) {
            const id = shape.ids[i];
            if (marks[id] !== current) {
              marks[id] = current;
              if (added !== null) {
                added.push(id);
              }
              const value = shape.values[i];
              if (value !== null) {
                chunk = chunk ? chunk + ' ' + value : value;
              }
            }
          }
          if (shape.debug !== '') {
            debugString = debugString
              ? shape.debug + '; ' + debugString
              : shape.debug;
          }
        } else {
          for (const key in style) {
            const value = style[key];
            if (key === '$$css') {
              if (value !== true) {
                const debug = String(value);
                debugString = debugString ? debug + '; ' + debugString : debug;
              }
            } else if (typeof value === 'string' || value === null) {
              const id = idFor(key);
              if (marks[id] !== current) {
                marks[id] = current;
                if (added !== null) {
                  added.push(id);
                }
                if (value !== null) {
                  chunk = chunk ? chunk + ' ' + value : value;
                }
              }
            } else {
              warnInvalid(key, value);
            }
          }
        }
        if (nextCache != null && added !== null) {
          const child: Cache = new WeakMap();
          nextCache.set(style, [chunk, added, debugString, child]);
          nextCache = child;
        }
      }
      if (chunk) {
        className = className ? chunk + ' ' + className : chunk;
      }
    } else {
      if (pending !== null) {
        markAll(pending, current);
        pending = null;
      }
      let subStyle: InlineStyle | null = null;
      for (const prop in style) {
        const value = style[prop];
        if (value !== undefined) {
          const id = idFor(prop);
          if (marks[id] !== current) {
            if (value != null) {
              if (subStyle == null) {
                const empty: InlineStyle = {};
                subStyle = empty;
              }
              // $FlowFixMe[incompatible-type] inline values are passed through as-is
              subStyle[prop] = value;
            }
            marks[id] = current;
            nextCache = null;
          }
        }
      }
      if (subStyle != null) {
        inlineStyle =
          inlineStyle == null ? subStyle : { ...subStyle, ...inlineStyle };
      }
    }
  }
  return [className, inlineStyle, debugString];
}

function markAll(
  pending: $ReadOnlyArray<$ReadOnlyArray<number>>,
  current: number,
): void {
  for (let p = 0; p < pending.length; p++) {
    const list = pending[p];
    for (let i = 0; i < list.length; i++) {
      marks[list[i]] = current;
    }
  }
}
