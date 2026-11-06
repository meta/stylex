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

const { isProxy } = require('node:util').types;
// The kind letter that starts a handle id; never runs user code (traps, getters).
function handleKind(value) {
  switch (typeof value) {
    case 'function':
      return 'f';
    case 'symbol':
      return 's';
    case 'bigint':
      return 'b';
    default: {
      if (isProxy(value)) return 'x';
      if (Array.isArray(value)) return 'a';
      const prototype = Object.getPrototypeOf(value);
      return prototype === Object.prototype || prototype === null ? 'p' : 'o';
    }
  }
}
// Marks an already-encoded structural reply of a host service.
const RAW = Symbol('raw host value');

// A lone surrogate: JavaScript strings may hold one, UTF-8 cannot.
const LONE_SURROGATE =
  /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/;
function codeUnits(value) {
  const units = new Array(value.length);
  for (let index = 0; index < value.length; index++) {
    units[index] = value.charCodeAt(index);
  }
  return units;
}
function fromCodeUnits(units) {
  let result = '';
  for (let index = 0; index < units.length; index += 8192) {
    result += String.fromCharCode.apply(null, units.slice(index, index + 8192));
  }
  return result;
}
// Turns the native too-shallow-stack marker into JavaScript's own RangeError.
function stackOverflow(error) {
  return (
    error != null &&
    typeof error.message === 'string' &&
    error.message.includes('STYLEX_STACK_OVERFLOW')
  );
}

// Binary operators the compiler may apply to host values (never evaluated text).
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

// The specifier `rewriteAliases` writes for a resolved theme file, matching the
// Babel plugin's getRelativePath (state-manager.js) minus the code extension.
function rewrittenImportPath(from, resolved, pathImpl = path) {
  let result = pathImpl
    .relative(pathImpl.parse(from).dir, resolved)
    .split(pathImpl.sep)
    .join(path.posix.sep);
  if (!result.startsWith('.')) result = `./${result}`;
  const extension = EXTENSIONS.find((suffix) => result.endsWith(suffix));
  if (extension != null) result = result.slice(0, -extension.length);
  return result;
}

