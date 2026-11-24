'use strict';
// Babel plugin adapter: a drop-in for @stylexjs/babel-plugin.
//
//   plugins: [['@stylexjs/babel-plugin', opts]]  ->  plugins: [['@stylexjs/oxc-plugin/babel', opts]]
//
// At Program enter it compiles the file with oxs, reparses the output with @babel/parser and
// swaps the program, so later plugins and Babel's printer see a normal AST.
//
// oxs compiles text. Usually that is the file's own text. When an earlier plugin may have edited
// the AST before StyleX's Program enter (a `pre` hook, an earlier pass, or an earlier plugin's
// Program/enter visitor in the same pass), the program is compared with a fresh parse of the
// text, and only when it differs is the current AST printed and compiled instead. The common
// case, StyleX first or alone, costs no extra parse or print.
const { createRequire } = require('node:module');
const nodePath = require('node:path');
const { createCompiler, processStylexRules } = require('./index.js');
const { PREFIX } = require('./validate.js');

let deps;
// Where the running @babel/core lives. Babel calls this plugin, so its core is already loaded:
// find it in the module cache first, then resolve from the usual places.
function babelCorePaths() {
  const out = [];
  const marker = `${nodePath.sep}@babel${nodePath.sep}core${nodePath.sep}`;
  for (const file of Object.keys(require.cache || {})) {
    const i = file.lastIndexOf(marker);
    if (i >= 0) out.push(file);
  }
  const from = [__dirname, process.cwd()];
  if (require.main && require.main.filename) from.push(nodePath.dirname(require.main.filename));
  for (const dir of from) {
    try {
      out.push(require.resolve('@babel/core', { paths: [dir] }));
    } catch {}
  }
  return out;
}

// @babel/parser and @babel/generator from the host's own @babel/core.
function babelDeps() {
  if (deps) return deps;
  for (const core of babelCorePaths()) {
    try {
      const req = createRequire(core);
      deps = { parser: req('@babel/parser'), generate: req('@babel/generator').default, types: req('@babel/types') };
      return deps;
    } catch {}
  }
  throw new Error('oxs/babel needs @babel/core, @babel/parser and @babel/generator');
}

// ---------------------------------------------------------------- did an earlier plugin edit?

function passesOf(opts) {
  return [opts.plugins || [], ...(opts.presets || []).map((p) => p.plugins || [])];
}

function hasProgramEnter(plugin) {
  const v = plugin.visitor || {};
  return v.enter != null || (v.Program != null && v.Program.enter != null && [].concat(v.Program.enter).length > 0);
}

// False only when nothing can have run on the AST before this plugin's Program enter.
function mayBeEdited(state) {
  const opts = state.file.opts;
  if (opts.wrapPluginVisitorMethod != null) return true;
  const passes = passesOf(opts);
  const index = passes.findIndex((plugins) => plugins.some((p) => p.key === state.key));
  if (index !== 0) return true;
  let before = true;
  for (const plugin of passes[0]) {
    if (plugin.key === state.key) {
      before = false;
      continue;
    }
    if (plugin.pre != null) return true;
    if (before && hasProgramEnter(plugin)) return true;
  }
  return false;
}

const IGNORED = new Set([
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
]);

function absent(v) {
  return v == null || v === false || (Array.isArray(v) && v.length === 0);
}

// Structural equality, ignoring positions, comments and parser extras.
function sameTree(a, b) {
  if (a === b) return true;
  if (absent(a) && absent(b)) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a == null || b == null) {
    return a === b || (Number.isNaN(a) && Number.isNaN(b));
  }
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (!sameTree(a[i], b[i])) return false;
    return true;
  }
  if (Array.isArray(b) || a.type !== b.type) return false;
  if (a instanceof RegExp || typeof a.type !== 'string') return JSON.stringify(a) === JSON.stringify(b);
  for (const k of Object.keys(a)) {
    if (IGNORED.has(k) || k[0] === '_') continue;
    if (!sameTree(a[k], b[k])) return false;
  }
  for (const k of Object.keys(b)) {
    if (IGNORED.has(k) || k[0] === '_' || k in a) continue;
    if (!absent(b[k])) return false;
  }
  return true;
}

function parseOptions(file, parserOpts, extra = []) {
  return {
    ...parserOpts,
    plugins: [...(parserOpts.plugins || []), ...extra],
    sourceType: parserOpts.sourceType ?? file.opts.sourceType ?? 'module',
    sourceFilename: file.opts.filename ?? undefined,
    tokens: false,
  };
}

