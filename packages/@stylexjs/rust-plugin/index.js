/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const path = require('node:path');
const { codeFrameColumns } = require('@babel/code-frame');

let binding;
function getBinding() {
  if (binding == null) {
    const filename = `stylex.${process.platform}-${process.arch}.node`;
    try {
      binding = require(path.join(__dirname, 'native', filename));
    } catch (cause) {
      throw new Error(
        `The StyleX native compiler for ${process.platform}-${process.arch} is not built. Run npm run build:native --workspace @stylexjs/rust-plugin.`,
        { cause },
      );
    }
  }
  return binding;
}

function sourcePosition(source, offset) {
  const prefix = Buffer.from(source).subarray(0, offset).toString('utf8');
  const lines = prefix.split(/\r\n|[\n\r\u2028\u2029]/u);
  return { line: lines.length, column: lines[lines.length - 1].length + 1 };
}

// `file` is the absolute filename Babel would report, or undefined for a file
// without one, which Babel's parser calls 'unknown' and its transform
// 'unknown file'.
function rethrowDiagnostic(cause, source, file) {
  const prefix = 'STYLEX_DIAGNOSTIC:';
  if (!cause.message?.startsWith(prefix)) throw cause;
  const diagnostic = JSON.parse(cause.message.slice(prefix.length));
  const parse = diagnostic.kind === 'parse';
  const filename = file ?? (parse ? 'unknown' : 'unknown file');
  let message = diagnostic.message;
  let start;
  if (diagnostic.range) {
    start = sourcePosition(source, diagnostic.range.start);
    const end = sourcePosition(source, diagnostic.range.end);
    // Babel's parser errors end with the position, e.g. "(1:6)".
    if (parse) message += ` (${start.line}:${start.column - 1})`;
    const location = { start };
    if (end.line === start.line) location.end = end;
    message += `\n${codeFrameColumns(source, location, { highlightCode: false })}`;
  }
  const Constructor =
    parse || diagnostic.name === 'SyntaxError' ? SyntaxError : Error;
  const error = new Constructor(`${filename}: ${message}`, { cause });
  // The fields tooling (bundler overlays, Jest) reads from Babel's errors.
  error.code = parse ? 'BABEL_PARSE_ERROR' : 'BABEL_TRANSFORM_ERROR';
  if (parse && start != null) {
    error.loc = {
      line: start.line,
      column: start.column - 1,
      index: Buffer.from(source)
        .subarray(0, diagnostic.range.start)
        .toString('utf8').length,
    };
    error.pos = error.loc.index;
  }
  error.stylexDiagnostic = diagnostic;
  throw error;
}

// The host outlives the transform when compiler functions were retained, so
// an exception is reported once and does not linger for their later calls.
function takeThrown(host) {
  const { thrown } = host.state;
  host.state.hasThrown = false;
  host.state.thrown = undefined;
  return thrown;
}

// Babel's `run` wraps an exception escaping a plugin in place: the same value,
// its message prefixed with the file and a code added when it has none. The
// statements are Babel's own, in strict mode, so a thrown string, null or
// frozen error fails with exactly the TypeError Babel raises for it.
function babelTransformError(e, label) {
  e.message = `${label}: ${e.message}`;
  if (!e.code) {
    e.code = 'BABEL_TRANSFORM_ERROR';
  }
  return e;
}

const MAX_NESTED_TRANSFORMS = 64;
let depth = 0;

function transformSync(source, options = {}) {
  if (typeof source !== 'string') {
    throw new TypeError('StyleX transformSync expects JavaScript source text');
  }
  const {
    filename: givenFilename,
    // Internal: set by the Babel facade (babel.js) on every compile, whose
    // exceptions Babel itself wraps once. Never forwarded to the compiler.
    __errorsHandledByBabel: handledByBabel,
    ...rest
  } = options;
  const unnamed = givenFilename === undefined;
  const filename = unnamed ? 'unknown.js' : givenFilename;
  // A file without a name compiles as Babel's plugin compiles one: with the
  // empty name the facade passes for it (no import resolution, no file label).
  const pluginOptions = unnamed
    ? { ...rest, __sourceFileName: '' }
    : { ...rest };
  // Babel resolves `filename` against the working directory when it loads the
  // options.
  const file =
    typeof givenFilename === 'string' ? path.resolve(givenFilename) : undefined;
  const label = file ?? 'unknown file';
  // A user exception escapes this transform: rethrown as Babel would.
  const userThrow = (host) => {
    const thrown = takeThrown(host);
    if (handledByBabel != null && typeof handledByBabel === 'object') {
      handledByBabel.userThrew = true;
      return thrown;
    }
    return babelTransformError(thrown, label);
  };
  // Babel's own map objects carry `file`/`sourceRoot` as `undefined`; an absent
  // field means the same, so only those are left out. Every other value is
  // passed through unchanged for the compiler to validate.
  const { inputSourceMap } = pluginOptions;
  if (
    inputSourceMap != null &&
    typeof inputSourceMap === 'object' &&
    !Array.isArray(inputSourceMap)
  ) {
    pluginOptions.inputSourceMap = Object.fromEntries(
      Object.entries(inputSourceMap).filter(([, value]) => value !== undefined),
    );
  }
  const host = require('./host').createHost(
    pluginOptions,
    filename,
    getBinding(),
  );
  // A transform started from a callback of another one nests native frames on
  // the JS stack; past this depth V8 could not report the overflow safely.
  if (depth >= MAX_NESTED_TRANSFORMS) {
    throw new RangeError('Maximum call stack size exceeded');
  }
  let output;
  depth++;
  try {
    output = getBinding().transformSync(
      source,
      filename,
      host.options,
      host.invoke,
    );
  } catch (cause) {
    // A user callback threw: propagate that exception, as Babel does, rather
    // than the compiler error it caused.
    if (host.state.hasThrown) throw userThrow(host);
    if (require('./host').stackOverflow(cause)) {
      throw new RangeError('Maximum call stack size exceeded');
    }
    rethrowDiagnostic(cause, source, file);
  } finally {
    depth--;
  }
  if (host.state.hasThrown) throw userThrow(host);
  const { map, metadata, errors, warnings } = JSON.parse(output.json);
  return { code: output.code, map, metadata, errors, warnings };
}

async function transform(source, options = {}) {
  return transformSync(source, options);
}

function processStylexRules(rules, options = {}) {
  // Mirror the JS compiler's tolerance: it reads only the first three entries
  // of a rule and coerces the priority with JavaScript number semantics.
  const normalized = Array.from(rules, (rule) => {
    const priority = Number(rule[2]);
    return [
      rule[0],
      rule[1],
      Number.isFinite(priority) ? priority : String(priority),
    ];
  });
  return getBinding().processStylexRules(JSON.stringify(normalized), options);
}

module.exports = { transformSync, transform, processStylexRules };
