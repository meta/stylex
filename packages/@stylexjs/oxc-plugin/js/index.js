'use strict';
// Node entry for @stylexjs/oxc-plugin.
//
//   transformSync(code, options) -> { code, map, metadata: { stylex }, warnings, errors }
//   createCompiler(options)      -> { transform(code, filename), errors }   (batch-friendly)
//   processStylexRules(rules, config) -> css
//   babelPlugin                  -> the Babel plugin (babel.js)
//
// `options` are the Babel plugin options plus `filename`. Options are validated once per
// distinct options value (Babel's messages and fall-back defaults, see options.js) and turned
// into one native Compiler, so a build pays the option cost once, not per file.
const path = require('node:path');
const { normalizeOptions, encodeValue, decodeValue } = require('./options.js');

const ADDON = process.env.OXS_ADDON || path.join(__dirname, 'stylex_oxc.node');
// Loaded with this module, as napi packages usually do: loading the native code (about 1 ms)
// is part of startup, not of the first file's transform.
const native = require(ADDON);
function addon() {
  return native;
}

// Keys the harness adds to the plugin options. Real callers never pass them.
const HARNESS_KEYS = ['__sourceType', '__parserPlugins', '__harness'];
// Per-file data never keys a compiler.
const SKIP_KEYS = new Set([...HARNESS_KEYS, 'filename', 'inputSourceMap']);

// ---------------------------------------------------------------- JS callbacks

// Babel resolves commonJS imports with `moduleResolve` from @dual-bundle/import-meta-resolve.
// Native resolution cannot see a Jest mock of that module, so when it is mocked the commonJS
// resolver runs here, with Babel's algorithm. Outside Jest this costs nothing.
function mockedModuleResolve() {
  if (typeof jest === 'undefined') return null;
  for (const from of [__dirname, process.cwd()]) {
    try {
      const mod = require(require.resolve('@dual-bundle/import-meta-resolve', { paths: [from] }));
      if (mod && mod.moduleResolve && mod.moduleResolve._isMockFunction === true) return mod;
      return null;
    } catch {}
  }
  return null;
}

// callJs(id, argsJson) -> resultJson, the one callback Rust uses (INTERFACES.md).
function makeCallJs(fns) {
  return (id, argsJson) => {
    const entry = fns[id];
    try {
      const args = decodeValue(JSON.parse(argsJson));
      return JSON.stringify(encodeValue(entry.fn.apply(entry.self, args), fns));
    } catch (e) {
      return JSON.stringify({ __oxsErr: String(e && e.message !== undefined ? e.message : e) });
    }
  };
}

// ---------------------------------------------------------------- compilers

const fnIds = new WeakMap();
let nextFnId = 0;
// A string that is equal for equal options. Functions count by identity.
function keyOf(v, seen) {
  switch (typeof v) {
    case 'string':
      return JSON.stringify(v);
    case 'number':
    case 'boolean':
    case 'bigint':
      return `${(typeof v)[0]}${String(v)}`;
    case 'undefined':
      return 'u';
    case 'function':
      if (!fnIds.has(v)) fnIds.set(v, ++nextFnId);
      return `f${fnIds.get(v)}`;
    case 'symbol':
      return 's';
  }
  if (v === null) return 'n';
  if (v instanceof RegExp) return `r${String(v)}`;
  if (seen.has(v)) return 'c';
  seen.add(v);
  let out;
  if (Array.isArray(v)) {
    out = '[';
    for (const x of v) out += `${keyOf(x, seen)},`;
    out += ']';
  } else {
    out = '{';
    for (const k of Object.keys(v)) out += `${JSON.stringify(k)}:${keyOf(v[k], seen)},`;
    out += '}';
  }
  seen.delete(v);
  return out;
}

function cacheKey(options) {
  let out = '{';
  for (const k of Object.keys(options)) {
    if (SKIP_KEYS.has(k)) continue;
    out += `${JSON.stringify(k)}:${keyOf(options[k], new Set())},`;
  }
  return `${out}}`;
}

