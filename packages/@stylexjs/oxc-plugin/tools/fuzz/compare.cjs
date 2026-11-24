'use strict';

// Compare one rendered program between the real Babel plugin and a candidate.
// Both run through @babel/core with the same parser options. The candidate
// runs behind the harness adapter, so both outputs are printed by Babel.

const fs = require('node:fs');
const path = require('node:path');
const { createAdapter } = require('./adapter.cjs');
const { render, features } = require('./gen.cjs');

// Differences that cannot change behavior: quoting, comments, generated names.
const HARMLESS = new Set(['format-only', 'uid-rename', 'metadata-order']);

// Babel-style generated names (_temp, _styles2, ...) renamed by first use.
function renameUids(code) {
  const map = new Map();
  return code.replace(/\b_[A-Za-z$][\w$]*\b/g, (m) => {
    if (!map.has(m)) map.set(m, `_u${map.size}`);
    return map.get(m);
  });
}

const BABEL_CRASH_TYPES = new Set(['TypeError', 'RangeError', 'ReferenceError', 'InternalError']);

function setup({ repo, candidate, root }) {
  const pluginRoot = path.join(repo, 'packages/@stylexjs/babel-plugin');
  const req = (m) => require(require.resolve(m, { paths: [pluginRoot] }));
  const babel = req('@babel/core');
  const parser = req('@babel/parser');
  const generate = req('@babel/generator').default;
  const t = req('@babel/types');
  const traverse = req('@babel/traverse').default;

  // Free identifiers of a program (referenced, never declared). If oxs output
  // has one Babel's output lacks, it would throw a ReferenceError at runtime.
  function globalsOf(code, input, what = 'globals') {
    try {
      const ast = parser.parse(code, { sourceType: 'module', plugins: input.parserPlugins ?? [] });
      let g = null;
      traverse(ast, { Program(p) { g = Object.keys(what === 'globals' ? p.scope.globals : p.scope.bindings); p.stop(); } });
      return new Set(g || []);
    } catch {
      return null;
    }
  }
  const realMod = require(path.join(pluginRoot, 'lib/index.js'));
  const realPlugin = realMod.default ?? realMod;

  let cand = null;
  let adapter = null;
  if (candidate) {
    const candMod = require(path.resolve(candidate));
    cand = typeof candMod.create === 'function' ? candMod.create({ babelPlugin: realPlugin, babel, parser, repo }) : candMod;
    adapter = createAdapter(cand, { parser, fallbackProcessStylexRules: realPlugin.processStylexRules });
  }

  function run(plugin, input) {
    const warnings = [];
    const ow = console.warn;
    const oe = console.error;
    const ol = console.log;
    console.warn = (...a) => warnings.push(a.map(String).join(' '));
    console.error = (...a) => warnings.push(a.map(String).join(' '));
    console.log = (...a) => warnings.push(a.map(String).join(' '));
    const t0 = process.hrtime.bigint();
    try {
      const r = babel.transformSync(input.code, {
        babelrc: false,
        configFile: false,
        filename: input.filename ?? undefined,
        sourceType: input.sourceType ?? 'module',
        parserOpts: { plugins: input.parserPlugins ?? [] },
        plugins: [[plugin, input.options ?? {}]],
      });
      const ms = Number(process.hrtime.bigint() - t0) / 1e6;
      const metadata = r.metadata?.stylex ?? [];
      // other files of the "app" (the imported .stylex file), compiled by the same side
      let allRules = metadata;
      for (const extra of input.extraFiles || []) {
        const e = babel.transformSync(extra.code, {
          babelrc: false, configFile: false, filename: extra.filename, sourceType: 'module',
          parserOpts: { plugins: input.parserPlugins ?? [] }, plugins: [[plugin, input.options ?? {}]],
        });
        allRules = [...(e.metadata?.stylex ?? []), ...allRules];
      }
      let css = null;
      try {
        css = plugin.processStylexRules(allRules, input.processConfig ?? { useLayers: false });
      } catch (e) {
        css = `ERROR: ${String(e && e.message).split('\n')[0]}`;
      }
      return { ok: true, code: r.code, metadata, allRules, css, warnings, ms };
    } catch (e) {
      const raw = String(e && e.message);
      const msg = raw.replace(/^(unknown file|\/[^:]*|[A-Za-z]:[^:]*): /, '').split('\n')[0];
      return {
        ok: false,
        error: msg,
        // the code frame's marked line ("> 12 | ..."), when Babel printed one
        errorLine: (raw.match(/^> +(\d+) \|/m) || [])[1] ?? (raw.match(/\((\d+):\d+\)/) || [])[1] ?? null,
        errorType: e && e.constructor ? e.constructor.name : 'unknown',
        errorCode: e && e.code,
        stack: e && e.stack ? String(e.stack).split('\n').slice(0, 6).join('\n') : null,
        warnings,
      };
    } finally {
      console.warn = ow;
      console.error = oe;
      console.log = ol;
    }
  }

  // Formatting-insensitive form. Reparse, drop comments, unquote keys that are
  // valid identifiers (quoted vs unquoted keys are known harmless).
  function canon(code, input) {
    try {
      const ast = parser.parse(code, {
        sourceType: 'module',
        plugins: input.parserPlugins ?? [],
      });
      const stack = [ast.program];
      while (stack.length) {
        const n = stack.pop();
        if (!n || typeof n !== 'object') continue;
        if (Array.isArray(n)) {
          for (const x of n) stack.push(x);
          continue;
        }
        if ((n.type === 'ObjectProperty' || n.type === 'ObjectMethod') && !n.computed && n.key) {
          if (n.key.type === 'StringLiteral' && t.isValidIdentifier(n.key.value, false)) n.key = t.identifier(n.key.value);
          else if (n.key.type === 'NumericLiteral') n.key = t.stringLiteral(String(n.key.value));
        }
        if (n.type === 'StringLiteral' || n.type === 'NumericLiteral') {
          delete n.extra;
        }
        for (const k of Object.keys(n)) {
          if (k === 'loc' || k === 'start' || k === 'end' || k === 'leadingComments' || k === 'trailingComments' || k === 'innerComments') continue;
          const v = n[k];
          if (v && typeof v === 'object') stack.push(v);
        }
      }
      return generate(ast, { compact: true, comments: false, jsescOption: { quotes: 'double', minimal: true } }).code;
    } catch {
      return null;
    }
  }

  function stable(v) {
    if (Array.isArray(v)) return v.map(stable);
    if (v && typeof v === 'object') return Object.fromEntries(Object.keys(v).sort().map((k) => [k, stable(v[k])]));
    return v;
  }

  // Babel's vars output, used to build createTheme inputs.
  const varsCache = new Map();
  function compileVars(varsSpec) {
    const k = JSON.stringify(varsSpec.items);
    if (varsCache.has(k)) return varsCache.get(k);
    const input = render(varsSpec, { root });
    const r = run(realPlugin, { ...input, options: { unstable_moduleResolution: { type: 'commonJS', rootDir: root } } });
    const out = r.ok ? r.code : null;
    varsCache.set(k, out);
    return out;
  }

  function isBabelCrash(r) {
    if (r.ok) return false;
    if (BABEL_CRASH_TYPES.has(r.errorType)) return true;
    return /Maximum call stack|Cannot read properties|Cannot read property|is not a function|is not iterable|undefined is not/.test(r.error);
  }

  function classify(a, b, input) {
    const kinds = [];
    if (a.ok && b.ok) {
      if (a.code !== b.code) {
        const ca = canon(a.code, input);
        const cb = canon(b.code, input);
        if (ca != null && ca === cb) kinds.push('format-only');
        else if (ca != null && cb != null && renameUids(ca) === renameUids(cb)) kinds.push('uid-rename');
        else kinds.push('code-diff');
      }
      const ma = JSON.stringify(stable(a.metadata));
      const mb = JSON.stringify(stable(b.metadata));
      if (ma !== mb) {
        const sa = a.metadata.map((r) => JSON.stringify(stable(r))).sort().join('\n');
        const sb = b.metadata.map((r) => JSON.stringify(stable(r))).sort().join('\n');
        kinds.push(sa === sb ? 'metadata-order' : 'metadata-diff');
      }
      if (a.css !== b.css) kinds.push('css-diff');
      if (kinds.includes('code-diff')) {
        const ga = globalsOf(a.code, input);
        const gb = globalsOf(b.code, input);
        // only names the input itself declares at top level (not undefined, window, ...)
        const declared = globalsOf(input.code, input, 'bindings');
        const extra = ga && gb && declared ? [...gb].filter((n) => !ga.has(n) && declared.has(n)) : [];
        if (extra.length) {
          kinds.push('undefined-ref');
          kinds.undefinedRefs = extra;
        }
      }
      const wa = (a.warnings || []).map((w) => w.replace(/\/[^\s:]+/g, 'PATH')).sort().join('\n');
      const wb = (b.warnings || []).map((w) => w.replace(/\/[^\s:]+/g, 'PATH')).sort().join('\n');
      if (wa !== wb) kinds.push('warning-diff');
      if (!kinds.length || kinds.every((k) => HARMLESS.has(k))) kinds.unshift('identical');
    } else if (!a.ok && !b.ok) {
      if (a.error !== b.error) kinds.push('both-throw-diff');
      else if (a.errorLine != null && a.errorLine !== b.errorLine) kinds.push('error-loc-diff');
      else kinds.push('both-throw-same');
    } else {
      kinds.push(a.ok ? 'candidate-only-error' : 'babel-only-error');
    }
    return kinds;
  }

  const PRIMARY = ['candidate-crash', 'babel-crash', 'invalid-input', 'undefined-ref', 'candidate-only-error', 'babel-only-error',
    'both-throw-diff', 'metadata-diff', 'css-diff', 'code-diff', 'error-loc-diff', 'warning-diff', 'both-throw-same', 'identical'];

  function evalSpec(spec, { full = false } = {}) {
    const input = render(spec, { root, compileVars });
    if (input == null) return { primary: 'invalid-input', kinds: ['invalid-input'], reason: 'vars compile failed', tags: features(spec) };
    const a = run(realPlugin, input);
    const tags = features(spec);
    if (!a.ok && (a.errorCode === 'BABEL_PARSE_ERROR' || /^Unexpected token|^Missing semicolon/.test(a.error))) {
      return { primary: 'invalid-input', kinds: ['invalid-input'], babelError: a.error, tags, input: full ? input : undefined };
    }
    if (isBabelCrash(a)) {
      return { primary: 'babel-crash', kinds: ['babel-crash'], babelError: a.error, babelStack: a.stack, tags, input: full ? input : undefined };
    }
    if (!adapter) return { primary: a.ok ? 'identical' : 'both-throw-same', kinds: ['babel-only-run'], babel: summarize(a, full), tags };
    const b = run(adapter, input);
    const kinds = classify(a, b, input);
    // processStylexRules on Babel's own metadata: isolates the CSS processor.
    if (a.ok && cand && typeof cand.processStylexRules === 'function') {
      let css2;
      try {
        css2 = cand.processStylexRules(a.allRules ?? a.metadata, input.processConfig ?? { useLayers: false });
      } catch (e) {
        css2 = `ERROR: ${String(e && e.message).split('\n')[0]}`;
      }
      if (css2 !== a.css) kinds.push('cssproc-diff');
      if (full) b.cssOnBabelMetadata = css2;
    }
    const primary = PRIMARY.find((p) => kinds.includes(p)) ?? (kinds.includes('cssproc-diff') ? 'css-diff' : 'identical');
    return {
      primary,
      kinds,
      tags,
      babelError: a.ok ? undefined : a.error,
      candError: b.ok ? undefined : b.error,
      diffHead: a.ok && b.ok && primary !== 'identical' ? diffHead(a, b, primary, input) : undefined,
      undefinedRefs: kinds.undefinedRefs,
      errorLines: !a.ok && !b.ok ? { babel: a.errorLine, candidate: b.errorLine } : undefined,
      warnDiff: a.ok && b.ok ? [...(a.warnings || []).filter((w) => !(b.warnings || []).includes(w)).map((w) => `B:${w}`), ...(b.warnings || []).filter((w) => !(a.warnings || []).includes(w)).map((w) => `C:${w}`)].slice(0, 4) : undefined,
      babel: full ? summarize(a, true) : undefined,
      candidate: full ? summarize(b, true) : undefined,
      input: full ? input : undefined,
      ms: { babel: a.ms, candidate: b.ms },
    };
  }

  function summarize(r, full) {
    if (!full) return { ok: r.ok, error: r.error };
    return r;
  }

  // First differing line, with hashes masked. Feeds the failure signature.
  function diffHead(a, b, primary, input) {
    let la;
    let lb;
    if (primary === 'metadata-diff') {
      la = a.metadata.map((r) => JSON.stringify(stable(r))).sort();
      lb = b.metadata.map((r) => JSON.stringify(stable(r))).sort();
    } else if (primary === 'css-diff') {
      la = String(a.css).split('\n');
      lb = String(b.css).split('\n');
    } else {
      la = String(canon(a.code, input) ?? a.code).split(/(?<=[;{}])/);
      lb = String(canon(b.code, input) ?? b.code).split(/(?<=[;{}])/);
    }
    const sb = new Set(lb);
    const sa = new Set(la);
    const onlyA = la.filter((l) => !sb.has(l));
    const onlyB = lb.filter((l) => !sa.has(l));
    return { babel: onlyA.slice(0, 6), candidate: onlyB.slice(0, 6) };
  }

  return { evalSpec, run, canon, realPlugin, adapter, candidate: cand };
}

module.exports = { setup };
