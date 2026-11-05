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

function rethrowDiagnostic(cause, source, filename) {
  const prefix = 'STYLEX_DIAGNOSTIC:';
  if (!cause.message?.startsWith(prefix)) throw cause;
  const diagnostic = JSON.parse(cause.message.slice(prefix.length));
  const parse = diagnostic.kind === 'parse';
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

const MAX_NESTED_TRANSFORMS = 64;
let depth = 0;

function transformSync(source, options = {}) {
  if (typeof source !== 'string') {
    throw new TypeError('StyleX transformSync expects JavaScript source text');
  }
  const { filename = 'unknown.js', ...pluginOptions } = options;
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
    if (host.state.hasThrown) throw host.state.thrown;
    rethrowDiagnostic(cause, source, filename);
  } finally {
    depth--;
  }
  if (host.state.hasThrown) throw host.state.thrown;
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