const byKey = new Map();
const byObject = new WeakMap();
const MAX_COMPILERS = 256;

function build(options) {
  const { native: nativeOptions, errors, fns, aliases } = normalizeOptions(stripKeys(options));
  const mr = nativeOptions.unstable_moduleResolution;
  if (mr != null && (mr.type === 'commonJS' || mr.type === 'experimental_crossFileParsing')) {
    const mod = mockedModuleResolve();
    if (mod != null) {
      const resolver = require('./resolve.js').commonJSResolver(mod, mr.rootDir);
      fns.push({ fn: (importPath, sourceFilePath) => resolver(importPath, sourceFilePath, aliases), self: undefined });
      mr.filePathResolver = { __oxsFn: fns.length - 1 };
    }
  }
  return {
    compiler: new (addon().Compiler)(JSON.stringify(nativeOptions)),
    errors,
    callJs: fns.length > 0 ? makeCallJs(fns) : undefined,
  };
}

function stripKeys(options) {
  const out = {};
  for (const k of Object.keys(options || {})) if (!SKIP_KEYS.has(k)) out[k] = options[k];
  return out;
}

// Babel reads the plugin options for every file, so an options object changed in place between
// files takes effect. Compilers are cached, so each cache hit is confirmed against a snapshot of
// the options' contents: a flat list of every key and leaf value, compared with === in one walk.
// Functions compare by identity. This allocates nothing and costs well under a microsecond for
// usual options. Deeper than SNAP_DEPTH, a value compares by identity.
const SNAP_DEPTH = 8;
const OPEN = Symbol('open');
const CLOSE = Symbol('close');

function snapValue(v, out, depth) {
  if (v === null || typeof v !== 'object' || depth >= SNAP_DEPTH || v instanceof RegExp) {
    out.push(v);
    return;
  }
  out.push(OPEN);
  if (Array.isArray(v)) {
    out.push(v.length);
    for (let i = 0; i < v.length; i++) snapValue(v[i], out, depth + 1);
  } else {
    for (const k in v) {
      out.push(k);
      snapValue(v[k], out, depth + 1);
    }
  }
  out.push(CLOSE);
}

function snapshot(opts) {
  const out = [];
  for (const k in opts) {
    if (SKIP_KEYS.has(k)) continue;
    out.push(k);
    snapValue(opts[k], out, 1);
  }
  return out;
}

// Walks `v` in snapshot order from index `i`. Returns the next index, or -1 on a difference.
function matchValue(v, snap, i, depth) {
  if (v === null || typeof v !== 'object' || depth >= SNAP_DEPTH || v instanceof RegExp) {
    return snap[i] === v ? i + 1 : -1;
  }
  if (snap[i] !== OPEN) return -1;
  i++;
  if (Array.isArray(v)) {
    if (snap[i] !== v.length) return -1;
    i++;
    for (let j = 0; j < v.length; j++) {
      i = matchValue(v[j], snap, i, depth + 1);
      if (i < 0) return -1;
    }
  } else {
    for (const k in v) {
      if (snap[i] !== k) return -1;
      i = matchValue(v[k], snap, i + 1, depth + 1);
      if (i < 0) return -1;
    }
  }
  return snap[i] === CLOSE ? i + 1 : -1;
}

function matches(snap, opts) {
  let i = 0;
  for (const k in opts) {
    if (SKIP_KEYS.has(k)) continue;
    if (snap[i] !== k) return false;
    i = matchValue(opts[k], snap, i + 1, 1);
    if (i < 0) return false;
  }
  return i === snap.length;
}

