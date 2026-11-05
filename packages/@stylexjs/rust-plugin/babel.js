/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

const compiler = require('./index');
const { createRequire } = require('node:module');
const babelCore = require('@babel/core');
const babelRequire = createRequire(require.resolve('@babel/core'));
const generate = babelRequire('@babel/generator').default;
const { TraceMap, originalPositionFor } = babelRequire(
  '@jridgewell/trace-mapping',
);
const { codeFrameColumns } = require('@babel/code-frame');
const DIAGNOSTIC_PREFIX = 'STYLEX_DIAGNOSTIC:';

function parseOptions(state, filename) {
  const parserOpts = { ...state.file.opts.parserOpts };
  const plugins = [...(parserOpts.plugins || [])];
  const syntax = { jsx: false, flow: false, typescript: false };
  const types = babelCore.types;
  types.traverseFast(state.file.ast.program, (node) => {
    if (node.type.startsWith('JSX')) syntax.jsx = true;
    if (types.isFlow(node)) syntax.flow = true;
    if (types.isTypeScript(node)) syntax.typescript = true;
  });
  // Parser overrides (notably Hermes) supply an AST without adding their syntax
  // to parserOpts.plugins. Recover grammar only; never rerun user visitors.
  if (
    (state.file.opts.plugins || []).some((plugin) =>
      /hermes/.test(plugin.key || ''),
    )
  ) {
    syntax.jsx = true;
    syntax.flow = !syntax.typescript;
  }
  const has = (name) =>
    plugins.some(
      (plugin) => (Array.isArray(plugin) ? plugin[0] : plugin) === name,
    );
  if (syntax.typescript && !has('typescript')) plugins.push('typescript');
  if (syntax.flow && !syntax.typescript && !has('flow'))
    plugins.push(['flow', { all: true }]);
  if (syntax.jsx && !has('jsx')) plugins.push('jsx');
  parserOpts.plugins = plugins;
  return {
    filename,
    babelrc: false,
    configFile: false,
    sourceType: state.file.opts.sourceType,
    parserOpts,
  };
}

function sourcePosition(source, offset) {
  // SWC reports UTF-8 offsets. Babel's columns count UTF-16 code units.
  const prefix = Buffer.from(source, 'utf8')
    .subarray(0, offset)
    .toString('utf8');
  const lines = prefix.split(/\r\n|\n|\r|\u2028|\u2029/);
  return { line: lines.length, column: lines[lines.length - 1].length + 1 };
}

function diagnosticFromError(error) {
  if (error != null && error.stylexDiagnostic != null)
    return error.stylexDiagnostic;
  if (error != null && error.diagnostic != null) return error.diagnostic;
  const message = error != null ? error.message : null;
  if (typeof message !== 'string' || !message.startsWith(DIAGNOSTIC_PREFIX))
    return null;
  try {
    const diagnostic = JSON.parse(message.slice(DIAGNOSTIC_PREFIX.length));
    if (
      typeof diagnostic.message === 'string' &&
      ['Error', 'SyntaxError'].includes(diagnostic.name)
    )
      return diagnostic;
  } catch (_) {
    // A malformed native envelope is a native error, not a StyleX diagnostic.
  }
  return null;
}

function diagnosticError(diagnostic, source, state) {
  let message = diagnostic.message;
  const range = diagnostic.range;
  if (
    range != null &&
    Number.isInteger(range.start) &&
    Number.isInteger(range.end)
  ) {
    const start = sourcePosition(source, range.start);
    const end = sourcePosition(source, range.end);
    // Exactly the standard Babel File.buildCodeFrameError behavior: a multiline
    // node highlights its start only, not every line through its end.
    message +=
      '\n' +
      codeFrameColumns(
        source,
        {
          start,
          end: start.line === end.line ? end : undefined,
        },
        { highlightCode: state.file.opts.highlightCode !== false },
      );
  }
  const Constructor = diagnostic.name === 'SyntaxError' ? SyntaxError : Error;
  const error = new Constructor(message);
  error.code =
    diagnostic.kind === 'parse' ? 'BABEL_PARSE_ERROR' : 'BABEL_TRANSFORM_ERROR';
  return error;
}

function metadataRules(rules) {
  return rules.map(([name, fields, priority]) => {
    const output =
      fields.constKey != null && fields.constVal != null
        ? {
            constKey: fields.constKey,
            constVal: fields.constVal,
            ltr: fields.ltr,
            rtl: fields.rtl ?? null,
          }
        : { ltr: fields.ltr, rtl: fields.rtl ?? null };
    return [name, output, priority];
  });
}

function logDiagnostic(method, message) {
  if (method === 'warn') {
    console.warn(message);
    return;
  }
  // Current Babel uses two console arguments for its package-prefixed errors.
  const match = /^\[([^\]]+)\] ([\s\S]*)$/.exec(message);
  if (match != null) console[method](`[${match[1]}]`, match[2]);
  else console[method](message);
}

