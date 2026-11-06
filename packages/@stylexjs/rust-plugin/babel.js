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
const { NodePath } = babelRequire('@babel/traverse');
const babelParser = babelRequire('@babel/parser');
const { TraceMap, originalPositionFor } = babelRequire(
  '@jridgewell/trace-mapping',
);
const { codeFrameColumns } = require('@babel/code-frame');
const DIAGNOSTIC_PREFIX = 'STYLEX_DIAGNOSTIC:';
const t = babelCore.types;
const BUILT_IN_SOURCES = ['@stylexjs/stylex', 'stylex', '@stylexjs/atoms'];
// Calls the original plugin compiles on Program.exit rather than on visit.
const EXIT_CALLS = new Set(['props', 'attrs', 'merge', 'default']);

// ---------------------------------------------------------------------------
// Parsing with the file's own grammar and parser.

// Parser overrides (notably Hermes) produce the file's AST.
function parserOverrides(state) {
  return (state.file.opts.plugins || []).filter(
    (plugin) => typeof plugin.parserOverride === 'function',
  );
}

// Babel-generated text needs Babel's parser with the syntax the AST contains,
// even when an override (whose plugins Babel does not list) produced the AST.
// Exactly the node types `t.isFlow` and `t.isTypeScript` accept.
const FLOW_TYPES = new Set(t.FLIPPED_ALIAS_KEYS.Flow);
const TYPESCRIPT_TYPES = new Set(t.FLIPPED_ALIAS_KEYS.TypeScript);

function syntaxOf(node, syntax) {
  const type = node.type;
  if (type.startsWith('JSX')) syntax.jsx = true;
  if (FLOW_TYPES.has(type)) syntax.flow = true;
  if (TYPESCRIPT_TYPES.has(type)) syntax.typescript = true;
}