// The last few option sets seen. Callers that spread one base options object per file
// (`{...base, filename}`) hit this without building a key. A few slots cover a tool that
// alternates between a handful of option sets (dev and prod builds, several plugins).
const RECENT_SLOTS = 4;
const recent = [];
function recentEntry(opts) {
  for (let i = 0; i < recent.length; i++) {
    const slot = recent[i];
    if (matches(slot.snap, opts)) {
      if (i > 0) {
        recent.splice(i, 1);
        recent.unshift(slot);
      }
      return slot.entry;
    }
  }
  return null;
}
function remember(snap, entry) {
  recent.unshift({ snap, entry });
  if (recent.length > RECENT_SLOTS) recent.pop();
}

function entryFor(options) {
  const opts = options || {};
  const own = byObject.get(opts);
  if (own != null && matches(own.snap, opts)) return own.entry;
  let entry = recentEntry(opts);
  let snap = null;
  if (entry == null) {
    entry = keyed(opts);
    snap = snapshot(opts);
    remember(snap, entry);
  }
  // Reuse by identity only for objects without per-file data, after a fresh check above.
  if (opts.filename === undefined && opts.__parserPlugins === undefined) {
    byObject.set(opts, { snap: snap ?? snapshot(opts), entry });
  }
  return entry;
}

function keyed(opts) {
  const key = cacheKey(opts);
  let entry = byKey.get(key);
  if (entry == null) {
    // Evict the oldest compiler. Maps iterate in insertion order.
    if (byKey.size >= MAX_COMPILERS) byKey.delete(byKey.keys().next().value);
    entry = build(opts);
    byKey.set(key, entry);
  }
  return entry;
}

function toError(e) {
  const [message, loc] = String(e.message).split('\u0000');
  const err = new Error(message);
  if (loc) {
    const [l1, c1, l2, c2] = loc.split(':').map(Number);
    err.loc = { line: l1, column: c1 };
    err.endLoc = { line: l2, column: c2 };
  }
  return err;
}

// `inputSourceMap`: a map object or JSON text to compose with, or `true` to read the file's own
// sourceMappingURL comment (emit). Passed per file as text.
function inputMapText(m) {
  if (m == null || m === false) return undefined;
  if (m === true) return 'true';
  return typeof m === 'string' ? m : JSON.stringify(m);
}

// Non-finite numbers in defineConsts values arrive tagged (host.rs const_val_json).
const NON_FINITE = { NaN: Number.NaN, Infinity: Number.POSITIVE_INFINITY, '-Infinity': Number.NEGATIVE_INFINITY };
function untag(_key, v) {
  if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
    const keys = Object.keys(v);
    if (keys.length === 1 && keys[0] === '__oxs' && Object.hasOwn(NON_FINITE, v.__oxs)) return NON_FINITE[v.__oxs];
  }
  return v;
}
// The packed form (host.rs rules_packed): \u0002, then class \0 ltr \0 rtl \0 priority per
// rule, joined by \0, with \u0001 for a null rtl. Splitting is about 3x faster than JSON.parse.
function unpackRules(text) {
  const f = text.slice(1).split('\u0000');
  const out = new Array(f.length / 4);
  for (let i = 0, j = 0; i < f.length; i += 4, j++) {
    const rtl = f[i + 2];
    out[j] = [f[i], { ltr: f[i + 1], rtl: rtl === '\u0001' ? null : rtl }, +f[i + 3]];
  }
  return out;
}
function parseRules(text, tagged) {
  if (text === '[]') return [];
  if (text.charCodeAt(0) === 2) return unpackRules(text);
  return tagged ? JSON.parse(text, untag) : JSON.parse(text);
}