function mayUseStyleX(program, options) {
  const sources = new Set(['@stylexjs/stylex', 'stylex', '@stylexjs/atoms']);
  for (const entry of Array.isArray(options.importSources)
    ? options.importSources
    : []) {
    const source = typeof entry === 'string' ? entry : entry && entry.from;
    if (typeof source === 'string') sources.add(source);
  }
  const sx =
    options.sxPropName === false
      ? null
      : typeof options.sxPropName === 'string'
        ? options.sxPropName
        : 'sx';
  let relevant = false;
  babelCore.types.traverseFast(program, (node) => {
    if (relevant) return;
    if (
      node.type === 'ImportDeclaration' &&
      node.importKind !== 'type' &&
      node.importKind !== 'typeof' &&
      sources.has(node.source.value)
    )
      relevant = true;
    if (
      node.type === 'CallExpression' &&
      node.callee.type === 'Identifier' &&
      node.callee.name === 'require' &&
      node.arguments.length === 1 &&
      node.arguments[0].type === 'StringLiteral' &&
      sources.has(node.arguments[0].value)
    )
      relevant = true;
    if (
      sx != null &&
      node.type === 'JSXOpeningElement' &&
      node.name.type === 'JSXIdentifier' &&
      node.name.name[0] === node.name.name[0].toLowerCase() &&
      node.attributes.some(
        (attr) =>
          attr.type === 'JSXAttribute' &&
          attr.name.type === 'JSXIdentifier' &&
          attr.name.name === sx,
      )
    )
      relevant = true;
  });
  return relevant;
}

function lineStarts(source) {
  const starts = [0];
  const breaks = /\r\n|\n|\r|\u2028|\u2029/g;
  for (let match; (match = breaks.exec(source)); )
    starts.push(match.index + match[0].length);
  return starts;
}

function remapLocations(parsed, nativeMap, priorMap, source) {
  const native = new TraceMap(nativeMap);
  const prior = priorMap == null ? null : new TraceMap(priorMap);
  const starts = lineStarts(source);
  function trace(position) {
    let result = originalPositionFor(native, {
      line: position.line,
      column: position.column,
    });
    if (result.line == null) return null;
    if (prior != null)
      result = originalPositionFor(prior, {
        line: result.line,
        column: result.column,
      });
    if (result.line == null || starts[result.line - 1] == null) return null;
    return {
      line: result.line,
      column: result.column,
      index: starts[result.line - 1] + result.column,
    };
  }
  function remap(node) {
    if (node.loc == null) return;
    const start = trace(node.loc.start);
    const end = trace(node.loc.end);
    if (start == null || end == null || end.index < start.index) {
      node.loc = null;
      delete node.start;
      delete node.end;
      return;
    }
    // Locations belong to this Babel input file. Leave filename unset so the
    // generator uses its configured sourceFileName instead of registering the
    // native compiler's absolute parse filename as a second source.
    node.loc = {
      start,
      end,
      identifierName: node.loc.identifierName,
    };
    node.start = start.index;
    node.end = end.index;
  }
  babelCore.types.traverseFast(parsed.program, remap);
  for (const comment of parsed.comments || []) remap(comment);
}

function styleXTransform() {
  return {
    name: 'stylex-rust',
    visitor: {
      Program: {
        enter(program, state) {
          state.file.metadata.stylex = [];
          // No compile/reparse for unrelated files. Their original locations and
          // syntax-plugin AST stay intact for subsequent Babel passes.
          if (!mayUseStyleX(program.node, state.opts)) return;
          const filename = state.file.opts.filename || 'unknown.js';
          const grammar = parseOptions(state, filename);
          const original = babelCore.parseSync(state.file.code, grammar);
          const originalGenerated = generate(
            original.program,
            { comments: true },
            state.file.code,
          ).code;
          const current = generate(
            program.node,
            { comments: true, sourceMaps: true, sourceFileName: filename },
            state.file.code,
          );
          const unchangedInput = originalGenerated === current.code;
          const source = unchangedInput ? state.file.code : current.code;
          let result;
          try {
            result = compiler.transformSync(source, {
              ...state.opts,
              filename,
              __sourceFileName: state.file.opts.filename || '',
              ...(state.file.opts.sourceType == null
                ? {}
                : { __sourceType: state.file.opts.sourceType }),
              sourceMaps: true,
            });
          } catch (error) {
            const diagnostic = diagnosticFromError(error);
            if (diagnostic != null)
              throw diagnosticError(diagnostic, source, state);
            throw error;
          }
          if (result.diagnostic != null)
            throw diagnosticError(result.diagnostic, source, state);
          if (result.metadata != null && Array.isArray(result.metadata.stylex))
            state.file.metadata.stylex = metadataRules(result.metadata.stylex);
          for (const warning of result.warnings || [])
            logDiagnostic('warn', warning);
          for (const error of result.errors || [])
            logDiagnostic('error', error);
          // Even a relevant import can be type-only, unused or contain no StyleX
          // DSL. Never replace a no-op AST merely to refresh its printed source.
          if (result.code === source) return;
          const parsed = babelCore.parseSync(result.code, grammar);
          if (parsed == null)
            throw new Error(
              'The native StyleX compiler produced no JavaScript program',
            );
          if (result.map == null)
            throw new Error(
              'The native StyleX compiler did not provide a source map for changed code',
            );
          remapLocations(
            parsed,
            result.map,
            unchangedInput ? null : current.map,
            state.file.code,
          );
          program.node.body = parsed.program.body;
          program.node.directives = parsed.program.directives;
          program.node.interpreter = parsed.program.interpreter;
          for (const field of [
            'leadingComments',
            'innerComments',
            'trailingComments',
          ]) {
            if (parsed.program[field] == null) delete program.node[field];
            else program.node[field] = parsed.program[field];
          }
          state.file.ast.comments = parsed.comments;
          program.scope.crawl();
        },
      },
    },
  };
}

styleXTransform.withOptions = (options) => [styleXTransform, options];
styleXTransform.processStylexRules = compiler.processStylexRules;
module.exports = styleXTransform;