function pluginName(p) {
  return Array.isArray(p) ? p[0] : p;
}

// Parses with the file's own parser options. A parserOverride (hermes-parser, for example)
// leaves parserOpts.plugins empty, so retry with the common syntax plugins.
function parse(code, file) {
  const { parser } = babelDeps();
  const parserOpts = file.opts.parserOpts || {};
  try {
    return parser.parse(code, parseOptions(file, parserOpts));
  } catch (error) {
    const names = (parserOpts.plugins || []).map(pluginName);
    const extra = ['jsx', 'flow'].filter((p) => !names.includes(p));
    if (!extra.length || names.includes('typescript')) throw error;
    return parser.parse(code, parseOptions(file, parserOpts, extra));
  }
}

// The text to compile: the file's own when the program still matches it.
function sourceText(path, state) {
  const file = state.file;
  if (!mayBeEdited(state)) return file.code;
  let original = null;
  try {
    original = parse(file.code, file).program;
  } catch {}
  if (original != null && sameTree(original, path.node)) return file.code;
  // Keep input lines, which debug data (`file:line`) reads.
  return babelDeps().generate(path.node, { comments: true, retainLines: true }, file.code).code;
}

// ---------------------------------------------------------------- folded JSX props

// Babel's plugin folds stylex.props() at Program exit, after any JSX transform in the same pass
// has run. The JSX transform then sees `{...stylex.props(a)}` (a call) and keeps the spread,
// which object spread lowering turns into `_objectSpread({}, ...)`. oxs folds at Program enter,
// so its plain object or attributes would be inlined instead. So the adapter asks oxs to print
// folded JSX props as `{...__oxs_folded_props__(expr)}` (option __oxsJsxFoldMarker) and finishes
// them at Program exit, the way Babel does there (visitors/stylex-props.js).
const FOLD = '__oxs_folded_props__';

function finishFoldedProps(path, t) {
  path.traverse({
    CallExpression(p) {
      const n = p.node;
      if (n.callee.type !== 'Identifier' || n.callee.name !== FOLD || n.arguments.length !== 1) return;
      const value = n.arguments[0];
      // Still a JSX spread (no JSX transform ran): Babel turns a simple object into attributes.
      if (
        p.parentPath.isJSXSpreadAttribute() &&
        t.isObjectExpression(value) &&
        value.properties.length > 0 &&
        value.properties.every((q) => t.isObjectProperty(q) && (t.isIdentifier(q.key) || t.isStringLiteral(q.key)) && !q.computed)
      ) {
        const attrs = value.properties.map((q) => {
          const name = t.isIdentifier(q.key) ? q.key.name : q.key.value;
          const v = t.isStringLiteral(q.value) ? q.value : t.stringLiteral(String(q.value));
          return t.jsxAttribute(t.jsxIdentifier(name), v);
        });
        p.parentPath.replaceWithMultiple(attrs);
        return;
      }
      p.replaceWith(value);
    },
  });
}

// ---------------------------------------------------------------- import elision

// Babel's plugin compiles later than oxs. A later plugin's Program enter in the same pass
// (TypeScript's import elision) still sees every reference the source had. After the swap, an
// import that only fed stylex.create (the stylex import, a .stylex tokens import) has no
// references, so TypeScript would drop it. So imports referenced in the source but not in the
// swapped program get a stand-in statement that references them, `(a, b);`, at the end of the
// body. It is removed when the first statement is visited, which is after every Program enter.
// It must be real syntax: other plugins re-crawl the scope at Program enter.
const HELD = '__oxs_held_imports__';

function referencedImports(path) {
  const names = [];
  for (const stmt of path.node.body) {
    if (stmt.type !== 'ImportDeclaration') continue;
    for (const spec of stmt.specifiers) {
      const binding = path.scope.getBinding(spec.local.name);
      if (binding != null && binding.referencePaths.length > 0) names.push(spec.local.name);
    }
  }
  return names;
}

function holdImports(path, names, t) {
  // Imports of the swapped program that the source referenced. Holding one that is still
  // referenced is harmless, so this needs no scope.
  const wanted = new Set(names);
  const held = [];
  for (const stmt of path.node.body) {
    if (stmt.type !== 'ImportDeclaration' || stmt.importKind === 'type') continue;
    for (const spec of stmt.specifiers) {
      if (spec.importKind !== 'type' && wanted.has(spec.local.name)) held.push(spec.local.name);
    }
  }
  if (held.length === 0) return null;
  const ids = held.map((name) => t.identifier(name));
  const stmt = t.expressionStatement(ids.length === 1 ? ids[0] : t.sequenceExpression(ids));
  path.node.body.push(stmt);
  return stmt;
}