function run(entry, code, filename, syntaxHint, inputSourceMap) {
  let out;
  try {
    out = entry.compiler.transform(
      code,
      filename == null ? undefined : String(filename),
      entry.callJs,
      filename == null ? syntaxHint : undefined,
      inputMapText(inputSourceMap),
    );
  } catch (e) {
    throw toError(e);
  }
  const mapText = out.map;
  const result = {
    code: out.code ?? code,
    map: null,
    metadata: { stylex: parseRules(out.rules, out.tagged) },
    warnings: out.warnings,
    errors: entry.errors,
  };
  if (mapText != null) {
    let parsed;
    Object.defineProperty(result, 'map', {
      enumerable: true,
      configurable: true,
      get: () => (parsed === undefined ? (parsed = JSON.parse(mapText)) : parsed),
    });
  }
  // UTF-8 byte ranges [start, end, ...] of text that replaced a source span (js/babel.js).
  if (out.generated != null) Object.defineProperty(result, 'generated', { value: out.generated });
  return result;
}

function parserPlugins(options) {
  return ((options && options.__parserPlugins) || []).map((p) => (Array.isArray(p) ? p[0] : p));
}

/**
 * transformSync(code, options) -> { code, map, metadata: { stylex }, warnings, errors }
 * Errors carry `loc` ({line, column}) and `endLoc` when the compiler has a span.
 * `errors` are Babel's option diagnostics (it logs them with console.error for every file).
 */
function transformSync(code, options = {}) {
  return run(
    entryFor(withParserPlugins(options, options && options.__parserPlugins)),
    code,
    options && options.filename,
    syntaxHint(parserPlugins(options)),
    options && options.inputSourceMap,
  );
}

// Babel's `['flow', { all: true }]` parser plugin parses every file as Flow, pragma or not.
function flowAll(plugins) {
  if (!Array.isArray(plugins)) return false;
  return plugins.some((p) => Array.isArray(p) && p[0] === 'flow' && p[1] != null && p[1].all === true);
}

// The compiler options for a file parsed with these Babel parser plugins: `flow: 'all'` when
// the parser does, unless the caller set `flow` itself.
function withParserPlugins(options, plugins) {
  return options != null && options.flow == null && flowAll(plugins) ? { ...options, flow: 'all' } : options;
}

// Babel parser plugin names -> a file name for oxc's dialect, for files without a name.
function syntaxHint(plugins) {
  return plugins.includes('typescript') ? 'input.tsx' : undefined;
}

/**
 * createCompiler(options) -> { transform(code, filename, parserPlugins?, inputSourceMap?), errors }.
 * Reuse it across files. `parserPlugins` are Babel's: `['flow', { all: true }]` turns on
 * `flow: 'all'`, and `typescript` picks the dialect of an unnamed file.
 */
function createCompiler(options = {}) {
  return {
    // Babel's option diagnostics for the options as they are now.
    get errors() {
      return entryFor(options).errors;
    },
    transform: (code, filename, plugins, inputSourceMap) => {
      // Options are re-checked per file (cheap), so changes made in place take effect.
      const e = options.flow == null && flowAll(plugins) ? entryFor(withParserPlugins(options, plugins)) : entryFor(options);
      const hint = plugins ? syntaxHint(plugins.map((p) => (Array.isArray(p) ? p[0] : p))) : undefined;
      return run(e, code, filename, hint, inputSourceMap);
    },
  };
}

/** processStylexRules(rules, config) -> css. Same contract as the Babel plugin's. */
function processStylexRules(rules, config = {}) {
  const normalized = Array.from(rules, (rule) => {
    const priority = Number(rule[2]);
    const fields = rule[1];
    const constVal = fields?.constVal;
    return [
      rule[0],
      typeof constVal === 'number' && !Number.isFinite(constVal) ? { ...fields, constVal: String(constVal) } : fields,
      Number.isFinite(priority) ? priority : String(priority),
    ];
  });
  return addon().processStylexRules(JSON.stringify(normalized), config ?? null);
}

module.exports = { transformSync, createCompiler, processStylexRules };
// The Babel plugin (babel.js). The harness uses it in place of its generic adapter, so the suite
// runs the same adapter users get. Lazy: babel.js requires this file.
Object.defineProperty(module.exports, 'babelPlugin', {
  enumerable: true,
  get: () => require('./babel.js'),
});
