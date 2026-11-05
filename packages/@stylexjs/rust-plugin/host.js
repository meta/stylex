/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const url = require('node:url');

const EXTENSIONS = ['.js', '.ts', '.tsx', '.jsx', '.mjs', '.cjs'];

// Binary operators the compiler may apply to host values; a fixed table, never
// evaluated source text.
const BINARY = {
  '+': (a, b) => a + b,
  '-': (a, b) => a - b,
  '*': (a, b) => a * b,
  '/': (a, b) => a / b,
  '%': (a, b) => a % b,
  '**': (a, b) => a ** b,
  '==': (a, b) => a == b, // eslint-disable-line eqeqeq
  '!=': (a, b) => a != b, // eslint-disable-line eqeqeq
  '===': (a, b) => a === b,
  '!==': (a, b) => a !== b,
  '<': (a, b) => a < b,
  '<=': (a, b) => a <= b,
  '>': (a, b) => a > b,
  '>=': (a, b) => a >= b,
  '&': (a, b) => a & b,
  '|': (a, b) => a | b,
  '^': (a, b) => a ^ b,
  '<<': (a, b) => a << b,
  '>>': (a, b) => a >> b,
  '>>>': (a, b) => a >>> b,
  in: (a, b) => a in b,
  instanceof: (a, b) => a instanceof b,
};
function binary(operator, left, right) {
  if (!Object.hasOwn(BINARY, operator)) {
    throw new TypeError(`Unsupported StyleX host operator ${operator}`);
  }
  return BINARY[operator](left, right);
}

const UNARY = {
  '+': (value) => +value,
  '-': (value) => -value,
  '~': (value) => ~value,
  '!': (value) => !value,
  typeof: (value) => typeof value,
  void: () => undefined,
};
function unary(operator, value) {
  if (!Object.hasOwn(UNARY, operator)) {
    throw new TypeError(`Unsupported StyleX host operator ${operator}`);
  }
  return UNARY[operator](value);
}

// JavaScript ToPrimitive (ECMA-262 7.1.1) on the host's own values.
function toPrimitive(value, hint) {
  if (
    value === null ||
    (typeof value !== 'object' && typeof value !== 'function')
  ) {
    return value;
  }
  const exotic = value[Symbol.toPrimitive];
  if (exotic != null) {
    const result = exotic.call(value, hint);
    if (
      result === null ||
      (typeof result !== 'object' && typeof result !== 'function')
    ) {
      return result;
    }
    throw new TypeError('Cannot convert object to primitive value');
  }
  for (const name of hint === 'string'
    ? ['toString', 'valueOf']
    : ['valueOf', 'toString']) {
    const method = value[name];
    if (typeof method === 'function') {
      const result = method.call(value);
      if (
        result === null ||
        (typeof result !== 'object' && typeof result !== 'function')
      ) {
        return result;
      }
    }
  }
  throw new TypeError('Cannot convert object to primitive value');
}

function candidates(file) {
  const extension = path.extname(file);
  const base = EXTENSIONS.includes(extension)
    ? file.slice(0, -extension.length)
    : file;
  return [file, ...EXTENSIONS.map((suffix) => base + suffix)];
}

function aliasedPaths(source, aliases) {
  const result = [source];
  for (const [alias, replacements] of Object.entries(aliases || {})) {
    const values = Array.isArray(replacements) ? replacements : [replacements];
    const [before, after] = alias.split('*');
    if (alias.includes('*')) {
      if (source.startsWith(before) && source.endsWith(after)) {
        const middle = source.slice(
          before.length,
          after.length ? -after.length : undefined,
        );
        for (const replacement of values)
          result.push(replacement.split('*').join(middle));
      }
    } else if (alias === source) {
      result.push(...values);
    }
  }
  return result;
}