function releaseImports(programPath, stmt) {
  const body = programPath.get('body');
  for (let i = body.length - 1; i >= 0; i--) {
    if (body[i].node === stmt) {
      body[i].remove();
      return;
    }
  }
}

// ---------------------------------------------------------------- generated nodes

// UTF-8 byte offsets (sorted) to string indexes (UTF-16 units).
function toIndexes(code, offsets) {
  if (Buffer.byteLength(code) === code.length) return offsets;
  const out = new Array(offsets.length);
  let k = 0;
  let byte = 0;
  for (let i = 0; i <= code.length && k < offsets.length; i++) {
    while (k < offsets.length && offsets[k] <= byte) out[k++] = i;
    const c = code.charCodeAt(i);
    if (c < 0x80) byte += 1;
    else if (c < 0x800) byte += 2;
    else if (c >= 0xd800 && c < 0xdc00) {
      byte += 4;
      i++;
    } else byte += 3;
  }
  while (k < offsets.length) out[k++] = code.length;
  return out;
}

// Babel replaces a StyleX call with nodes that have no location, and its printer puts a
// comment it inherited on lines of its own around such a node. The reparsed output gives
// every node a location, so a node that starts spliced-in text and carries a comment from
// before it loses its location here. Then Babel prints that comment the way it does for
// its own output. Other nodes keep theirs (formatting and maps read them).
function unlocateCommentedRoots(ast, code, ranges) {
  const idx = toIndexes(code, ranges);
  const inRange = (pos) => {
    let lo = 0;
    let hi = idx.length / 2;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (idx[2 * mid + 1] <= pos) lo = mid + 1;
      else hi = mid;
    }
    return lo < idx.length / 2 && idx[2 * lo] <= pos ? idx[2 * lo] : -1;
  };
  const { types } = babelDeps();
  types.traverseFast(ast.program, (node) => {
    const lead = node.leadingComments;
    if (lead == null || lead.length === 0 || node.start == null) return;
    const start = inRange(node.start);
    if (start >= 0 && lead.every((c) => c.end <= start)) node.loc = null;
  });
}

// ---------------------------------------------------------------- the plugin

function toBabelError(error, file) {
  if (error == null || typeof error !== 'object') return error;
  const message = String(error.message ?? error);
  const loc = error.loc;
  let wrapped;
  if (loc && typeof loc.line === 'number') {
    const start = { line: loc.line, column: loc.column ?? 0 };
    wrapped = file.buildCodeFrameError({ loc: { start, end: error.endLoc ?? start } }, message, SyntaxError);
  } else {
    wrapped = new SyntaxError(message);
  }
  if (error.code != null) wrapped.code = error.code;
  return wrapped;
}

// An error in a re-print (an earlier plugin edited the AST) has positions in the printed text.
// Babel's plugin builds its frame from the AST node, so find the node the error spans: parse
// the printed text, which has the program's shape, locate the span there, and follow the same
// path in the current program. buildCodeFrameError on that node then gives Babel's frame at the
// node's original loc, or its "internal node" note when an earlier plugin created the node.
const SKIP_KEYS = new Set([...IGNORED, 'type']);

function offsetOf(text, loc) {
  let line = 1;
  let i = 0;
  while (line < loc.line) {
    const nl = text.indexOf('\n', i);
    if (nl < 0) return text.length;
    i = nl + 1;
    line++;
  }
  return i + (loc.column ?? 0);
}

// Keys from `root` to the outermost node spanning exactly [start, end], else to the smallest
// node containing it.
function pathToSpan(root, start, end) {
  let best = null;
  let bestSize = Infinity;
  const walk = (node, keys) => {
    if (node.start === start && node.end === end) return keys;
    if (node.start <= start && node.end >= end && node.end - node.start < bestSize) {
      best = keys;
      bestSize = node.end - node.start;
    }
    for (const k of Object.keys(node)) {
      if (SKIP_KEYS.has(k) || k[0] === '_') continue;
      const v = node[k];
      if (Array.isArray(v)) {
        for (let i = 0; i < v.length; i++) {
          const c = v[i];
          if (c != null && typeof c.type === 'string' && c.start <= start && c.end >= end) {
            const found = walk(c, [...keys, k, i]);
            if (found) return found;
          }
        }
      } else if (v != null && typeof v === 'object' && typeof v.type === 'string' && v.start <= start && v.end >= end) {
        const found = walk(v, [...keys, k]);
        if (found) return found;
      }
    }
    return null;
  };
  return walk(root, []) ?? best;
}

