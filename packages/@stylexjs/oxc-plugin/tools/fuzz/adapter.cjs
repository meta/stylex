'use strict';

// Generic Babel adapter for a candidate StyleX compiler.
//
// The candidate is any object with
//   transformSync(code, options) -> { code, map?, metadata: { stylex: rules },
//                                     warnings?: string[], errors?: string[] }
//   processStylexRules?(rules, config) -> string
//
// The adapter is a Babel plugin. At Program enter it hands the original file
// text, the plugin options and the filename to the candidate, parses the
// candidate's output with Babel (same parser options as the input), swaps the
// program body, and sets `file.metadata.stylex`. Candidate errors are rethrown
// as Babel errors (with a code frame when the candidate gives a location).

const HARNESS_KEYS = ['__sourceType', '__parserPlugins', '__harness'];

function pluginName(p) {
  return Array.isArray(p) ? p[0] : p;
}

function createAdapter(candidate, { parser, fallbackProcessStylexRules } = {}) {
  if (candidate == null || typeof candidate.transformSync !== 'function') {
    throw new Error('candidate must export transformSync(code, options)');
  }
  if (parser == null) throw new Error('createAdapter needs @babel/parser');

  const stats = { calls: 0, replaced: 0, unchanged: 0, errors: 0 };

  function toBabelError(error, path, file) {
    if (error == null || typeof error !== 'object') return error;
    const message = String(error.message ?? error);
    let wrapped;
    const loc = error.loc;
    if (loc && typeof loc.line === 'number') {
      const start = { line: loc.line, column: loc.column ?? 0 };
      const end = error.endLoc ?? start;
      wrapped = file.buildCodeFrameError({ loc: { start, end } }, message, SyntaxError);
    } else {
      wrapped = new SyntaxError(message);
    }
    if (error.code != null) wrapped.code = error.code;
    wrapped.candidateError = error;
    return wrapped;
  }

  function adapterPlugin(_api, options) {
    return {
      name: 'stylex-candidate-adapter',
      visitor: {
        Program: {
          enter(path, state) {
            const file = state.file;
            stats.calls++;
            const parserOpts = file.opts.parserOpts || {};
            const callOptions = {
              ...(options || {}),
              __sourceType: file.opts.sourceType ?? parserOpts.sourceType,
              __parserPlugins: parserOpts.plugins || [],
            };
            if (file.opts.filename != null) callOptions.filename = file.opts.filename;

            let result;
            try {
              result = candidate.transformSync(file.code, callOptions);
            } catch (error) {
              stats.errors++;
              throw toBabelError(error, path, file);
            }
            if (result == null) result = { code: file.code };
            for (const w of result.warnings || []) console.warn(w);
            for (const e of result.errors || []) console.error(e);
            file.metadata.stylex = result.metadata?.stylex ?? [];

            if (result.code == null || result.code === file.code) {
              stats.unchanged++;
              return;
            }
            const reparse = (extra) =>
              parser.parse(result.code, {
                ...parserOpts,
                plugins: [...(parserOpts.plugins || []), ...extra],
                sourceType: parserOpts.sourceType ?? file.opts.sourceType ?? 'module',
                sourceFilename: file.opts.filename ?? undefined,
                tokens: false,
              });
            let parsed;
            try {
              parsed = reparse([]);
            } catch (error) {
              // A parserOverride (e.g. hermes-parser) leaves parserOpts.plugins
              // empty; retry with the common syntax plugins before giving up.
              const names = (parserOpts.plugins || []).map(pluginName);
              const extra = ['jsx', 'flow'].filter((p) => !names.includes(p));
              if (!extra.length || names.includes('typescript')) throw error;
              parsed = reparse(extra);
            }
            const program = parsed.program;
            path.node.body = program.body;
            path.node.directives = program.directives;
            path.node.interpreter = program.interpreter ?? path.node.interpreter;
            path.node.innerComments = program.innerComments;
            path.node.leadingComments = program.leadingComments;
            path.node.trailingComments = program.trailingComments;
            file.ast.comments = parsed.comments;
            path.scope.crawl();
            stats.replaced++;
          },
        },
      },
    };
  }

  adapterPlugin.processStylexRules =
    typeof candidate.processStylexRules === 'function'
      ? candidate.processStylexRules.bind(candidate)
      : fallbackProcessStylexRules;
  adapterPlugin.default = adapterPlugin;
  adapterPlugin.__stats = stats;
  adapterPlugin.__candidate = candidate;
  return adapterPlugin;
}

function stripHarnessKeys(options) {
  const out = { ...options };
  for (const k of HARNESS_KEYS) delete out[k];
  return out;
}

module.exports = { createAdapter, stripHarnessKeys, HARNESS_KEYS };