function resolveFile(source, filename, aliases, rootDir) {
  const { moduleResolve } = require('@dual-bundle/import-meta-resolve');
  for (const candidate of candidates(source)) {
    if (candidate.startsWith('.')) {
      try {
        return url.fileURLToPath(
          moduleResolve(candidate, url.pathToFileURL(filename)),
        );
      } catch {
        continue;
      }
    }
    for (const alias of aliasedPaths(candidate, aliases)) {
      if (alias.startsWith('/ROOT/') && rootDir != null) {
        const found = candidates(path.join(rootDir, alias.slice(6))).find(
          (file) => fs.existsSync(file),
        );
        if (found) return found;
        continue;
      }
      if (path.isAbsolute(alias)) {
        const found = candidates(alias).find((file) => fs.existsSync(file));
        if (found) return found;
        continue;
      }
      try {
        return url.fileURLToPath(
          moduleResolve(alias, url.pathToFileURL(filename)),
        );
      } catch {
        continue;
      }
    }
  }
  return undefined;
}

function createHost(options, filename, binding) {
  // Values the compiler cannot copy (functions, class instances, Map/Date,
  // cycles, symbols) stay here; the compiler reads them lazily by id. The same
  // JavaScript value always has the same id within a transform.
  const handles = [];
  const handleIds = new Map();
  function handle(value) {
    let id = handleIds.get(value);
    if (id == null) {
      id = String(handles.length);
      handles.push(value);
      handleIds.set(value, id);
    }
    return { type: 'handle', value: id };
  }
  function isPlain(value) {
    if (Array.isArray(value)) return true;
    const prototype = Object.getPrototypeOf(value);
    return (
      (prototype === Object.prototype || prototype === null) &&
      Object.getOwnPropertySymbols(value).length === 0
    );
  }
  function encode(value, path = new Set()) {
    if (value === undefined) return { type: 'undefined' };
    if (value === null) return { type: 'null' };
    if (typeof value === 'boolean') return { type: 'boolean', value };
    if (typeof value === 'number') {
      return {
        type: 'number',
        value: Object.is(value, -0) ? '-0' : String(value),
      };
    }
    if (typeof value === 'string') return { type: 'string', value };
    if (typeof value !== 'object' || path.has(value) || !isPlain(value)) {
      return handle(value);
    }
    path.add(value);
    const result = Array.isArray(value)
      ? { type: 'array', value: value.map((item) => encode(item, path)) }
      : {
          type: 'object',
          value: Object.entries(value).map(([key, item]) => [
            key,
            encode(item, path),
          ]),
        };
    path.delete(value);
    return result;
  }
  // Compiler functions passed to JavaScript run back on the compiler thread.
  const compilerFunctions = new Map();
  function compilerFunction(id) {
    let wrapper = compilerFunctions.get(id);
    if (wrapper == null) {
      wrapper = function (...args) {
        let result;
        try {
          result = binding.callCompilerFunction(
            id,
            encode(this),
            args.map((arg) => encode(arg)),
          );
        } catch (error) {
          // The compiler function failed because user code it called threw:
          // that exception now propagates through the caller, which may catch it.
          if (state.hasThrown) {
            const thrown = state.thrown;
            state.hasThrown = false;
            state.thrown = undefined;
            throw thrown;
          }
          throw error;
        }
        return decode(result);
      };
      compilerFunctions.set(id, wrapper);
    }
    return wrapper;
  }
  function decode(value) {
    switch (value.type) {
      case 'undefined':
        return undefined;
      case 'null':
        return null;
      case 'boolean':
      case 'string':
        return value.value;
      case 'number':
        return Number(value.value);
      case 'array':
        return value.value.map(decode);
      case 'object':
        return Object.fromEntries(
          value.value.map(([key, item]) => [key, decode(item)]),
        );
      case 'handle':
        if (!Object.hasOwn(handles, value.value)) {
          throw new TypeError(`Unknown StyleX host value ${value.value}`);
        }
        return handles[value.value];
      case 'closure':
        return compilerFunction(value.value);
      default:
        throw new TypeError(`Unknown StyleX host value: ${value.type}`);
    }
  }
  const functions = new Map();
  const functionsByIdentity = new Map();
  const seen = new Set();
  // Callbacks and non-JSON numbers are tagged in-band, so user data must never
  // be able to spell a tag (for example an `env` object loaded from JSON).
  const RESERVED = [
    '__stylexCallback',
    '__stylexUndefined',
    '__stylexNumber',
    '__stylexHandle',
  ];
  const { isProxy } = require('node:util').types;
  // `env` values are read by compiled code, so anything a copy would not
  // preserve (class instances, Map/Date, proxies, symbols, holes, cycles, very
  // large arrays) stays a live JavaScript value the compiler reads lazily.
  function opaqueEnvValue(value) {
    if (typeof value === 'symbol' || typeof value === 'bigint') return true;
    if (value == null || typeof value !== 'object') return false;
    if (isProxy(value) || seen.has(value)) return true;
    if (Array.isArray(value)) {
      if (value.length > 10000) return true;
      for (let index = 0; index < value.length; index++) {
        if (!(index in value)) return true;
      }
      return Object.getPrototypeOf(value) !== Array.prototype;
    }
    return !isPlain(value);
  }
  function serialize(value, receiver, inEnv = false) {
    if (inEnv && opaqueEnvValue(value)) {
      return { __stylexHandle: handle(value).value };
    }
    if (typeof value === 'function') {
      let id = functionsByIdentity.get(value);
      if (id == null) {
        id = String(functions.size);
        functions.set(id, { callback: value, receiver });
        functionsByIdentity.set(value, id);
      }
      return { __stylexCallback: id };
    }
    if (value === undefined) return { __stylexUndefined: true };
    if (
      typeof value === 'number' &&
      (Object.is(value, -0) || !Number.isFinite(value))
    ) {
      return { __stylexNumber: Object.is(value, -0) ? '-0' : String(value) };
    }
    if (value != null && typeof value === 'object') {
      if (seen.has(value))
        throw new TypeError('StyleX options contain a cyclic object');
      const reserved = RESERVED.find((key) => Object.hasOwn(value, key));
      if (reserved != null) {
        throw new TypeError(
          `StyleX options cannot contain the key ${reserved}`,
        );
      }
      seen.add(value);
      const result = Array.isArray(value)
        ? value.map((item) => serialize(item, value, inEnv))
        : Object.fromEntries(
            Object.entries(value).map(([key, item]) => [
              key,
              serialize(
                item,
                value,
                inEnv || (value === options && key === 'env'),
              ),
            ]),
          );
      seen.delete(value);
      return result;
    }
    return value;
  }
  const serialized = serialize(options);
  const resolutionCache = new Map();
  const canonicalCache = new Map();
  function canonical(file) {
    if (canonicalCache.has(file)) return canonicalCache.get(file);
    const mode = options.unstable_moduleResolution || {};
    if (mode.type === 'custom') return mode.getCanonicalFilePath(file);
    let directory = path.dirname(file);
    let result;
    for (;;) {
      const manifest = path.join(directory, 'package.json');
      if (fs.existsSync(manifest)) {
        try {
          const pkg = JSON.parse(fs.readFileSync(manifest, 'utf8'));
          result = `${pkg.name}:${path.relative(directory, file)}`;
        } catch (error) {
          console.error(error);
        }
        break;
      }
      const parent = path.dirname(directory);
      if (parent === directory || directory === '') break;
      directory = parent;
    }
    if (result == null) {
      result =
        mode.rootDir != null
          ? path.relative(mode.rootDir, file)
          : `_unknown_path_:${path.basename(file)}`;
    }
    canonicalCache.set(file, result);
    return result;
  }
  function resolve(source, from = filename) {
    const key = `${from}\0${source}`;
    if (resolutionCache.has(key)) return resolutionCache.get(key);
    const mode = options.unstable_moduleResolution || {};
    const resolved =
      mode.type === 'custom'
        ? mode.filePathResolver(source, from, options.aliases)
        : resolveFile(source, from, options.aliases, mode.rootDir);
    resolutionCache.set(key, resolved);
    return resolved;
  }
  const packageCache = new Map();
  function packageDetails(file) {
    if (packageCache.has(file)) return packageCache.get(file);
    let directory = path.dirname(file);
    let result;
    for (;;) {
      const manifest = path.join(directory, 'package.json');
      if (fs.existsSync(manifest)) {
        try {
          result = [
            JSON.parse(fs.readFileSync(manifest, 'utf8')).name,
            directory,
          ];
        } catch (error) {
          console.error(error);
        }
        break;
      }
      const parent = path.dirname(directory);
      if (parent === directory || directory === '') break;
      directory = parent;
    }
    packageCache.set(file, result);
    return result;
  }
  function debugFile(file) {
    if (typeof options.debugFilePath === 'function') {
      // Babel interpolates the result into `${name}:${line}` unless it is ''.
      const name = options.debugFilePath(file);
      return name === '' ? '' : `${name}`;
    }
    const current = packageDetails(process.cwd());
    const pkg = packageDetails(file);
    if (pkg) {
      const relative = path.relative(pkg[1], file);
      return current && current[0] === pkg[0]
        ? relative
        : `${pkg[0]}:${relative}`;
    }
    const mode = options.unstable_moduleResolution || {};
    const short =
      mode.type === 'commonJS' &&
      typeof mode.rootDir === 'string' &&
      mode.rootDir
        ? path.relative(mode.rootDir, file)
        : file.split(path.sep).slice(-2).join('/');
    const index = file.indexOf('node_modules');
    if (index !== -1) {
      const name = file
        .slice(index + 'node_modules'.length + 1)
        .split(path.sep)[0];
      if (name) return `${name}:${short}`;
    }
    return mode.type === 'haste' ? path.basename(file) : short;
  }
  // The first exception thrown by user code; the facade rethrows it unchanged
  // once the native call returns, exactly as Babel would have propagated it.
  const state = { thrown: undefined, hasThrown: false };
  function run(id, args) {
    if (id === '@callbackArity') {
      const entry = functions.get(String(args[0]));
      if (entry == null)
        throw new TypeError(`Unknown StyleX host callback ${args[0]}`);
      return entry.callback.length;
    }
    if (id === '@callbackValue') {
      const entry = functions.get(String(args[0]));
      if (entry == null)
        throw new TypeError(`Unknown StyleX host callback ${args[0]}`);
      return entry.callback;
    }
    if (id === '@get') return args[0][args[1]];
    if (id === '@call') return Reflect.apply(args[0], args[1], args.slice(2));
    if (id === '@typeof') return typeof args[0];
    if (id === '@toPrimitive') return toPrimitive(args[0], args[1]);
    if (id === '@toString') return String(args[0]);
    if (id === '@toNumber') return Number(args[0]);
    if (id === '@truthy') return Boolean(args[0]);
    if (id === '@sameValue') return Object.is(args[0], args[1]);
    if (id === '@binary') return binary(args[0], args[1], args[2]);
    if (id === '@unary') return unary(args[0], args[1]);
    if (id === '@debugFile') return debugFile(args[0]);
    // Babel only tests the resolver's result for truthiness and later uses the
    // canonical path in string concatenation, so custom resolvers may return
    // any value.
    if (id === '@resolveImport') {
      const resolved = resolve(args[0], args[1]);
      return resolved ? `${canonical(resolved)}` : undefined;
    }
    if (id === '@canonicalFile') return `${canonical(args[0])}`;
    if (id === '@rewriteImport') {
      const resolved = resolve(args[0], args[1]);
      if (!resolved) return undefined;
      let result = path
        .relative(path.dirname(args[1]), resolved)
        .split(path.sep)
        .join('/');
      const extension = EXTENSIONS.find((suffix) => result.endsWith(suffix));
      if (extension) result = result.slice(0, -extension.length);
      return result.startsWith('.') ? result : `./${result}`;
    }
    const entry = functions.get(id);
    if (entry == null)
      throw new TypeError(`Unknown StyleX host callback ${id}`);
    return entry.callback.apply(entry.receiver, args);
  }
  function invoke(id, encodedArgs) {
    try {
      return encode(run(id, encodedArgs.map(decode)));
    } catch (error) {
      if (!state.hasThrown) {
        state.hasThrown = true;
        state.thrown = error;
      }
      // Not a host value: the compiler treats it as a failed callback.
      return { type: 'thrown' };
    }
  }
  return { options: serialized, invoke, state };
}

module.exports = { createHost };