// `known` is the syntax of the current program when the caller collected it.
function grammarOptions(state, known = null) {
  const parserOpts = { ...state.file.opts.parserOpts };
  delete parserOpts.sourceFileName;
  const plugins = [...(parserOpts.plugins || [])];
  const syntax = known ?? { jsx: false, flow: false, typescript: false };
  if (known == null)
    t.traverseFast(state.file.ast.program, (node) => syntaxOf(node, syntax));
  if (parserOverrides(state).length > 0 && !syntax.typescript) {
    syntax.jsx = true;
    syntax.flow = true;
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
  return parserOpts;
}

// ---------------------------------------------------------------------------
// Diagnostics.

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

// A position in Babel-generated text, traced back to the input file.
function tracedPosition(map, position) {
  const traced = originalPositionFor(map, {
    line: position.line,
    column: position.column - 1,
  });
  return traced.line == null
    ? null
    : { line: traced.line, column: traced.column + 1 };
}

function diagnosticError(diagnostic, input, state) {
  let message = diagnostic.message;
  const range = diagnostic.range;
  if (
    range != null &&
    Number.isInteger(range.start) &&
    Number.isInteger(range.end)
  ) {
    let source = input.code;
    let start = sourcePosition(source, range.start);
    let end = sourcePosition(source, range.end);
    if (input.map != null) {
      // The compiler read Babel-generated text; frame the user's file.
      const map = new TraceMap(input.map);
      const originalStart = tracedPosition(map, start);
      if (originalStart != null) {
        const covered = Buffer.from(source, 'utf8')
          .subarray(range.start, range.end)
          .toString('utf8');
        const startNode = originalNodeAt(state, originalStart, covered);
        source = state.file.code;
        start = originalStart;
        end =
          startNode != null && startNode.loc != null
            ? {
                line: startNode.loc.end.line,
                column: startNode.loc.end.column + 1,
              }
            : start;
      }
    }
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

// The input node whose Babel printing is the text the compiler's diagnostic
// range covered, among the nodes starting at the traced position (outermost
// first; indentation is the only difference between nested and lone prints).
function originalNodeAt(state, position, covered) {
  const candidates = [];
  t.traverseFast(state.file.ast.program, (node) => {
    const loc = node.loc;
    if (
      loc != null &&
      loc.start.line === position.line &&
      loc.start.column === position.column - 1
    )
      candidates.push(node);
  });
  const text = covered.replace(/\s+/g, '');
  return (
    candidates.find(
      (node) => generate(node).code.replace(/\s+/g, '') === text,
    ) ??
    candidates[0] ??
    null
  );
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

// Items of `next` that are not accounted for by `previous` (as a multiset).
function multisetDifference(next, previous, key = JSON.stringify) {
  const counts = new Map();
  for (const item of previous) {
    const k = key(item);
    counts.set(k, (counts.get(k) || 0) + 1);
  }
  return next.filter((item) => {
    const k = key(item);
    const count = counts.get(k) || 0;
    if (count === 0) return true;
    counts.set(k, count - 1);
    return false;
  });
}

// ---------------------------------------------------------------------------
// Which files and calls belong to StyleX.

function importSources(options) {
  const sources = new Map(BUILT_IN_SOURCES.map((source) => [source, null]));
  for (const entry of Array.isArray(options.importSources)
    ? options.importSources
    : []) {
    if (typeof entry === 'string') sources.set(entry, null);
    else if (entry != null && typeof entry.from === 'string')
      sources.set(entry.from, typeof entry.as === 'string' ? entry.as : null);
  }
  return sources;
}

function sxPropName(options) {
  return options.sxPropName === false
    ? null
    : typeof options.sxPropName === 'string'
      ? options.sxPropName
      : 'sx';
}

function isRequireOf(node, sources) {
  return (
    node != null &&
    node.type === 'CallExpression' &&
    node.callee.type === 'Identifier' &&
    node.callee.name === 'require' &&
    node.arguments.length === 1 &&
    node.arguments[0].type === 'StringLiteral' &&
    sources.has(node.arguments[0].value)
  );
}

// Whether source text provably spells no StyleX import source and no sx
// attribute, so no import, require or sx element can name one. A JSX attribute
// name spells itself. A string value spells its source unless an escape stands
// in for some of its characters: one that can stand for any character (\u, \x,
// an octal or digit escape, a line continuation) gives up, and so does an
// identity escape of a character some source contains or a control escape
// when a source holds a control character.
const CONTROL_ESCAPES = 'nrtbfv';
function cannotSpellStyleX(code, s) {
  if (s.sx != null && code.includes(s.sx)) return false;
  const needles = [...s.sources.keys()];
  if (needles.some((needle) => code.includes(needle))) return false;
  const controls = needles.some((needle) => /[\n\r\t\b\f\v]/.test(needle));
  for (let i = code.indexOf('\\'); i !== -1; i = code.indexOf('\\', i + 2)) {
    const next = code[i + 1];
    if (next === undefined || /[ux0-9\r\n\u2028\u2029]/.test(next))
      return false;
    if (CONTROL_ESCAPES.includes(next)) {
      if (controls) return false;
    } else if (needles.some((needle) => needle.includes(next))) return false;
  }
  return true;
}

// Also collects the program's syntax into `syntax`, and with `declarations`
// the local declarations by name, in the same walk.
function mayUseStyleX(
  program,
  options,
  syntax,
  declarations = null,
  visit = null,
) {
  const sources = importSources(options);
  const sx = sxPropName(options);
  let relevant = false;
  t.traverseFast(program, (node) => {
    syntaxOf(node, syntax);
    if (declarations != null) indexDeclaration(node, declarations);
    if (visit != null) visit(node);
    if (relevant) return;
    if (
      node.type === 'ImportDeclaration' &&
      node.importKind !== 'type' &&
      node.importKind !== 'typeof' &&
      sources.has(node.source.value)
    )
      relevant = true;
    if (isRequireOf(node, sources)) relevant = true;
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

function indexDeclaration(node, declarations) {
  let names = null;
  if (node.type === 'VariableDeclarator')
    names =
      node.id.type === 'Identifier'
        ? [node.id.name]
        : Object.keys(t.getBindingIdentifiers(node.id));
  else if (
    (node.type === 'FunctionDeclaration' || node.type === 'ClassDeclaration') &&
    node.id != null
  )
    names = [node.id.name];
  if (names == null) return;
  for (const name of names) {
    const list = declarations.get(name);
    if (list == null) declarations.set(name, [node]);
    else list.push(node);
  }
}

// The local declarations a subtree may read, transitively, by name: a
// superset of the bindings the compiler resolved (shadowing is ignored, which
// only adds declarations to check).
function dependencies(root, declarations) {
  const found = new Set();
  const seen = new Set();
  const pending = [root];
  while (pending.length > 0) {
    t.traverseFast(pending.pop(), (node) => {
      if (node.type !== 'Identifier' || seen.has(node.name)) return;
      seen.add(node.name);
      for (const declaration of declarations.get(node.name) || [])
        if (!found.has(declaration)) {
          found.add(declaration);
          pending.push(declaration);
        }
    });
  }
  found.delete(root);
  return [...found];
}

// Local names through which top-level imports and requires reach StyleX.
// `namespaces` hold the module object, `named` map a local to its export.
function stylexNames(program, sources) {
  const namespaces = new Set();
  const named = new Map();
  for (const statement of program.body) {
    if (
      statement.type === 'ImportDeclaration' &&
      statement.importKind !== 'type' &&
      statement.importKind !== 'typeof' &&
      sources.has(statement.source.value)
    ) {
      const as = sources.get(statement.source.value);
      for (const specifier of statement.specifiers) {
        if (specifier.type === 'ImportSpecifier') {
          if (specifier.importKind === 'type') continue;
          const imported =
            specifier.imported.type === 'Identifier'
              ? specifier.imported.name
              : specifier.imported.value;
          if (as != null && imported === as)
            namespaces.add(specifier.local.name);
          else named.set(specifier.local.name, imported);
        } else namespaces.add(specifier.local.name);
      }
    }
    if (statement.type === 'VariableDeclaration') {
      for (const declarator of statement.declarations) {
        let init = declarator.init;
        // `_interopRequireWildcard(require(...))` and similar wrappers.
        while (
          init != null &&
          init.type === 'CallExpression' &&
          !isRequireOf(init, sources) &&
          init.arguments.length > 0
        )
          init = init.arguments[0];
        if (!isRequireOf(init, sources)) continue;
        if (declarator.id.type === 'Identifier')
          namespaces.add(declarator.id.name);
        else if (declarator.id.type === 'ObjectPattern') {
          for (const property of declarator.id.properties) {
            if (
              property.type === 'ObjectProperty' &&
              property.key.type === 'Identifier' &&
              property.value.type === 'Identifier'
            )
              named.set(property.value.name, property.key.name);
          }
        }
      }
    }
  }
  return {
    namespaces,
    named,
    empty: namespaces.size === 0 && named.size === 0,
  };
}

// The StyleX API a call uses (`create`, `props`, `default` for `stylex(...)`),
// or null for any other call.
function stylexCall(node, names) {
  let callee = node.callee;
  if (callee.type === 'SequenceExpression')
    callee = callee.expressions[callee.expressions.length - 1];
  if (callee.type === 'Identifier') {
    if (names.named.has(callee.name)) return names.named.get(callee.name);
    return names.namespaces.has(callee.name) ? 'default' : null;
  }
  let property = null;
  while (callee.type === 'MemberExpression') {
    property = callee.property;
    callee = callee.object;
  }
  if (callee.type !== 'Identifier' || property == null) return null;
  const name =
    property.type === 'Identifier'
      ? property.name
      : property.type === 'StringLiteral'
        ? property.value
        : '';
  if (names.namespaces.has(callee.name)) return name;
  return names.named.has(callee.name) ? names.named.get(callee.name) : null;
}

function sxAttribute(node, sx) {
  return (
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
  );
}

// ---------------------------------------------------------------------------
// Structural equality of ASTs, ignoring positions, comments, parser extras and
// the field-presence differences between parsers (an absent `optional` and
// `optional: false` describe the same program). Hashes only reject: every
// subtree kept on a hash match is first confirmed equal.

const IGNORED_FIELDS = new Set([
  'type',
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
  'sourceFile',
  'parent',
]);

function isNode(value) {
  return (
    value != null &&
    typeof value === 'object' &&
    typeof value.type === 'string' &&
    !Array.isArray(value)
  );
}

const typeFields = new Map();
function fieldsOf(node) {
  let names = typeFields.get(node.type);
  if (names !== undefined) return names;
  const fields = t.NODE_FIELDS[node.type];
  if (fields == null)
    // A node type Babel does not define: compare its own data fields.
    return Object.keys(node).filter(
      (key) => key[0] !== '_' && !IGNORED_FIELDS.has(key),
    );
  names = Object.keys(fields).filter((key) => !IGNORED_FIELDS.has(key));
  typeFields.set(node.type, names);
  return names;
}

// Scalars that describe the same program compare equal.
function absent(value) {
  return (
    value == null ||
    value === false ||
    (Array.isArray(value) && value.length === 0)
  );
}

function sameScalar(a, b) {
  if (absent(a) && absent(b)) return true;
  if (a === b) return true;
  if (typeof a === 'object' && typeof b === 'object' && a && b)
    return JSON.stringify(a) === JSON.stringify(b);
  return Number.isNaN(a) && Number.isNaN(b);
}

function mix(hash, value) {
  return Math.imul(hash ^ value, 0x01000193) >>> 0;
}

// Strings hash by an interned id: V8 hashes and compares them natively, which
// is far cheaper than mixing every character. Ids are only stable within one
// table, so every hash compared with another comes from the same `Shapes`.
function hashString(hash, text, strings) {
  let id = strings.get(text);
  if (id === undefined) {
    id = strings.size + 1;
    strings.set(text, id);
  }
  return mix(hash, id);
}

const SCALAR_TAGS = { string: 1, number: 2, boolean: 3, bigint: 4 };

function hashScalar(hash, value, strings) {
  if (absent(value)) return hash;
  if (typeof value === 'object')
    return hashString(mix(hash, 6), JSON.stringify(value), strings);
  return hashString(
    mix(hash, SCALAR_TAGS[typeof value] || 5),
    String(value),
    strings,
  );
}

function commentsEqual(a, b) {
  for (const field of COMMENT_FIELDS) {
    const left = a[field] || [];
    const right = b[field] || [];
    if (left.length !== right.length) return false;
    for (let index = 0; index < left.length; index++)
      if (
        left[index].type !== right[index].type ||
        left[index].value !== right[index].value
      )
        return false;
  }
  return true;
}

// Exact structural equality; with `comments`, attached comment text too.
// Nodes of `b` in `opaque` only need a counterpart of the same type.
function equalTrees(a, b, comments, opaque = null) {
  if (a === b) return true;
  if (opaque != null && isNode(b) && opaque.has(b))
    return isNode(a) && a.type === b.type;
  if (!isNode(a) || !isNode(b)) return sameScalar(a, b);
  if (a.type !== b.type) return false;
  if (comments && !commentsEqual(a, b)) return false;
  for (const field of fieldsOf(a)) {
    const left = a[field];
    const right = b[field];
    if (Array.isArray(left) || Array.isArray(right)) {
      const l = left || [];
      const r = right || [];
      if (l.length !== r.length) return false;
      for (let index = 0; index < l.length; index++)
        if (!equalTrees(l[index], r[index], comments, opaque)) return false;
    } else if (isNode(left) || isNode(right)) {
      if (!equalTrees(left, right, comments, opaque)) return false;
    } else if (!sameScalar(left, right)) return false;
  }
  return true;
}

// A compact exact record of a subtree's structure, as `equalTrees` compares
// it: node types, field contents and list lengths in a fixed walk order. A
// subtree matches its record only if it is structurally equal to the subtree
// recorded, whatever an in-place edit changed (the record shares no nodes).
// Scalars are a numeric tag then the value, so no scalar can align with a
// node type (a string) after a structural change.
const ABSENT = 0;
const SCALAR = 1;
const OBJECT = 2;
// A node recorded as opaque: any subtree matches there (definitions StyleX
// itself replaces at their visit, which carry records of their own).
const OPAQUE = 3;

function shapeRecord(node, record = [], opaque = null) {
  if (!isNode(node)) {
    recordScalar(node, record);
    return record;
  }
  if (opaque != null && opaque.has(node)) {
    record.push(OPAQUE);
    return record;
  }
  record.push(node.type);
  for (const field of fieldsOf(node)) {
    const value = node[field];
    if (Array.isArray(value)) {
      record.push(value.length);
      for (const item of value) shapeRecord(item, record, opaque);
    } else if (isNode(value)) shapeRecord(value, record, opaque);
    else recordScalar(value, record);
  }
  return record;
}

// Scalars that describe the same program record the same (see `sameScalar`).
function recordScalar(value, record) {
  if (absent(value)) record.push(ABSENT);
  else if (typeof value === 'object')
    record.push(OBJECT, JSON.stringify(value));
  else record.push(SCALAR, value);
}

// Whether another plugin of this pass can change a definition before StyleX
// visits it: any visitor of it may run first at the definition or an ancestor,
// and a Program.enter of a later plugin runs before the program's children.
// Program.exit runs after every visit.
function othersMayEditDefinitions(state) {
  const opts = state.file.opts;
  const passes = [
    opts.plugins || [],
    ...(opts.presets || []).map((p) => p.plugins),
  ];
  const pass = passes.find((plugins) =>
    plugins.some((plugin) => plugin.key === state.key),
  );
  if (pass == null) return true;
  return pass.some((plugin) => {
    if (plugin.key === state.key) return false;
    const visitor = plugin.visitor || {};
    return Object.keys(visitor).some(
      (key) =>
        key !== '_exploded' &&
        key !== '_verified' &&
        (key !== 'Program' || visitor.Program.enter != null),
    );
  });
}

// The plugins of the pass StyleX runs in, and StyleX's place in it.
function passOf(state) {
  const opts = state.file.opts;
  const passes = [
    opts.plugins || [],
    ...(opts.presets || []).map((p) => p.plugins),
  ];
  const index = passes.findIndex((plugins) =>
    plugins.some((plugin) => plugin.key === state.key),
  );
  return index < 0 ? null : { first: index === 0, plugins: passes[index] };
}

const visitorKeys = (plugin) =>
  Object.keys(plugin.visitor || {}).filter(
    (key) => key !== '_exploded' && key !== '_verified',
  );

// Whether StyleX is the only plugin of its pass that sees the program: no
// other plugin of the pass has a visitor or a `pre` hook, so nothing but StyleX
// changes the program during the traversal.
function aloneInPass(state) {
  // A visitor wrapper runs code around every visitor, StyleX's included.
  if (state.file.opts.wrapPluginVisitorMethod != null) return false;
  const pass = passOf(state);
  return (
    pass != null &&
    pass.plugins.every(
      (plugin) =>
        plugin.key === state.key ||
        (plugin.pre == null && visitorKeys(plugin).length === 0),
    )
  );
}

// The prototype of the nodes Babel's own parser makes (the parser Babel core
// itself resolves): an input AST is cloned into plain objects.
const PARSED = Object.getPrototypeOf(babelParser.parse(''));

// Whether the program is still exactly Babel's parse of the file's text, so the
// text describes it without a second parse to compare: Babel parsed it (rather
// than being handed an AST), in the first pass, and no plugin before StyleX's
// Program.enter could have changed it -- no other plugin of the pass has a
// `pre` hook, and none before StyleX enters the program.
function parsedAndUntouched(state, program) {
  const opts = state.file.opts;
  // An AST handed to `transformFromAst` is deep-cloned into plain objects
  // unless `cloneInputAst` is off: with it off, the caller's own (possibly
  // edited) parser nodes reach the plugins, so their prototype proves nothing.
  if (opts.cloneInputAst !== true) return false;
  // A visitor wrapper can change the program before StyleX's Program.enter.
  if (opts.wrapPluginVisitorMethod != null) return false;
  if (
    state.file.ast.program !== program.node ||
    Object.getPrototypeOf(state.file.ast) !== PARSED ||
    Object.getPrototypeOf(program.node) !== PARSED
  )
    return false;
  // A parser override in any pass parses instead of Babel.
  const passes = [
    opts.plugins || [],
    ...(opts.presets || []).map((preset) => preset.plugins),
  ];
  if (
    passes.some((plugins) =>
      plugins.some((plugin) => typeof plugin.parserOverride === 'function'),
    )
  )
    return false;
  const pass = passOf(state);
  if (pass == null || !pass.first) return false;
  let before = true;
  for (const plugin of pass.plugins) {
    if (plugin.key === state.key) {
      before = false;
      continue;
    }
    if (plugin.pre != null) return false;
    const visitor = plugin.visitor || {};
    if (before && (visitor.enter != null || visitor.Program?.enter != null))
      return false;
  }
  return true;
}

// Parser options under which JSX parses without the `jsx` plugin.
function impliesJsx(parserOpts) {
  return (parserOpts?.plugins || []).some(
    (plugin) =>
      Array.isArray(plugin) &&
      plugin[0] === 'typescript' &&
      plugin[1]?.isTSX === true,
  );
}

function matchesShapeRecord(node, record) {
  let index = 0;
  const scalar = (value) => {
    const tag = record[index++];
    if (absent(value)) return tag === ABSENT;
    if (typeof value === 'object')
      return tag === OBJECT && record[index++] === JSON.stringify(value);
    if (tag !== SCALAR) return false;
    const recorded = record[index++];
    return (
      recorded === value ||
      (typeof value === 'number' &&
        Number.isNaN(value) &&
        Number.isNaN(recorded))
    );
  };
  const walk = (current) => {
    if (record[index] === OPAQUE) {
      index++;
      return true;
    }
    if (!isNode(current)) return scalar(current);
    if (record[index++] !== current.type) return false;
    for (const field of fieldsOf(current)) {
      const value = current[field];
      if (Array.isArray(value)) {
        if (record[index++] !== value.length) return false;
        for (const item of value) if (!walk(item)) return false;
      } else if (isNode(value)) {
        if (!walk(value)) return false;
      } else if (!scalar(value)) return false;
    }
    return true;
  };
  return walk(node) && index === record.length;
}

class Shapes {
  constructor() {
    this.hashes = new WeakMap();
    this.strings = new Map();
  }

  hash(node) {
    let hash = this.hashes.get(node);
    if (hash !== undefined) return hash;
    const strings = this.strings;
    hash = hashString(0x811c9dc5, node.type, strings);
    const fields = fieldsOf(node);
    for (let position = 0; position < fields.length; position++) {
      const value = node[fields[position]];
      if (absent(value)) continue;
      hash = mix(hash, position + 1);
      if (Array.isArray(value)) {
        hash = mix(hash, value.length);
        for (const item of value)
          hash = isNode(item)
            ? mix(hash, this.hash(item))
            : hashScalar(hash, item, strings);
      } else if (isNode(value)) hash = mix(hash, this.hash(value));
      else hash = hashScalar(hash, value, strings);
    }
    this.hashes.set(node, hash);
    return hash;
  }

  itemKey(item) {
    return isNode(item) ? this.hash(item) : hashScalar(1, item, this.strings);
  }

  // A key equal for equal subtrees from the node's type and own scalars only:
  // items whose shallow keys differ cannot match, so they need no full hash.
  shallowKey(item) {
    if (!isNode(item)) return hashScalar(1, item, this.strings);
    const strings = this.strings;
    let hash = hashString(0x811c9dc5, item.type, strings);
    const fields = fieldsOf(item);
    for (let position = 0; position < fields.length; position++) {
      const value = item[fields[position]];
      if (absent(value) || Array.isArray(value) || isNode(value)) continue;
      hash = hashScalar(mix(hash, position + 1), value, strings);
    }
    return hash;
  }

  // A key equal for equal subtrees from the first `depth` levels only.
  boundedKey(item, depth) {
    if (!isNode(item)) return hashScalar(1, item, this.strings);
    const strings = this.strings;
    let hash = hashString(0x811c9dc5, item.type, strings);
    const fields = fieldsOf(item);
    for (let position = 0; position < fields.length; position++) {
      const value = item[fields[position]];
      if (absent(value)) continue;
      hash = mix(hash, position + 1);
      if (Array.isArray(value)) {
        hash = mix(hash, value.length);
        if (depth > 1)
          for (const child of value)
            hash = mix(hash, this.boundedKey(child, depth - 1));
      } else if (isNode(value)) {
        if (depth > 1) hash = mix(hash, this.boundedKey(value, depth - 1));
      } else hash = hashScalar(hash, value, strings);
    }
    return hash;
  }

  // Full keys for the items of two lists, computed only where an equal item
  // can exist on the other side; every other item gets a key matching nothing.
  // The filter looks a few levels deep: statements of one kind usually differ
  // just below their top (an export of a call vs. of an object).
  listKeys(before, after) {
    const shallowBefore = before.map((item) => this.boundedKey(item, 4));
    const shallowAfter = after.map((item) => this.boundedKey(item, 4));
    const inAfter = new Set(shallowAfter);
    const inBefore = new Set(shallowBefore);
    return [
      before.map((item, index) =>
        inAfter.has(shallowBefore[index]) ? this.itemKey(item) : `b${index}`,
      ),
      after.map((item, index) =>
        inBefore.has(shallowAfter[index]) ? this.itemKey(item) : `a${index}`,
      ),
    ];
  }

  // Exact equality, confirmed structurally after a hash match.
  same(a, b) {
    if (a === b) return true;
    if (!isNode(a) || !isNode(b)) return sameScalar(a, b);
    return this.hash(a) === this.hash(b) && equalTrees(a, b, false);
  }

  // Only the nodes' own non-node fields.
  sameScalars(a, b) {
    for (const field of new Set([...fieldsOf(a), ...fieldsOf(b)])) {
      const left = a[field];
      const right = b[field];
      if (
        Array.isArray(left) ||
        isNode(left) ||
        Array.isArray(right) ||
        isNode(right)
      )
        continue;
      if (!sameScalar(left, right)) return false;
    }
    return true;
  }
}

// ---------------------------------------------------------------------------
// Grafting: apply a compiler result to the live AST by replacing only the
// subtrees whose structure changed. Every unchanged node keeps its identity,
// so other plugins' node-keyed state and unrelated comments survive.

// The outer bindings a declaration statement declares, as one key; null for
// anything else (and for declarations of no name).
const DECLARATIONS = new Set([
  'VariableDeclaration',
  'FunctionDeclaration',
  'ClassDeclaration',
  'ExportNamedDeclaration',
  'ExportDefaultDeclaration',
  'VariableDeclarator',
]);
function declaredKey(node) {
  if (!isNode(node) || !DECLARATIONS.has(node.type)) return null;
  // The common shapes directly: one declarator of a name, a named function or
  // class, or an export of one of those.
  const declaration =
    node.type === 'ExportNamedDeclaration' ? node.declaration : node;
  if (declaration != null) {
    if (
      declaration.type === 'VariableDeclaration' &&
      declaration.declarations.length === 1 &&
      declaration.declarations[0].id.type === 'Identifier'
    )
      return declaration.declarations[0].id.name;
    if (
      declaration.type === 'VariableDeclarator' &&
      declaration.id.type === 'Identifier'
    )
      return declaration.id.name;
    if (
      (declaration.type === 'FunctionDeclaration' ||
        declaration.type === 'ClassDeclaration') &&
      declaration.id != null
    )
      return declaration.id.name;
  }
  const names = Object.keys(t.getBindingIdentifiers(node, false, true));
  return names.length === 0 ? null : names.join(',');
}

class Graft {
  constructor(shapes, record, defer = null, hold = null) {
    this.shapes = shapes;
    this.record = record;
    // Items the compile dropped that must stay until the traversal ends.
    this.defer = defer;
    this.deferred = [];
    // Nodes that keep their place until the traversal visits them; their
    // replacement waits in `held`.
    this.hold = hold;
    this.held = [];
    this.edits = [];
    // Whether the live tree changed at all (held replacements wait outside).
    this.changed = false;
    this.adopted = new Set();
    // [replaced node, replacement] pairs.
    this.replaced = [];
    // Unmatched list runs, as { removed, added } (with `record`).
    this.regions = [];
    // The list each deferred item stays in.
    this.deferredIn = new Map();
    // Commented items removed outright, with their neighbours in the result.
    this.removed = [];
  }

  adopt(node, previous = null) {
    this.adopted.add(node);
    if (isNode(previous)) this.replaced.push([previous, node]);
    return node;
  }

  node(previous, next) {
    if (previous === next) return previous;
    if (!isNode(previous) || !isNode(next) || previous.type !== next.type)
      return isNode(next) ? this.replace(previous, next) : next;
    if (!this.shapes.sameScalars(previous, next))
      return this.replace(previous, next);
    // Descending compares the children (lists by direct equality first), so
    // an equal subtree is walked once and an edited one is never hashed whole.
    this.children(previous, next);
    return previous;
  }

  replace(previous, next) {
    this.adopt(next, previous);
    if (isNode(previous) && this.hold != null && this.hold(previous)) {
      this.held.push([previous, next]);
      return previous;
    }
    return next;
  }

  children(previous, next) {
    const fields = new Set([...fieldsOf(previous), ...fieldsOf(next)]);
    for (const field of fields) {
      const before = previous[field];
      const after = next[field];
      if (Array.isArray(before) || Array.isArray(after)) {
        this.list(previous, field, before || [], after || []);
      } else if (isNode(before) && isNode(after)) {
        const result = this.node(before, after);
        if (result !== before) this.set(previous, field, result);
      } else if (isNode(before) || isNode(after)) {
        this.set(
          previous,
          field,
          isNode(after) ? this.adopt(after, before) : after,
        );
      }
    }
  }

  set(parent, field, value) {
    this.changed = true;
    if (this.record)
      this.edits.push({ parent, field, before: parent[field], after: value });
    parent[field] = value;
  }

  list(parent, field, before, after) {
    const shapes = this.shapes;
    const pairs = matchItems(before, after, shapes);
    const result = [];
    let declared = null;
    const removed = [];
    const drop = (item) => {
      if (this.defer == null || !isNode(item) || !this.defer(item)) {
        if (isNode(item) && (item.leadingComments || item.trailingComments))
          removed.push({ item, at: result.length });
        return;
      }
      // A declaration the compile re-emitted elsewhere in this list was
      // rewritten, not removed: its new form is where the definition went.
      if (declared == null) {
        declared = new Map();
        // Outer bindings only: a function's parameters are not declarations
        // of its scope (`function C(s)` does not declare the program's `s`).
        for (const other of after)
          if (isNode(other))
            for (const name of Object.keys(
              t.getBindingIdentifiers(other, false, true),
            ))
              if (!declared.has(name)) declared.set(name, other);
      }
      const rewritten = Object.keys(t.getBindingIdentifiers(item, false, true))
        .map((name) => declared.get(name))
        .find((other) => other != null);
      if (rewritten != null) {
        this.replaced.push([item, rewritten]);
        return;
      }
      this.deferred.push(item);
      this.deferredIn.set(item, { parent, field });
      result.push(item);
    };
    let i = 0;
    let j = 0;
    const gap = (iEnd, jEnd) => {
      // With `record`, the items a gap drops and adopts whole are kept as one
      // region: the adopted items are what the compile made of the dropped.
      // Items paired and grafted within keep their own, finer regions.
      const region = this.record ? { removed: [], added: [] } : null;
      const adopted = (next) => {
        if (region != null && isNode(next)) region.added.push(next);
        return isNode(next) ? this.adopt(next) : next;
      };
      const dropped = (item) => {
        if (region != null && isNode(item)) region.removed.push(item);
        drop(item);
      };
      // Unmatched runs: pair same-typed items in order and graft within them.
      // Types still ahead in the run, counted once (a lookahead scan per item
      // would be quadratic in long runs such as arrays of definitions).
      const ahead = new Map();
      for (let k = j; k < jEnd; k++) {
        const type = isNode(after[k]) ? after[k].type : null;
        ahead.set(type, (ahead.get(type) || 0) + 1);
      }
      // Declared names of the items ahead in the run, counted on first need.
      let aheadKeys = null;
      const aheadOf = (key) => {
        if (aheadKeys == null) {
          aheadKeys = new Map();
          for (let k = j; k < jEnd; k++) {
            const other = declaredKey(after[k]);
            if (other != null)
              aheadKeys.set(other, (aheadKeys.get(other) || 0) + 1);
          }
        }
        return aheadKeys.get(key) || 0;
      };
      const passed = (next) => {
        const type = isNode(next) ? next.type : null;
        ahead.set(type, ahead.get(type) - 1);
        if (aheadKeys != null) {
          const key = declaredKey(next);
          if (key != null) aheadKeys.set(key, aheadKeys.get(key) - 1);
        }
      };
      // Declarations of different bindings never pair: the compiled `const u`
      // is not what became of an unused `const s` before it. Names declared
      // by the items still unmatched in the run are counted on first need.
      let remaining = null;
      const remainingOf = (key) => {
        if (remaining == null) {
          remaining = new Map();
          for (let k = i; k < iEnd; k++) {
            const other = declaredKey(before[k]);
            if (other != null)
              remaining.set(other, (remaining.get(other) || 0) + 1);
          }
        }
        return remaining.get(key) || 0;
      };
      // An item leaving the run: it no longer counts as remaining.
      const left = (item) => {
        if (remaining != null) {
          const key = declaredKey(item);
          if (key != null) remaining.set(key, remaining.get(key) - 1);
        }
        return item;
      };
      while (i < iEnd && j < jEnd) {
        const previous = before[i];
        const next = after[j];
        const sameType =
          isNode(previous) && isNode(next) && previous.type === next.type;
        const previousKey = declaredKey(previous);
        const nextKey = sameType ? declaredKey(next) : null;
        if (previousKey != null && aheadOf(previousKey) === 0) {
          // No compiled declaration of these bindings ahead: removed.
          dropped(left(before[i++]));
        } else if (sameType && previousKey !== nextKey) {
          // The compiled declaration's own source comes later in the run:
          // this item is gone. Otherwise the compiled one is new here.
          if (
            nextKey != null &&
            previousKey == null &&
            remainingOf(nextKey) > 0
          )
            dropped(left(before[i++]));
          else {
            passed(after[j++]);
            result.push(adopted(next));
          }
        } else if (sameType) {
          passed(after[j++]);
          result.push(this.node(left(previous), next));
          i++;
        } else if (isNode(previous) && ahead.get(previous.type) > 0) {
          passed(after[j++]);
          result.push(adopted(next));
        } else if (
          isNode(previous) &&
          isNode(next) &&
          this.hold != null &&
          this.hold(previous)
        ) {
          // A definition the compile replaced in place (an array element, an
          // argument): it stays until visited, like any other definition.
          passed(after[j++]);
          result.push(this.replace(left(previous), next));
          i++;
        } else dropped(left(before[i++]));
      }
      for (; j < jEnd; j++) result.push(adopted(after[j]));
      while (i < iEnd) dropped(left(before[i++]));
      if (
        region != null &&
        region.removed.length > 0 &&
        region.added.length > 0
      )
        this.regions.push(region);
    };
    for (const [bi, ai] of pairs) {
      gap(bi, ai);
      result.push(before[bi]);
      i = bi + 1;
      j = ai + 1;
    }
    gap(before.length, after.length);
    for (const { item, at } of removed)
      this.removed.push({
        item,
        previous: isNode(result[at - 1]) ? result[at - 1] : null,
        next: isNode(result[at]) ? result[at] : null,
      });
    const unchanged =
      result.length === before.length &&
      result.every((item, index) => item === before[index]);
    if (unchanged) return;
    this.changed = true;
    if (this.record)
      this.edits.push({
        parent,
        field,
        list: true,
        before: before.slice(),
        after: result.slice(),
      });
    if (Array.isArray(parent[field]))
      parent[field].splice(0, parent[field].length, ...result);
    else parent[field] = result;
  }
}

// Applies a compiled program to a live Babel program, in place: only the
// subtrees whose structure differs are replaced, so every unchanged node keeps
// its identity (and its comments). Internal and frontend-agnostic: the
// compiled program is any compiler's output parsed to a Babel AST. `adopted` holds the replacement subtrees, whose
// locations the caller traces to the input; `edits` (with `record`) undo the
// graft through `revert`.
function graftProgram(
  program,
  compiled,
  { record = false, defer = null, hold = null } = {},
) {
  const graft = new Graft(new Shapes(), record, defer, hold);
  graft.children(program, compiled);
  return {
    adopted: graft.adopted,
    edits: graft.edits,
    replaced: graft.replaced,
    deferred: graft.deferred,
    held: graft.held,
    regions: graft.regions,
    removed: graft.removed,
    deferredIn: graft.deferredIn,
    changed: graft.changed,
  };
}

// Equal items of two lists, as index pairs in order. Equal prefixes and
// suffixes are compared directly (a difference ends the comparison early, so
// an edited subtree is never hashed whole); only the changed middle is keyed.
function matchItems(before, after, shapes) {
  const pairs = [];
  let start = 0;
  while (
    start < before.length &&
    start < after.length &&
    equalTrees(before[start], after[start], false)
  ) {
    pairs.push([start, start]);
    start++;
  }
  let endBefore = before.length;
  let endAfter = after.length;
  const suffix = [];
  while (
    endBefore > start &&
    endAfter > start &&
    equalTrees(before[endBefore - 1], after[endAfter - 1], false)
  ) {
    endBefore--;
    endAfter--;
    suffix.push([endBefore, endAfter]);
  }
  const n = endBefore - start;
  const m = endAfter - start;
  let aligned;
  if (n > 0 && m > 0 && n * m <= 64) {
    // A short changed run (typically one edited statement): compare its
    // items directly rather than hashing whole subtrees.
    const shallow = (item) => shapes.shallowKey(item);
    const keysAfter = after.slice(start, endAfter).map(shallow);
    const equal = [];
    for (let x = 0; x < n; x++) {
      const key = shallow(before[start + x]);
      equal.push(
        keysAfter.map(
          (other, y) =>
            other === key &&
            equalTrees(before[start + x], after[start + y], false),
        ),
      );
    }
    const table = new Uint32Array((n + 1) * (m + 1));
    for (let x = n - 1; x >= 0; x--)
      for (let y = m - 1; y >= 0; y--)
        table[x * (m + 1) + y] = equal[x][y]
          ? table[(x + 1) * (m + 1) + y + 1] + 1
          : Math.max(table[(x + 1) * (m + 1) + y], table[x * (m + 1) + y + 1]);
    let x = 0;
    let y = 0;
    while (x < n && y < m) {
      if (equal[x][y]) {
        pairs.push([start + x, start + y]);
        x++;
        y++;
      } else if (table[(x + 1) * (m + 1) + y] >= table[x * (m + 1) + y + 1])
        x++;
      else y++;
    }
  } else if (
    n > 0 &&
    n === m &&
    (aligned = positional(before, after, start, n, shapes)) != null
  ) {
    // Items replaced in place (an array of definitions): the equal items at
    // equal positions are the whole common subsequence, since no remaining
    // item could equal any remaining item on the other side.
    for (const x of aligned) pairs.push([x, x]);
  } else if (n > 0 && m > 0) {
    const middleBefore = before.slice(start, endBefore);
    const middleAfter = after.slice(start, endAfter);
    const [keysBefore, keysAfter] = shapes.listKeys(middleBefore, middleAfter);
    for (const [x, y] of matchLists(keysBefore, keysAfter))
      if (shapes.same(middleBefore[x], middleAfter[y]))
        pairs.push([start + x, start + y]);
  }
  return pairs.concat(suffix.reverse());
}

function equalItem(a, b, shapes) {
  return (
    shapes.shallowKey(a) === shapes.shallowKey(b) && equalTrees(a, b, false)
  );
}

// The equal positions of two equally long runs, when matching them is optimal:
// once those pairs are taken, no unmatched item shares even a shallow key with
// an unmatched item on the other side. Null otherwise.
function positional(before, after, start, length, shapes) {
  const equal = [];
  const unmatchedBefore = new Set();
  const unmatchedAfter = [];
  for (let x = start; x < start + length; x++) {
    if (equalItem(before[x], after[x], shapes)) equal.push(x);
    else {
      unmatchedBefore.add(shapes.shallowKey(before[x]));
      unmatchedAfter.push(shapes.shallowKey(after[x]));
    }
  }
  return unmatchedAfter.some((key) => unmatchedBefore.has(key)) ? null : equal;
}

// Longest common subsequence of two key lists, as index pairs. Edits are
// sparse, so equal prefixes and suffixes are matched first.
function matchLists(a, b) {
  const pairs = [];
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) {
    pairs.push([start, start]);
    start++;
  }
  let endA = a.length;
  let endB = b.length;
  const suffix = [];
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
    endA--;
    endB--;
    suffix.push([endA, endB]);
  }
  const n = endA - start;
  const m = endB - start;
  if (n > 0 && m > 0) {
    if (n * m <= 1 << 20) {
      const table = new Uint32Array((n + 1) * (m + 1));
      for (let x = n - 1; x >= 0; x--)
        for (let y = m - 1; y >= 0; y--)
          table[x * (m + 1) + y] =
            a[start + x] === b[start + y]
              ? table[(x + 1) * (m + 1) + y + 1] + 1
              : Math.max(
                  table[(x + 1) * (m + 1) + y],
                  table[x * (m + 1) + y + 1],
                );
      let x = 0;
      let y = 0;
      while (x < n && y < m) {
        if (a[start + x] === b[start + y]) {
          pairs.push([start + x, start + y]);
          x++;
          y++;
        } else if (table[(x + 1) * (m + 1) + y] >= table[x * (m + 1) + y + 1])
          x++;
        else y++;
      }
    } else {
      // Very long changed runs: match each key to its next occurrence.
      const positions = new Map();
      for (let y = start; y < endB; y++) {
        if (!positions.has(b[y])) positions.set(b[y], []);
        positions.get(b[y]).push(y);
      }
      const next = new Map();
      let last = start - 1;
      for (let x = start; x < endA; x++) {
        const candidates = positions.get(a[x]);
        if (candidates == null) continue;
        let cursor = next.get(a[x]) || 0;
        while (cursor < candidates.length && candidates[cursor] <= last)
          cursor++;
        if (cursor < candidates.length) {
          last = candidates[cursor++];
          pairs.push([x, last]);
        }
        next.set(a[x], cursor);
      }
    }
  }
  return pairs.concat(suffix.reverse());
}

// Undo recorded edits that are still in place, newest first. Lists keep items
// other plugins have since added, at their positions relative to ours.
function revert(edits) {
  const restored = new Set();
  for (let index = edits.length - 1; index >= 0; index--) {
    const edit = edits[index];
    const { parent, field } = edit;
    if (!edit.list) {
      if (parent[field] === edit.after) {
        parent[field] = edit.before;
        if (isNode(edit.before)) restored.add(edit.before);
      }
      continue;
    }
    const current = parent[field];
    if (!Array.isArray(current)) continue;
    const kept = new Set(edit.before);
    const ours = new Set(edit.after);
    const inserted = new Set(edit.after.filter((item) => !kept.has(item)));
    const present = new Set(current);
    // Only items this graft removed come back: an item the graft kept and a
    // later plugin removed (or moved) stays where that plugin left it.
    const removed = edit.before.filter(
      (item) => !present.has(item) && !ours.has(item),
    );
    const order = new Map(
      edit.before.map((item, position) => [item, position]),
    );
    const result = [];
    let next = 0;
    const flush = (limit) => {
      while (next < removed.length && order.get(removed[next]) < limit) {
        result.push(removed[next]);
        if (isNode(removed[next])) restored.add(removed[next]);
        next++;
      }
    };
    for (const item of current) {
      if (inserted.has(item)) continue;
      if (order.has(item)) flush(order.get(item));
      result.push(item);
    }
    flush(Infinity);
    current.splice(0, current.length, ...result);
  }
  return restored;
}

// ---------------------------------------------------------------------------
// Locations of grafted nodes, traced to the Babel input file.

function lineStarts(source) {
  const starts = [0];
  const breaks = /\r\n|\n|\r|\u2028|\u2029/g;
  for (let match; (match = breaks.exec(source)); )
    starts.push(match.index + match[0].length);
  return starts;
}

function locator(nativeMap, priorMap, source) {
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
  return function remap(node) {
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
    node.loc = { start, end, identifierName: node.loc.identifierName };
    node.start = start.index;
    node.end = end.index;
  };
}

const COMMENT_FIELDS = ['leadingComments', 'innerComments', 'trailingComments'];

// Remap grafted subtrees and drop their copies of comments that unchanged
// nodes still carry (the compiler reprints comments next to its edits).
// `visit` sees every node of the result. Returns every attached comment, for
// `file.ast.comments`.
function settle(program, adopted, remap, visit, previousOf) {
  const kept = new Set();
  const keptStarts = new Set();
  const walk = (node) => {
    if (adopted.has(node)) return;
    if (visit != null) visit(node);
    for (const field of COMMENT_FIELDS) {
      const comments = node[field];
      if (comments != null)
        for (const comment of comments) {
          kept.add(comment);
          if (comment.start != null) keptStarts.add(comment.start);
        }
    }
    for (const field of fieldsOf(node)) {
      const value = node[field];
      if (Array.isArray(value)) {
        for (const item of value) if (isNode(item)) walk(item);
      } else if (isNode(value)) walk(value);
    }
  };
  walk(program);
  const all = new Set(kept);
  for (const root of adopted) {
    // A replacement spans the node it replaced: the compiler's map anchors
    // tokens, so a traced end can collapse onto the start.
    const previous = previousOf.get(root);
    const spans = previous?.loc != null && previous.start != null;
    if (spans) {
      root.loc = {
        start: previous.loc.start,
        end: previous.loc.end,
        identifierName: root.loc?.identifierName,
      };
      root.start = previous.start;
      root.end = previous.end;
    }
    t.traverseFast(root, (node) => {
      if (visit != null) visit(node);
      if (!spans || node !== root) remap(node);
      for (const field of COMMENT_FIELDS) {
        if (node[field] == null) continue;
        const comments = [];
        for (const comment of node[field]) {
          if (!all.has(comment) && !kept.has(comment)) {
            remap(comment);
            if (comment.start != null && keptStarts.has(comment.start))
              continue;
            all.add(comment);
          }
          if (all.has(comment)) comments.push(comment);
        }
        if (comments.length > 0) node[field] = comments;
        else delete node[field];
      }
    });
  }
  return [...all].sort((x, y) => (x.start ?? Infinity) - (y.start ?? Infinity));
}

// ---------------------------------------------------------------------------
// Compilation.

// The diagnostics `compile` raised, so callers recognize them without reading
// properties of values user code threw.
const compileDiagnostics = new WeakSet();

function compile(input, state, filename) {
  // Babel wraps an exception escaping this plugin exactly once, so the
  // compiler rethrows user exceptions unwrapped here and says which they are.
  const errors = { userThrew: false };
  try {
    return compiler.transformSync(input.code, {
      ...state.opts,
      filename,
      __sourceFileName: state.file.opts.filename || '',
      ...(state.file.opts.sourceType == null
        ? {}
        : { __sourceType: state.file.opts.sourceType }),
      sourceMaps: true,
      __errorsHandledByBabel: errors,
    });
  } catch (error) {
    // A user exception reaches Babel untouched: the same value, uninspected.
    if (errors.userThrew) throw error;
    const diagnostic = diagnosticFromError(error);
    if (diagnostic != null) {
      const wrapped = diagnosticError(diagnostic, input, state);
      wrapped.stylexDiagnostic = diagnostic;
      compileDiagnostics.add(wrapped);
      throw wrapped;
    }
    throw error;
  }
}

function generated(program, state, filename) {
  // `retainLines` keeps each node on its input line, which the compiler's
  // debug data (`file:line`) reports.
  const output = generate(
    program,
    {
      comments: true,
      retainLines: true,
      sourceMaps: true,
      sourceFileName: filename,
    },
    state.file.code,
  );
  return { code: output.code, map: output.map };
}

// Applies a compiler result to the program. Returns the grafted nodes.
function apply(
  program,
  state,
  input,
  result,
  record,
  defer = null,
  hold = null,
) {
  if (result.map == null)
    throw new Error(
      'The native StyleX compiler did not provide a source map for changed code',
    );
  // The compiler's output keeps its input's syntax: the file's own grammar for
  // the file's text, Babel's for its printing.
  // The entry compile runs on the program `mayUseStyleX` just walked, whose
  // syntax it collected; later compiles see a changed program.
  const parsed = babelParser.parse(
    result.code,
    grammarOptions(state, record ? state.stylexFacade.syntax : null),
  );
  if (parsed == null)
    throw new Error(
      'The native StyleX compiler produced no JavaScript program',
    );
  const graft = graftProgram(program.node, parsed.program, {
    record,
    defer,
    hold,
  });
  const remap = locator(result.map, input.map, state.file.code);
  const s = state.stylexFacade;
  s.names = stylexNames(program.node, s.sources);
  // With StyleX alone in its pass, every StyleX node the traversal meets is
  // one the program holds now or StyleX itself places: nothing arrives late.
  const everything = record && s.alone;
  const known = everything ? EVERY_NODE : new WeakSet();
  state.file.ast.comments = settle(
    program.node,
    graft.adopted,
    remap,
    everything
      ? null
      : (node) => {
          if (styleXNode(node, s)) known.add(node);
        },
    new Map(graft.replaced.map(([previous, node]) => [node, previous])),
  );
  dropCommentCopies(program.node.body, graft.adopted);
  // Recompiled output: what the compile removed hands its comments to its
  // siblings, as Babel's `path.remove()` does. (Entry output keeps unused
  // definitions in place until exit, and removes them there the same way.)
  if (!record && graft.removed.length > 0) {
    const all = new Set(state.file.ast.comments);
    for (const { item, previous, next } of graft.removed) {
      shareComments(item, previous, next);
      for (const field of COMMENT_FIELDS)
        for (const comment of item[field] || []) all.add(comment);
    }
    state.file.ast.comments = [...all].sort(
      (x, y) => (x.start ?? Infinity) - (y.start ?? Infinity),
    );
  }
  s.known = known;
  // Scope information is rebuilt only where the tree changed: definitions
  // waiting for their visit leave it as it was.
  if (graft.changed) program.scope.crawl();
  return graft;
}

function styleXNode(node, s) {
  return (
    (node.type === 'CallExpression' && stylexCall(node, s.names) != null) ||
    sxAttribute(node, s.sx)
  );
}

// Every StyleX call (and sx element) the program now contains: calls the
// compiler deliberately left for the runtime are not new work.
// The known set of a traversal no other plugin takes part in.
const EVERY_NODE = Object.freeze({ has: () => true, add() {} });

function knownStyleX(program, s) {
  const known = new WeakSet();
  if (s.names.empty && s.sx == null) return known;
  t.traverseFast(program, (node) => {
    if (styleXNode(node, s)) known.add(node);
  });
  return known;
}

function enter(program, state) {
  state.file.metadata.stylex = [];
  const s = {
    sources: importSources(state.opts),
    sx: sxPropName(state.opts),
    names: null,
    known: new WeakSet(),
    edits: [],
    pending: [],
    pendingNodes: new WeakSet(),
    seenSx: new WeakSet(),
    syntax: null,
    sites: [],
    holds: new Map(),
    // Unused definitions' calls, with what they read as compiled.
    unusedForms: new Map(),
    // Every definition's recorded input, and records of the declarations
    // they read (shared).
    inputs: [],
    dependencyShapes: new Map(),
    // Compiled replacements of other StyleX (props, attrs), and the calls.
    consumers: [],
    // Those compiled from a definition found stale, with that definition.
    staleConsumers: null,
    // Program-level references, to their declarations (with records).
    references: null,
    sxElements: null,
    // The consumers as written, every node the entry graft adopted, and the
    // compiled replacement of each replaced source.
    consumerCalls: null,
    adopted: null,
    outputs: null,
    // Entry output undone when the compile became stale.
    restored: new Set(),
    checkedProgram: false,
    program: null,
    // A provisionally compiled definition changed before its visit.
    stale: false,
    // Whether definitions are recorded to detect such changes (only another
    // plugin of the pass can make them).
    recordShapes: false,
    // StyleX is the only plugin of its pass that sees the program.
    alone: false,
    // The list each unused definition stays in until exit.
    unusedLists: null,
    // The program is Babel's untouched parse of the file's text.
    pristine: false,
    hoisted: [],
    unused: [],
    untouched: new WeakSet(),
    deferred: null,
    dirty: false,
    rules: [],
    warnings: [],
    errors: [],
  };
  state.stylexFacade = s;
  s.names = stylexNames(program.node, s.sources);
  // No compile for unrelated files. Their original locations and syntax-plugin
  // AST stay intact for subsequent Babel passes.
  // Another plugin of this pass may edit definitions, or what they read,
  // before StyleX visits them: then the entry compile stays provisional until
  // each definition's visit confirms its input.
  s.recordShapes = othersMayEditDefinitions(state);
  s.alone = aloneInPass(state);
  const declarations = s.recordShapes ? new Map() : null;
  const syntax = { jsx: false, flow: false, typescript: false };
  // With records, the consumers (props-like calls and sx elements) as written,
  // to tell which compiled output read a definition found stale later.
  const consumerCalls = s.recordShapes ? [] : null;
  // sx elements as written: the entry graft keeps them (renaming the
  // attribute), and an exit recompile that undoes it finds them as written.
  const sxElements = [];
  s.sxElements = sxElements;
  // Babel's untouched parse of the file's text (see below) holds an sx
  // attribute only where the text spells its name.
  const overrides = parserOverrides(state);
  s.pristine = overrides.length === 0 && parsedAndUntouched(state, program);
  const sxPossible =
    s.sx != null && !(s.pristine && !state.file.code.includes(s.sx));
  const visit =
    consumerCalls == null
      ? !sxPossible
        ? null
        : (node) => {
            if (sxAttribute(node, s.sx)) sxElements.push(node);
          }
      : (node) => {
          if (sxAttribute(node, s.sx)) sxElements.push(node);
          const consumer = (call) =>
            call?.type === 'CallExpression' &&
            EXIT_CALLS.has(stylexCall(call, s.names));
          // A spread of props is compiled to attributes replacing the spread.
          if (
            consumer(node) ||
            (node.type === 'JSXSpreadAttribute' && consumer(node.argument))
          )
            consumerCalls.push(node);
          else if (sxAttribute(node, s.sx))
            for (const attribute of node.attributes)
              if (
                attribute.type === 'JSXAttribute' &&
                attribute.name.type === 'JSXIdentifier' &&
                attribute.name.name === s.sx
              )
                consumerCalls.push(attribute);
        };
  // A program Babel parsed from the file's text, untouched since, holds no
  // StyleX import, require or sx attribute when the text cannot spell one:
  // the walk below could only answer no.
  if (s.pristine && s.names.empty && cannotSpellStyleX(state.file.code, s))
    return;
  // A program Babel parsed from the file's text, untouched since, needs no walk
  // when it imports StyleX at the top and nothing else is collected: its
  // grammar is the file's own parser options, which hold every syntax it has.
  const walkFree =
    s.pristine &&
    !s.names.empty &&
    visit == null &&
    declarations == null &&
    !impliesJsx(state.file.opts.parserOpts);
  if (
    !walkFree &&
    !mayUseStyleX(program.node, state.opts, syntax, declarations, visit)
  )
    return;
  s.consumerCalls = consumerCalls;
  // The program-level binding each reference resolves to, from Babel's scope
  // of the program as StyleX's traversal started (by identity: shadowing and
  // nested scopes are Babel's own).
  if (s.recordShapes) {
    s.references = new Map();
    for (const binding of Object.values(program.scope.bindings))
      for (const reference of binding.referencePaths)
        s.references.set(reference.node, binding.path.node);
    // Each consumer as written (the compile replaces parts of it in place):
    // its nodes, and the program-level declarations it references.
    s.consumerCalls = consumerCalls.map((call) => {
      const nodes = [];
      const declarations = new Set();
      t.traverseFast(call, (node) => {
        nodes.push(node);
        const declaration = s.references.get(node);
        if (declaration != null) declarations.add(declaration);
      });
      return { call, nodes, declarations };
    });
  }
  s.syntax = syntax;
  s.program = program;
  const filename = state.file.opts.filename || 'unknown.js';
  // Compile the file's own text when the AST still is exactly that text (its
  // positions are the user's); otherwise compile Babel's printing of the
  // current AST and compose the maps. A parser override (Hermes) produced an
  // AST in Babel's grammar from syntax the compiler need not read: compile its
  // printing too, so the override runs only for Babel's own parse of the file.
  let unchanged = false;
  if (overrides.length === 0) {
    if (s.pristine) unchanged = true;
    else {
      let original = null;
      try {
        original = babelParser.parse(
          state.file.code,
          state.file.opts.parserOpts,
        ).program;
      } catch (_) {
        original = null;
      }
      unchanged = original != null && equalTrees(original, program.node, true);
      // Free the reference tree before the compile and the output parse.
      original = null;
    }
  }
  const input = unchanged
    ? { code: state.file.code, map: null }
    : generated(program.node, state, filename);
  let result;
  try {
    result = compile(input, state, filename);
  } catch (error) {
    if (
      !compileDiagnostics.has(error) ||
      error.stylexDiagnostic.kind === 'parse'
    )
      throw error;
    // The original plugin compiles each definition when it visits it, after
    // earlier plugins' visitors (macros) ran there; a definition may depend on
    // values they produce. Report the error on exit unless the traversal
    // changed what this compile saw.
    s.deferred = { error, program: t.cloneNode(program.node, true, true) };
    return;
  }
  s.rules = metadataRules(result.metadata?.stylex || []);
  state.file.metadata.stylex = s.rules;
  s.warnings = result.warnings || [];
  s.errors = result.errors || [];
  for (const warning of s.warnings) logDiagnostic('warn', warning);
  for (const error of s.errors) logDiagnostic('error', error);
  // Even a relevant import can be type-only, unused or contain no StyleX DSL.
  // Never touch a no-op AST merely to refresh its printed source.
  if (result.code !== input.code) {
    // The original plugin compiles a definition only if the traversal still
    // reaches it, so the entry compile is provisional where it consumed one:
    // a later or earlier plugin may detach it (dead-branch elimination), and
    // then neither its output, its hoists nor its rules may remain.
    const names = s.names;
    // Whether a subtree contains a definition call: the nodes `traverseFast`
    // visits, stopping at the first one (usually the subtree's own init).
    const defines = (node) => {
      if (node == null || typeof node.type !== 'string') return false;
      if (node.type === 'CallExpression') {
        const api = stylexCall(node, names);
        if (api != null && !EXIT_CALLS.has(api)) return true;
      }
      for (const key of t.VISITOR_KEYS[node.type] || []) {
        const value = node[key];
        if (Array.isArray(value)) {
          for (const child of value) if (defines(child)) return true;
        } else if (defines(value)) return true;
      }
      return false;
    };
    // Definitions the compile removed as unused stay in place, untouched by
    // later plugins, until exit: the original plugin removes them there.
    // A definition keeps its source form, and the references in it, until
    // the traversal visits it: plugins that run first (TypeScript import
    // elision) see what they see with the original plugin. Its compiled form
    // replaces it then.
    const definition = (node) => {
      if (node.type !== 'CallExpression') return false;
      const api = stylexCall(node, names);
      return api != null && !EXIT_CALLS.has(api);
    };
    const graft = apply(
      program,
      state,
      input,
      result,
      true,
      defines,
      definition,
    );
    s.edits = graft.edits;
    if (s.recordShapes) {
      s.adopted = graft.adopted;
      s.outputs = [
        ...graft.replaced.map(([previous, node]) => ({
          removed: [previous],
          added: [node],
        })),
        ...graft.regions,
      ];
    }
    const held = new Map(graft.held);
    // Declarations the compile hoisted to the program for a held definition
    // arrive with it, as the original plugin hoists when it visits one.
    const hoistsFor = new Map();
    const owners = new Map();
    for (const [previous, node] of graft.held)
      t.traverseFast(node, (inner) => {
        if (inner.type === 'Identifier') owners.set(inner.name, previous);
      });
    const replacements = new Set(graft.replaced.map(([, node]) => node));
    const ownerOf = (statement) => {
      if (
        statement.type !== 'VariableDeclaration' ||
        !graft.adopted.has(statement) ||
        replacements.has(statement)
      )
        return null;
      const names = Object.keys(t.getBindingIdentifiers(statement));
      const owner = names.length > 0 ? owners.get(names[0]) : null;
      return owner != null && names.every((name) => owners.get(name) === owner)
        ? owner
        : null;
    };
    const body = program.node.body;
    const owned = body.map(ownerOf);
    for (const [index, owner] of owned.entries()) {
      if (owner == null) continue;
      // It returns where the compiler put it: before the next statement.
      let next = index + 1;
      while (next < body.length && owned[next] != null) next++;
      if (!hoistsFor.has(owner)) hoistsFor.set(owner, []);
      hoistsFor.get(owner).push({ node: body[index], before: body[next] });
    }
    body.splice(
      0,
      body.length,
      ...body.filter((_, index) => owned[index] == null),
    );

    // What each definition read, recorded so its visit can confirm it. Other
    // definitions among what it read are checked at their own visits (and
    // replaced there by StyleX itself).
    const definitionCalls = new Set();
    if (s.recordShapes) {
      for (const [previous] of graft.held) definitionCalls.add(previous);
      for (const node of graft.deferred)
        t.traverseFast(node, (inner) => {
          if (definition(inner)) definitionCalls.add(inner);
        });
    }
    const inputs = (call) => {
      if (!s.recordShapes) return null;
      const record = { call, shape: shapeRecord(call), dependencies: [] };
      for (const declaration of dependencies(call, declarations)) {
        let shape = s.dependencyShapes.get(declaration);
        if (shape == null) {
          shape = shapeRecord(declaration, [], definitionCalls);
          s.dependencyShapes.set(declaration, shape);
        }
        record.dependencies.push(declaration);
      }
      s.inputs.push(record);
      return record;
    };
    for (const [previous, node] of graft.replaced)
      if (held.has(previous)) {
        s.sites.push({ node, previous, held: previous });
        s.holds.set(previous, {
          replacement: node,
          hoists: hoistsFor.get(previous) || [],
          callee: previous.callee,
          args: previous.arguments.slice(),
          inputs: inputs(previous),
        });
      } else if (defines(previous)) s.sites.push({ node, previous });
      else if (s.recordShapes) s.consumers.push({ node, previous });
    s.unusedLists = graft.deferredIn;
    for (const node of graft.deferred) {
      s.sites.push({ node });
      // A declaration is checked by its initializers' identity; anything
      // else by structure, against a copy.
      const declaration = node.type === 'VariableDeclaration';
      s.unused.push({
        node,
        copy: declaration ? null : t.cloneNode(node, true, true),
        inits: declaration
          ? node.declarations.map((declarator) => declarator.init)
          : null,
      });
      t.traverseFast(node, (inner) => {
        if (inner.type !== 'CallExpression') return;
        s.untouched.add(inner);
        if (s.recordShapes && definition(inner))
          s.unusedForms.set(inner, inputs(inner));
      });
    }
  } else {
    s.names = stylexNames(program.node, s.sources);
    if (s.alone) {
      s.known = EVERY_NODE;
      s.unusedLists = new Map();
    } else s.known = knownStyleX(program.node, s);
  }
}

function unchangedInputs(record, s) {
  return (
    matchesShapeRecord(record.call, record.shape) &&
    record.dependencies.every((declaration) =>
      matchesShapeRecord(declaration, s.dependencyShapes.get(declaration)),
    )
  );
}

// The entry compile no longer describes what the traversal reaches. Undo its
// output now, before other plugins visit and transform it, and compile the
// program at exit, as the deferred and dead-branch cases do.
function markStale(s) {
  if (s.stale) return;
  s.stale = true;
  s.staleConsumers = staleConsumers(s);
  for (const node of revert(s.edits)) s.restored.add(node);
  s.edits = [];
}

// The compiled StyleX calls (props, attrs) that read a definition whose input
// changed: they reference, by name, a binding of a changed definition or of
// one reading it. Rare path: computed only when a compile turns stale.
function staleConsumers(s) {
  if ((s.consumerCalls || []).length === 0 || s.references == null) return [];
  // The declarator each recorded definition initializes.
  const calls = new Map(s.inputs.map((record) => [record.call, record]));
  const declaratorOf = new Map();
  t.traverseFast(s.program.node, (node) => {
    if (node.type !== 'VariableDeclarator' || node.init == null) return;
    t.traverseFast(node.init, (inner) => {
      if (calls.has(inner)) declaratorOf.set(calls.get(inner), node);
    });
  });
  // The program-level declarations a subtree references, by Babel's scope.
  const referenced = (root) => {
    const found = new Set();
    t.traverseFast(root, (node) => {
      const declaration = s.references.get(node);
      if (declaration != null) found.add(declaration);
    });
    return found;
  };
  // Changed definitions (or a declaration a definition read that changed),
  // then every definition referencing an affected declaration.
  const affected = new Map();
  for (const record of s.inputs) {
    if (unchangedInputs(record, s)) continue;
    const declarator = declaratorOf.get(record);
    if (!matchesShapeRecord(record.call, record.shape) && declarator != null)
      affected.set(declarator, record);
    for (const dependency of record.dependencies)
      if (!matchesShapeRecord(dependency, s.dependencyShapes.get(dependency)))
        affected.set(dependency, record);
  }
  for (let grown = true; grown; ) {
    grown = false;
    for (const record of s.inputs) {
      const declarator = declaratorOf.get(record);
      if (declarator == null || affected.has(declarator)) continue;
      for (const declaration of referenced(record.call))
        if (affected.has(declaration)) {
          affected.set(declarator, affected.get(declaration));
          grown = true;
          break;
        }
    }
  }
  const result = [];
  for (const { call, nodes, declarations } of s.consumerCalls) {
    if (call.start == null) continue;
    for (const declaration of declarations)
      if (affected.has(declaration)) {
        result.push({ call, nodes, definition: affected.get(declaration) });
        break;
      }
  }
  return result;
}

function locationOf(node) {
  const start = node?.loc?.start;
  return start == null
    ? 'an unknown location'
    : `${start.line}:${start.column + 1}`;
}

// A compiled consumer of a stale definition that other plugins already
// transformed cannot be undone or recompiled: its stale class names would
// reach the output without CSS. Fail the build instead.
// The entry graft's output for a stale consumer: what replaced the source
// node or list run containing it.
function assertNoStaleConsumers(program, state, s) {
  const stale = s.staleConsumers || [];
  if (stale.length === 0 || s.outputs == null) return;
  // Every node of a stale consumer as written: what the compile replaced at
  // or inside it is its output (an sx attribute keeps its node and gets new
  // children).
  const consumerOf = new Map();
  for (const entry of stale)
    for (const node of entry.nodes)
      if (!consumerOf.has(node)) consumerOf.set(node, entry);
  const ownerIn = (items) => {
    for (const item of items) {
      if (!isNode(item)) continue;
      let found = null;
      t.traverseFast(item, (node) => {
        found ??= consumerOf.get(node) ?? null;
      });
      if (found != null) return found;
    }
    return null;
  };
  const owners = new Map();
  const own = (items, owner) => {
    for (const item of items)
      if (isNode(item))
        t.traverseFast(item, (node) => {
          owners.set(node, owner);
        });
  };
  for (const { removed, added } of s.outputs) {
    const owner = ownerIn(removed);
    if (owner != null) own(added, owner);
  }
  if (owners.size === 0) return;
  let leaked = null;
  t.traverseFast(program.node, (node) => {
    if (leaked == null && owners.has(node)) leaked = owners.get(node);
  });
  if (leaked == null) return;
  const { call, definition } = leaked;
  throw state.file.buildCodeFrameError(
    call,
    `A plugin rewrote the StyleX definition at ${locationOf(
      definition.call,
    )} (or a value it reads) during the traversal, after its consumer at ${locationOf(
      call,
    )} was compiled and transformed by other plugins, so the consumer would keep class names without CSS. Move the definition above its consumers, or run that plugin in an earlier pass.`,
    Error,
  );
}

// Plugins later in the pass edit the program in their Program.enter, after
// StyleX's: check every recorded input once, at the first node visited.
function checkProgram(s) {
  if (s.checkedProgram || !s.recordShapes) return;
  s.checkedProgram = true;
  if (s.inputs.some((record) => !unchangedInputs(record, s))) markStale(s);
}

// Program statements the graft adopted whole (hoists) may carry copies of
// comments that the kept statements around them still own: the compiler
// reprints a neighbour's comments next to what it inserts. Positions do not
// always trace onto the original, so copies are also recognized by text.
function dropCommentCopies(body, adopted) {
  for (let index = 0; index < body.length; index++) {
    const statement = body[index];
    if (!adopted.has(statement)) continue;
    let previous = index - 1;
    while (previous >= 0 && adopted.has(body[previous])) previous--;
    let next = index + 1;
    while (next < body.length && adopted.has(body[next])) next++;
    const owned = [];
    for (const neighbour of [body[previous], body[next]])
      if (neighbour != null)
        for (const field of COMMENT_FIELDS)
          for (const comment of neighbour[field] || []) owned.push(comment);
    if (owned.length === 0) continue;
    for (const field of ['leadingComments', 'trailingComments']) {
      const comments = statement[field];
      if (comments == null) continue;
      const left = comments.filter(
        (comment) =>
          owned.includes(comment) ||
          !owned.some(
            (other) =>
              other.type === comment.type && other.value === comment.value,
          ),
      );
      if (left.length > 0) statement[field] = left;
      else delete statement[field];
    }
  }
}

// A StyleX call that earlier plugins created after entry. The original plugin
// compiles definitions (`create`, `keyframes`, ...) when it visits them: no
// plugin visits their original arguments, later plugins visit only the
// compiled replacement. Keep the definition as visited and leave its subtree
// to the exit compile, whose output is queued for every plugin.
function created(path, state) {
  const s = state.stylexFacade;
  if (s == null || s.names.empty) return;
  const node = path.node;
  const hold = s.holds.get(node);
  if (hold != null) {
    s.holds.delete(node);
    // An earlier plugin's visitor that rewrote the call here makes it a
    // different definition: the exit compile handles it.
    // An earlier plugin's visitor may also have changed the definition in
    // place, deep inside its arguments: the entry compile then read another
    // definition than the one the traversal reached.
    checkProgram(s);
    const same =
      !s.stale &&
      node.callee === hold.callee &&
      node.arguments.length === hold.args.length &&
      node.arguments.every(
        (argument, index) => argument === hold.args[index],
      ) &&
      (hold.inputs == null || unchangedInputs(hold.inputs, s));
    if (!same) {
      // Compiled at exit from the program the traversal leaves, as visited
      // here (later plugins see the compiled form, as with the original).
      markStale(s);
      s.dirty = true;
      s.pending.push({ path, node, copy: t.cloneNode(node, true) });
      s.pendingNodes.add(node);
      path.skip();
      return;
    }
    const inserted = [];
    for (const { node: hoist, before } of hold.hoists) {
      const program = path.scope.getProgramParent().path;
      const index = before == null ? -1 : program.node.body.indexOf(before);
      const anchor =
        index >= 0
          ? NodePath.get({
              parentPath: program,
              parent: program.node,
              container: program.node.body,
              listKey: 'body',
              key: index,
            }).setContext(program.context)
          : path.find((ancestor) => ancestor.parentPath?.isProgram());
      if (anchor == null) {
        s.dirty = true;
        return;
      }
      inserted.push(...anchor.insertBefore(hoist));
      s.hoisted.push(hoist);
    }
    const [replaced] = path.replaceWith(hold.replacement);
    if (inserted.length > 0) registerHoists(inserted, replaced);
    return;
  }
  if (s.untouched.has(node)) {
    checkProgram(s);
    const recorded = s.unusedForms.get(node);
    if (recorded != null) {
      s.unusedForms.delete(node);
      if (!unchangedInputs(recorded, s)) markStale(s);
    }
    path.skip();
    return;
  }
  if (s.known.has(node)) return;
  const api = stylexCall(node, s.names);
  if (api == null || EXIT_CALLS.has(api)) return;
  if (path.findParent((parent) => s.pendingNodes.has(parent.node)) != null)
    return;
  s.pending.push({ path, node, copy: t.cloneNode(node, true) });
  s.pendingNodes.add(node);
  path.skip();
}

// Hoists inserted with a definition at its visit are declared at the program
// and referenced by the replacement (and by each other). Babel's insertion
// does not count those references, and no whole-program crawl follows, so
// register them: plugins that remove unreferenced bindings must keep them.
function registerHoists(hoists, replaced) {
  const scope = replaced.scope.getProgramParent();
  const names = new Set();
  for (const hoist of hoists) {
    const declared = Object.keys(hoist.getBindingIdentifiers());
    if (!declared.some((name) => scope.hasOwnBinding(name)))
      scope.registerDeclaration(hoist);
    for (const name of declared) names.add(name);
  }
  const reference = (path) => {
    const name = path.node.name;
    if (!names.has(name)) return;
    const binding = scope.getOwnBinding(name);
    if (binding != null && path.scope.getBinding(name) === binding)
      binding.reference(path);
  };
  const visitor = { ReferencedIdentifier: reference };
  for (const root of [replaced, ...hoists]) {
    if (root.isReferencedIdentifier()) reference(root);
    root.traverse(visitor);
  }
}

// StyleX the program gained after entry, classified as the original plugin
// would treat it on exit: `props`-like calls and sx elements it visited are
// compiled; definitions and sx elements that appeared after its visit (from
// later plugins or on exit) are never compiled by it and stay as they are.
function lateStyleX(program, s, skip) {
  const found = { compile: false, hidden: [], positions: new Map() };
  const sites = new Set(s.sites.map((site) => site.node));
  for (const consumer of s.consumers) sites.add(consumer.node);
  for (const site of s.sites) if (site.held != null) sites.add(site.held);
  const walk = (node, parent, field, index) => {
    if (sites.has(node)) found.positions.set(node, { parent, field, index });
    if (skip.has(node) || s.known.has(node)) return;
    if (node.type === 'CallExpression') {
      const api = stylexCall(node, s.names);
      if (api != null) {
        if (!EXIT_CALLS.has(api)) {
          found.hidden.push({ node, parent, field, index });
          return;
        }
        found.compile = true;
      }
    } else if (sxAttribute(node, s.sx)) {
      if (s.seenSx.has(node)) found.compile = true;
      else found.hidden.push({ node, sx: true });
    }
    for (const key of fieldsOf(node)) {
      const value = node[key];
      if (Array.isArray(value)) {
        value.forEach((item, position) => {
          if (isNode(item)) walk(item, node, key, position);
        });
      } else if (isNode(value)) walk(value, node, key, null);
    }
  };
  walk(program, null, null, null);
  // A detached definition site, or one the traversal never visited: rebuild
  // from the definitions that survive, as the original plugin compiled them.
  for (const site of s.sites) {
    const applied = found.positions.has(site.node);
    const waiting = site.held != null && found.positions.has(site.held);
    if (!applied && !waiting) found.compile = true;
    if (waiting && s.holds.has(site.held)) found.compile = true;
  }
  return found;
}

// Keep constructs the exit compile must not touch out of its sight, behind
// placeholders the compiler treats as opaque values; `restore` puts them back.
function hide(hidden) {
  const placeholders = new Map();
  for (const [position, entry] of hidden.entries()) {
    const name = `__stylexExitPlaceholder${position}`;
    placeholders.set(name, entry);
    const placeholder = t.identifier(name);
    if (entry.index == null) entry.parent[entry.field] = placeholder;
    else entry.parent[entry.field][entry.index] = placeholder;
  }
  return placeholders;
}

function restore(program, placeholders) {
  if (placeholders.size === 0) return;
  const walk = (node) => {
    for (const key of fieldsOf(node)) {
      const value = node[key];
      if (Array.isArray(value)) {
        value.forEach((item, position) => {
          if (item?.type === 'Identifier' && placeholders.has(item.name))
            value[position] = placeholders.get(item.name).node;
          else if (isNode(item)) walk(item);
        });
      } else if (value?.type === 'Identifier' && placeholders.has(value.name))
        node[key] = placeholders.get(value.name).node;
      else if (isNode(value)) walk(value);
    }
  };
  walk(program);
}

// Removes the unused definitions the entry compile dropped, wherever the
// traversal left them, as the original plugin removes unused style variables
// on exit.
function removeUnused(program, unused, positions) {
  const byList = new Map();
  let nested = false;
  for (const { node } of unused) {
    const position = positions.get(node);
    if (position == null || position.index == null) continue;
    const list = position.parent[position.field];
    if (!byList.has(list)) byList.set(list, new Set());
    byList.get(list).add(node);
    if (position.parent === program.node) {
      for (const name of Object.keys(t.getBindingIdentifiers(node)))
        program.scope.removeBinding(name);
    } else nested = true;
  }
  for (const [list, nodes] of byList) {
    shareRemovedComments(list, nodes);
    list.splice(0, list.length, ...list.filter((item) => !nodes.has(item)));
  }
  if (nested) program.scope.crawl();
}

// Removed statements hand their comments to their siblings, as Babel's
// `path.remove()` does (one removal after another, in list order).
function shareRemovedComments(list, nodes) {
  for (let index = 0; index < list.length; index++) {
    const node = list[index];
    if (!nodes.has(node)) continue;
    const leading = node.leadingComments;
    const trailing = node.trailingComments;
    if (leading == null && trailing == null) continue;
    let back = index - 1;
    while (back >= 0 && nodes.has(list[back])) back--;
    shareComments(
      node,
      isNode(list[back]) ? list[back] : null,
      isNode(list[index + 1]) ? list[index + 1] : null,
    );
  }
}

// Babel's `shareCommentsWithSiblings`, for a node leaving between two others.
function shareComments(node, previous, next) {
  const without = (comments, existing) =>
    existing == null
      ? comments
      : comments.filter((comment) => !existing.includes(comment));
  const leading = node.leadingComments;
  const trailing = node.trailingComments;
  if (previous != null) {
    if (leading != null)
      t.addComments(
        previous,
        'trailing',
        without(leading, previous.trailingComments),
      );
    if (trailing != null && next == null)
      t.addComments(previous, 'trailing', trailing);
  }
  if (next != null) {
    if (trailing != null)
      t.addComments(next, 'leading', without(trailing, next.leadingComments));
    if (leading != null && previous == null)
      t.addComments(next, 'leading', leading);
  }
}

function exit(program, state) {
  const s = state.stylexFacade;
  if (s == null) return;
  s.names = stylexNames(program.node, s.sources);
  // Unrelated files pay only this scan of top-level statements.
  if (
    s.names.empty &&
    s.pending.length === 0 &&
    !s.dirty &&
    s.deferred == null &&
    s.sites.length === 0
  )
    return;
  // An unused definition whose initializer another plugin replaced (coverage
  // instrumentation wraps it) is no longer the one the compile removed.
  // Rewrites around it (block scoping turning `const` into `var`) or inside
  // the definition call (module lowering renaming its callee) are not.
  // With StyleX alone in its pass nothing else ran: no StyleX arrived late, no
  // definition moved, and each unused one is still in the list it was left in.
  if (
    s.alone &&
    s.pending.length === 0 &&
    !s.dirty &&
    !s.stale &&
    s.deferred == null &&
    s.unusedLists != null
  ) {
    const positions = new Map();
    for (const { node } of s.unused) {
      const list = s.unusedLists.get(node);
      if (list != null) positions.set(node, { ...list, index: 0 });
    }
    removeUnused(program, s.unused, positions);
    return;
  }
  const late = lateStyleX(program.node, s, new Set());
  const changed = s.unused.some(
    ({ node, copy, inits }) =>
      late.positions.has(node) &&
      (inits != null
        ? node.declarations.length !== inits.length ||
          node.declarations.some(
            (declarator, index) => declarator.init !== inits[index],
          )
        : !equalTrees(copy, node, false, s.untouched)),
  );
  if (s.pending.length === 0 && !changed && !s.stale && !late.compile) {
    if (s.deferred != null) throw s.deferred.error;
    removeUnused(program, s.unused, late.positions);
    return;
  }
  // Earlier plugins added StyleX the original plugin compiles. Recompile once,
  // from the program as it would see it: StyleX's own entry output undone and
  // new definitions as they were when first visited.
  const restored = revert(s.edits);
  for (const node of s.restored) restored.add(node);
  // sx elements as written are restored by the revert in place (the graft
  // keeps them and renames their attribute): they are source, not late.
  for (const element of s.sxElements || []) restored.add(element);
  s.restored = new Set();
  s.edits = [];
  // A live definition another plugin moved or wrapped returns at its place;
  // so do StyleX calls compiled from a stale input, wherever they are now.
  const moved = lateStyleX(program.node, s, new Set()).positions;
  // A consumer's output returns to its call only where an expression stood:
  // another plugin may have moved its parts anywhere (a JSX attribute value
  // into an object), where the call cannot go.
  const returning = s.stale
    ? [
        ...s.sites,
        ...s.consumers.filter(
          ({ node, previous }) =>
            t.isExpression(node) && t.isExpression(previous),
        ),
      ]
    : s.sites;
  for (const site of returning) {
    const position = site.previous != null ? moved.get(site.node) : null;
    if (position == null || position.parent == null) continue;
    if (position.index == null) position.parent[position.field] = site.previous;
    else position.parent[position.field][position.index] = site.previous;
    restored.add(site.previous);
  }
  if (s.stale) assertNoStaleConsumers(program, state, s);
  s.consumers = [];
  s.sites = [];
  s.unused = [];
  // Hoists that arrived with applied definitions are compiler output too.
  if (s.hoisted.length > 0) {
    const hoisted = new Set(s.hoisted);
    const body = program.node.body;
    body.splice(
      0,
      body.length,
      ...body.filter((statement) => !hoisted.has(statement)),
    );
    s.hoisted = [];
  }
  for (const { path, node, copy } of s.pending) {
    if (path.removed || path.node !== node || path.container == null) continue;
    path.container[path.key] = copy;
    restored.add(copy);
  }
  s.pending = [];
  if (s.deferred != null) {
    if (equalTrees(s.deferred.program, program.node, false))
      throw s.deferred.error;
  }
  const { hidden } = lateStyleX(program.node, s, restored);
  // sx attributes are renamed rather than replaced.
  const sxNames = [];
  for (const entry of hidden)
    if (entry.sx) {
      const attribute = entry.node.attributes.find(
        (attr) =>
          attr.type === 'JSXAttribute' &&
          attr.name.type === 'JSXIdentifier' &&
          attr.name.name === s.sx,
      );
      sxNames.push(attribute.name);
      attribute.name.name = `data-stylex-exit-placeholder-${sxNames.length}`;
    }
  const placeholders = hide(hidden.filter((entry) => !entry.sx));
  const filename = state.file.opts.filename || 'unknown.js';
  const input = generated(program.node, state, filename);
  let result;
  try {
    result = compile(input, state, filename);
  } finally {
    for (const name of sxNames) name.name = s.sx;
  }
  // The entry compile was provisional: rules come from the definitions the
  // traversal left in the program.
  s.rules = metadataRules(result.metadata?.stylex || []);
  state.file.metadata.stylex = s.rules;
  for (const warning of multisetDifference(result.warnings || [], s.warnings))
    logDiagnostic('warn', warning);
  for (const error of multisetDifference(result.errors || [], s.errors))
    logDiagnostic('error', error);
  let fresh = restored;
  if (result.code !== input.code) {
    const { adopted } = apply(program, state, input, result, false);
    fresh = new Set([...restored, ...adopted]);
  }
  restore(program.node, placeholders);
  for (const name of sxNames) name.name = s.sx;
  program.scope.crawl();
  s.names = stylexNames(program.node, s.sources);
  s.known = knownStyleX(program.node, s);
  requeue(program, fresh);
}

// Nodes placed on exit have not been visited by the plugins of this pass:
// queue them on the file's traversal, as Babel does for inserted nodes.
function requeue(program, nodes) {
  if (nodes.size === 0) return;
  // Paths are created along the walk, one per visited node: resolving each
  // key path from the program would map whole sibling lists every time.
  const walk = (path) => {
    if (nodes.has(path.node)) {
      program.requeue(path);
      return;
    }
    for (const field of fieldsOf(path.node)) {
      const value = path.node[field];
      if (Array.isArray(value)) {
        value.forEach((item, index) => {
          if (isNode(item))
            walk(
              NodePath.get({
                parentPath: path,
                parent: path.node,
                container: value,
                listKey: field,
                key: index,
              }).setContext(path.context),
            );
        });
      } else if (isNode(value))
        walk(
          NodePath.get({
            parentPath: path,
            parent: path.node,
            container: path.node,
            key: field,
          }).setContext(path.context),
        );
    }
  };
  walk(program);
}

// An sx element an earlier plugin created after entry. The original plugin
// rewrites it when visited, before later plugins lower the JSX, into a spread
// of `props(...)` through the StyleX namespace in scope; the exit compile then
// compiles that call. Without a visible namespace it stays for the exit compile.
function sxElement(path, state) {
  const s = state.stylexFacade;
  if (s == null || s.known.has(path.node) || !sxAttribute(path.node, s.sx))
    return;
  s.seenSx.add(path.node);
  s.dirty = true;
  const program = path.scope.getProgramParent();
  const namespace = [...s.names.namespaces].find((name) => {
    const binding = program.getBinding(name);
    return binding != null && path.scope.getBinding(name) === binding;
  });
  if (namespace == null) return;
  for (const attribute of path.get('attributes')) {
    if (
      !attribute.isJSXAttribute() ||
      !attribute.get('name').isJSXIdentifier({ name: s.sx })
    )
      continue;
    const value = attribute.get('value');
    if (
      !value.isJSXExpressionContainer() ||
      value.get('expression').isJSXEmptyExpression()
    )
      continue;
    attribute.replaceWith(
      t.jsxSpreadAttribute(
        t.callExpression(
          t.memberExpression(t.identifier(namespace), t.identifier('props')),
          [value.node.expression],
        ),
      ),
    );
    break;
  }
}

function styleXTransform() {
  return {
    name: 'stylex-rust',
    visitor: {
      Program: { enter, exit },
      'Statement|Directive'(path, state) {
        const s = state.stylexFacade;
        if (s != null) checkProgram(s);
      },
      CallExpression: created,
      ImportDeclaration(path, state) {
        const s = state.stylexFacade;
        if (s == null || !s.sources.has(path.node.source.value)) return;
        // An earlier plugin imported StyleX during the traversal.
        s.names = stylexNames(path.parentPath.node, s.sources);
      },
      JSXOpeningElement: sxElement,
    },
  };
}

styleXTransform.withOptions = (options) => [styleXTransform, options];
styleXTransform.processStylexRules = compiler.processStylexRules;
module.exports = styleXTransform;