// The canonical name a theme file's variables hash under, as the Babel plugin's
// getCanonicalFilePath computes it; an unparsable package.json is reported.
function canonicalFilePath(
  file,
  rootDir,
  pathImpl = path,
  files = {
    exists: (manifest) => fs.existsSync(manifest),
    read: (manifest) => fs.readFileSync(manifest, 'utf8'),
  },
) {
  let folder = pathImpl.dirname(file);
  for (;;) {
    const manifest = pathImpl.join(folder, 'package.json');
    if (files.exists(manifest)) {
      try {
        const { name } = JSON.parse(files.read(manifest));
        return `${name}:${pathImpl.relative(folder, file)}`;
      } catch (error) {
        console.error(error);
        break;
      }
    }
    const parent = pathImpl.dirname(folder);
    if (
      folder === pathImpl.parse(folder).root ||
      folder === '' ||
      parent === folder
    ) {
      break;
    }
    folder = parent;
  }
  if (rootDir != null) return pathImpl.relative(rootDir, file);
  return `_unknown_path_:${pathImpl.relative(pathImpl.dirname(file), file)}`;
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

// Native state of a compiler function lives until its JS wrapper is collected.
const releases =
  typeof FinalizationRegistry === 'function'
    ? new FinalizationRegistry(({ binding, id }) => {
        try {
          binding.releaseCompilerFunction(id);
        } catch {
          // The compiler is already gone (the thread is exiting).
        }
      })
    : null;

function createHost(options, filename, binding) {
  // Non-primitive values stay live here and are read lazily by id, as Babel's
  // evaluator reads them; the same value always has the same id.
  const handles = new Map();
  const handleIds = new Map();
  function handle(value) {
    let id = handleIds.get(value);
    if (id == null) {
      id = `${handleKind(value)}${handles.size}`;
      handles.set(id, value);
      handleIds.set(value, id);
    }
    return { type: 'handle', value: id };
  }
  // Primitives cross as themselves; objects become `{ type, value }` records.
  function encode(value) {
    const kind = typeof value;
    return kind === 'string' ||
      kind === 'number' ||
      kind === 'boolean' ||
      value == null
      ? value
      : handle(value);
  }
  // Option data as records: numbers as text, lone-surrogate strings as code units.
  function encodeData(value) {
    if (value === undefined) return { type: 'undefined' };
    if (value === null) return { type: 'null' };
    if (typeof value === 'boolean') return { type: 'boolean', value };
    if (typeof value === 'number') {
      return {
        type: 'number',
        value: Object.is(value, -0) ? '-0' : String(value),
      };
    }
    if (typeof value === 'string') {
      return LONE_SURROGATE.test(value)
        ? { type: 'utf16', value: codeUnits(value) }
        : { type: 'string', value };
    }
    return handle(value);
  }
  // One-level views of values the compiler traverses, read once in JS order.
  const MAX_ENTRIES = 262144;
  function entriesOf(value) {
    const keys = Object.keys(value);
    if (keys.length > MAX_ENTRIES) {
      throw new RangeError(
        `StyleX cannot read an object with more than ${MAX_ENTRIES} properties`,
      );
    }
    return keys.map((key) => [key, encode(value[key])]);
  }
  function arrayItems(items) {
    if (items.length > MAX_ENTRIES) {
      throw new RangeError(
        `StyleX cannot read an array with more than ${MAX_ENTRIES} items`,
      );
    }
    const result = new Array(items.length);
    for (let index = 0; index < items.length; index++) {
      result[index] = encode(items[index]);
    }
    return { type: 'array', value: result };
  }
  // Result values: arrays and plain objects become data; anything else stays live.
  function materialize(value) {
    if (value === null || typeof value !== 'object') return { type: 'null' };
    if (Array.isArray(value)) {
      const length = value.length;
      if (length > MAX_ENTRIES) arrayItems({ length });
      const items = new Array(length);
      for (let index = 0; index < length; index++) items[index] = value[index];
      return {
        type: 'array',
        value: [{ type: 'string', value: 'array' }, arrayItems(items)],
      };
    }
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) {
      return { type: 'null' };
    }
    return {
      type: 'array',
      value: [
        {
          type: 'string',
          value: prototype === null ? 'nullObject' : 'object',
        },
        { type: 'object', value: entriesOf(value) },
      ],
    };
  }
  // Creates the live value of an evaluated object or array literal from data;
  // an own `__proto__` key stays a data property and nothing is evaluated.
  function adopt(kind, contents, holes = []) {
    if (kind === 'array') {
      if (!Array.isArray(contents)) {
        throw new TypeError('StyleX array contents must be an array');
      }
      const skip = new Set(holes);
      const result = new Array(contents.length);
      for (let index = 0; index < contents.length; index++) {
        if (!skip.has(index)) result[index] = contents[index];
      }
      return result;
    }
    if (kind !== 'object' && kind !== 'nullObject') {
      throw new TypeError(`Unsupported StyleX value kind ${String(kind)}`);
    }
    const result = kind === 'object' ? {} : Object.create(null);
    for (const [key, item] of Object.entries(contents)) {
      Object.defineProperty(result, key, {
        value: item,
        writable: true,
        enumerable: true,
        configurable: true,
      });
    }
    return result;
  }
  // Pure, allowlisted Object/Array statics the evaluator may run on live values.
  const STATIC_BUILTINS = {
    Object: {
      keys: 'array',
      values: 'array',
      entries: 'pairs',
      getOwnPropertyNames: 'array',
      getOwnPropertySymbols: 'array',
      hasOwn: 'value',
      isExtensible: 'value',
      isFrozen: 'value',
      isSealed: 'value',
      fromEntries: 'object',
      groupBy: 'value',
    },
    Array: { from: 'array' },
    String: { raw: 'value' },
  };
  function staticBuiltin(namespace, method, args) {
    const shape =
      Object.hasOwn(STATIC_BUILTINS, namespace) &&
      Object.hasOwn(STATIC_BUILTINS[namespace], method)
        ? STATIC_BUILTINS[namespace][method]
        : null;
    if (shape == null) {
      throw new TypeError(`Unsupported StyleX built-in ${namespace}.${method}`);
    }
    const result = { Object, Array, String }[namespace][method](...args);
    switch (shape) {
      case 'array':
        return arrayItems(result);
      case 'pairs':
        return {
          type: 'array',
          value: result.map((pair) => arrayItems(pair)),
        };
      case 'object':
        return { type: 'object', value: entriesOf(result) };
      default:
        return encode(result);
    }
  }
  // Compiler functions run back on the compiler thread, even after their transform.
  const compilerFunctions = new Map();
  // Failure ids of exceptions this host raised, recognized when they come back.
  const compilerFailures = new WeakMap();
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
            invoke,
          );
        } catch (error) {
          if (stackOverflow(error)) {
            throw new RangeError('Maximum call stack size exceeded');
          }
          // User code the compiler function called threw: propagate that exception.
          if (state.hasThrown) {
            const thrown = state.thrown;
            state.hasThrown = false;
            state.thrown = undefined;
            throw thrown;
          }
          // Addon errors carry the N-API status as `code`; anything else is user JS.
          if (typeof error?.code === 'string') throw new Error(error.message);
          throw error;
        }
        if (result?.type === 'failed') {
          // A plain Error with the compiler's message, as the Babel plugin throws it.
          if (state.hasThrown) {
            const thrown = state.thrown;
            state.hasThrown = false;
            state.thrown = undefined;
            throw thrown;
          }
          const failure = new Error(result.message);
          compilerFailures.set(failure, result.cause);
          throw failure;
        }
        return decode(result);
      };
      binding.retainCompilerFunction(id);
      releases?.register(wrapper, { binding, id });
      compilerFunctions.set(id, wrapper);
    }
    return wrapper;
  }
  function decode(value) {
    if (value === null || typeof value !== 'object') return value;
    switch (value.type) {
      case 'undefined':
        return undefined;
      case 'null':
        return null;
      case 'boolean':
      case 'string':
        return value.value;
      case 'utf16':
        return fromCodeUnits(value.value);
      case 'number':
        return Number(value.value);
      case 'array':
        return value.value.map(decode);
      case 'object':
        return Object.fromEntries(
          value.value.map(([key, item]) => [key, decode(item)]),
        );
      case 'handle':
        if (!handles.has(value.value)) {
          throw new TypeError(`Unknown StyleX host value ${value.value}`);
        }
        return handles.get(value.value);
      case 'closure':
        return compilerFunction(value.value);
      default:
        throw new TypeError(`Unknown StyleX host value: ${value.type}`);
    }
  }
  const functions = new Map();
  // One id per (function, receiver), so each read calls it with its own `this`.
  const functionsByIdentity = new Map();
  const seen = new Set();
  // Option data must never spell an in-band tag; `env` is never serialized.
  const RESERVED = [
    '__stylexCallback',
    '__stylexUndefined',
    '__stylexNumber',
    '__stylexHandle',
  ];
  // Like Babel, `env` becomes a frozen shallow copy once per transform.
  function isEnvObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
  }
  function serializeEnv(env) {
    const frozen = Object.freeze({ ...env });
    return {
      __stylexHandle: handle(frozen).value,
      __stylexEntries: Object.keys(frozen).map((key) => [
        key,
        encodeData(frozen[key]),
      ]),
    };
  }
  function serialize(value, receiver) {
    if (typeof value === 'function') {
      let byReceiver = functionsByIdentity.get(value);
      if (byReceiver == null) {
        byReceiver = new Map();
        functionsByIdentity.set(value, byReceiver);
      }
      let id = byReceiver.get(receiver);
      if (id == null) {
        id = String(functions.size);
        functions.set(id, { callback: value, receiver });
        byReceiver.set(receiver, id);
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
        ? value.map((item) => serialize(item, value))
        : Object.fromEntries(
            Object.entries(value).map(([key, item]) => [
              key,
              value === options && key === 'env' && isEnvObject(item)
                ? serializeEnv(item)
                : serialize(item, value),
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
    const result = canonicalFilePath(file, mode.rootDir);
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
      // Called once, as in Babel; a non-string result is always stringified here.
      const name = options.debugFilePath(file);
      return typeof name === 'string' ? name : `${name}`;
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
  // The first user exception, which the facade rethrows unchanged, as Babel would.
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
    if (id === '@materialize') return { [RAW]: materialize(args[0]) };
    if (id === '@ownEntries') {
      return { [RAW]: { type: 'object', value: entriesOf(args[0]) } };
    }
    if (id === '@iterate') return { [RAW]: arrayItems([...args[0]]) };
    if (id === '@isArray') return Array.isArray(args[0]);
    if (id === '@adopt') return adopt(args[0], args[1], args[2]);
    if (id === '@staticBuiltin') {
      return { [RAW]: staticBuiltin(args[0], args[1], args.slice(2)) };
    }
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
    // Babel only tests a resolver's result for truthiness, so any value is allowed.
    if (id === '@resolveImport') {
      const resolved = resolve(args[0], args[1]);
      return resolved ? `${canonical(resolved)}` : undefined;
    }
    if (id === '@canonicalFile') return `${canonical(args[0])}`;
    // Like the Babel plugin: aliases via its commonJS resolver, never a custom one.
    if (id === '@rewriteImport') {
      const [source, from] = args;
      const resolved = resolveFile(source, from, options.aliases);
      if (resolved == null) return undefined;
      return rewrittenImportPath(from, resolved);
    }
    const entry = functions.get(id);
    if (entry == null)
      throw new TypeError(`Unknown StyleX host callback ${id}`);
    return entry.callback.apply(entry.receiver, args);
  }
  function invoke(id, encodedArgs) {
    try {
      const result = run(id, encodedArgs.map(decode));
      return result !== null &&
        typeof result === 'object' &&
        Object.hasOwn(result, RAW)
        ? result[RAW]
        : encode(result);
    } catch (error) {
      if (!state.hasThrown) {
        state.hasThrown = true;
        state.thrown = error;
      }
      // Not a host value: the compiler treats it as a failed callback.
      return compilerFailures.has(error)
        ? { type: 'thrown', cause: compilerFailures.get(error) }
        : { type: 'thrown' };
    }
  }
  return { options: serialized, invoke, state };
}

module.exports = {
  createHost,
  stackOverflow,
  rewrittenImportPath,
  canonicalFilePath,
};