function errorOnReprint(error, file, program, printed) {
  if (error == null || typeof error !== 'object' || !error.loc || typeof error.loc.line !== 'number') {
    return toBabelError(error, file);
  }
  let node = null;
  try {
    const start = offsetOf(printed, error.loc);
    const end = error.endLoc ? offsetOf(printed, error.endLoc) : start;
    const keys = pathToSpan(parse(printed, file).program, start, end) ?? [];
    node = program;
    for (const k of keys) {
      const next = node[k];
      if (next == null || typeof next !== 'object') break;
      node = next;
    }
  } catch {
    return toBabelError(error, file);
  }
  const wrapped = file.buildCodeFrameError(node, String(error.message ?? error), SyntaxError);
  if (error.code != null) wrapped.code = error.code;
  return wrapped;
}

function logError(line) {
  // Babel logs `console.error('[@stylexjs/babel-plugin]', message)`.
  if (line.startsWith(`${PREFIX} `)) console.error(PREFIX, line.slice(PREFIX.length + 1));
  else console.error(line);
}

function stylexOxsPlugin(api, rawOptions) {
  const types = api.types;
  return {
    name: 'stylex',
    visitor: {
      Program: {
        enter(path, state) {
          const file = state.file;
          const code = sourceText(path, state);
          // With source maps on, oxs maps its output to the file (through Babel's input map,
          // if any) and becomes the file's input map. The nodes parsed below carry positions
          // in oxs's output. The generator traces every mark through its inputSourceMap, so
          // the printed map points back at the source. Babel core sets generatorOpts from
          // file.inputMap. Callers that run the generator themselves (Next.js's loader) read
          // file.opts.generatorOpts, so set it there too.
          const wantMap = Boolean(file.opts.sourceMaps) && code === file.code;
          // Babel reads the plugin options for every file, so changes made in place count.
          // createCompiler finds the cached compiler for options with these contents.
          const options = { ...(state.opts ?? rawOptions ?? {}), __oxsJsxFoldMarker: true };
          const c = createCompiler(wantMap ? { ...options, sourceMaps: true } : options);
          let result;
          try {
            const input = wantMap && file.inputMap != null ? file.inputMap.toObject() : undefined;
            result = c.transform(code, file.opts.filename, (file.opts.parserOpts || {}).plugins, input);
          } catch (error) {
            throw code === file.code ? toBabelError(error, file) : errorOnReprint(error, file, path.node, code);
          }
          for (const line of result.errors) logError(line);
          for (const w of result.warnings) console.warn(w);
          file.metadata.stylex = result.metadata.stylex;
          if (result.code === code) return;
          if (wantMap && result.map != null) {
            const map = result.map;
            file.inputMap = { toObject: () => map };
            if (file.opts.generatorOpts != null) file.opts.generatorOpts.inputSourceMap = map;
          }
          const wasReferenced = referencedImports(path);
          const parsed = parse(result.code, file);
          if (result.generated != null) unlocateCommentedRoots(parsed, result.code, result.generated);
          const program = parsed.program;
          path.node.body = program.body;
          path.node.directives = program.directives;
          path.node.interpreter = program.interpreter ?? path.node.interpreter;
          path.node.innerComments = program.innerComments;
          path.node.leadingComments = program.leadingComments;
          path.node.trailingComments = program.trailingComments;
          file.ast.comments = parsed.comments;
          if (result.code.includes(FOLD)) state.set(FOLD, true);
          const held = wasReferenced.length > 0 ? holdImports(path, wasReferenced, types) : null;
          if (held != null) state.set(HELD, held);
          path.scope.crawl();
        },
        exit(path, state) {
          if (state.get(HELD)) {
            releaseImports(path, state.get(HELD));
            state.set(HELD, null);
          }
          if (state.get(FOLD)) finishFoldedProps(path, types);
        },
      },
      'Statement|Directive'(path, state) {
        const held = state.get(HELD);
        if (held) {
          state.set(HELD, null);
          releaseImports(path.findParent((p) => p.isProgram()), held);
        }
      },
    },
  };
}

stylexOxsPlugin.processStylexRules = processStylexRules;
stylexOxsPlugin.withOptions = (options) => [stylexOxsPlugin, options];
stylexOxsPlugin.default = stylexOxsPlugin;
module.exports = stylexOxsPlugin;
