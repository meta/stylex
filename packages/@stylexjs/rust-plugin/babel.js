/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// The Babel plugin: compiles each file with the native compiler and grafts the
// result onto Babel's AST.

const compiler = require('./index');
const { babelParser, generate, NodePath, t } = require('./plugin/babel-deps');
const {
  aloneInPass,
  COMMENT_FIELDS,
  equalTrees,
  fieldsOf,
  impliesJsx,
  isNode,
  othersMayEditDefinitions,
  parsedAndUntouched,
  shapeRecord,
} = require('./plugin/ast-equality');
const {
  diagnosticError,
  diagnosticFromError,
  logDiagnostic,
  metadataRules,
  multisetDifference,
} = require('./plugin/diagnostics');
const { graftProgram, revert } = require('./plugin/graft');
const { grammarOptions, parserOverrides } = require('./plugin/grammar');
const {
  dropCommentCopies,
  locator,
  settle,
  shareComments,
  shareRemovedComments,
} = require('./plugin/locations');
const {
  assertNoStaleConsumers,
  checkProgram,
  markStale,
  unchangedInputs,
} = require('./plugin/staleness');
const {
  cannotSpellStyleX,
  dependencies,
  importSources,
  mayUseStyleX,
  stylexCall,
  stylexNames,
  sxAttribute,
  sxPropName,
} = require('./plugin/stylex-usage');

// Calls the original plugin compiles on Program.exit rather than on visit.
const EXIT_CALLS = new Set(['props', 'attrs', 'merge', 'default']);

// Diagnostics `compile` raised, so callers needn't inspect user-thrown values.
const compileDiagnostics = new WeakSet();

// Babel validates the options of every file, so a file skipped for having no
// StyleX still reports a malformed option. The diagnostics depend only on the
// options, so they are computed once per options object.
const optionDiagnostics = new WeakMap();
const NO_OPTIONS = {};
function reportOptionDiagnostics(state) {
  const key =
    state.opts != null && typeof state.opts === 'object'
      ? state.opts
      : NO_OPTIONS;
  let errors = optionDiagnostics.get(key);
  if (errors == null) {
    try {
      errors =
        compiler.transformSync('', {
          ...state.opts,
          filename: state.file.opts.filename || 'unknown.js',
        }).errors || [];
    } catch (_) {
      // Options the compiler refuses outright stay as before for skipped files.
      errors = [];
    }
    optionDiagnostics.set(key, errors);
  }
  for (const error of errors) logDiagnostic('error', error);
}

function compile(input, state, filename) {
  // Babel wraps an escaping exception once, so user exceptions stay unwrapped.
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
  // `retainLines` keeps input lines for the debug data's `file:line`.
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
  // The output keeps its input's syntax; the entry compile knows it from its walk.
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
  // With StyleX alone in its pass, no StyleX node reaches the traversal late.
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
  // Removals share comments with siblings, like Babel's `path.remove()`.
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
  if (graft.changed) program.scope.crawl();
  return graft;
}

function styleXNode(node, s) {
  return (
    (node.type === 'CallExpression' && stylexCall(node, s.names) != null) ||
    sxAttribute(node, s.sx)
  );
}

// The known set when no other plugin takes part in the traversal.
const EVERY_NODE = Object.freeze({ has: () => true, add() {} });

// StyleX already in the program; calls left for the runtime are not new work.
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
    rewriteAliases: state.opts?.rewriteAliases === true,
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
    // Every definition's recorded input, with shared records of what they read.
    inputs: [],
    dependencyShapes: new Map(),
    // Compiled replacements of other StyleX (props, attrs), and the calls.
    consumers: [],
    // Those compiled from a definition found stale, with that definition.
    staleConsumers: null,
    // Program-level references, to their declarations (with records).
    references: null,
    sxElements: null,
    // Consumers as written, entry-adopted nodes, and replacements by source.
    consumerCalls: null,
    adopted: null,
    outputs: null,
    // Entry output undone when the compile became stale.
    restored: new Set(),
    checkedProgram: false,
    program: null,
    // A provisionally compiled definition changed before its visit.
    stale: false,
    // Whether definitions are recorded to detect such changes.
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
  // If others may edit definitions first, the entry compile stays provisional.
  s.recordShapes = othersMayEditDefinitions(state);
  s.alone = aloneInPass(state);
  const declarations = s.recordShapes ? new Map() : null;
  const syntax = { jsx: false, flow: false, typescript: false };
  // Consumers as written, to find output that read a definition found stale.
  const consumerCalls = s.recordShapes ? [] : null;
  // sx elements as written; the entry graft keeps them and renames the attribute.
  const sxElements = [];
  s.sxElements = sxElements;
  const overrides = parserOverrides(state);
  s.pristine = overrides.length === 0 && parsedAndUntouched(state, program);
  // An untouched parse holds an sx attribute only where the text spells its name.
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
  // An untouched parse cannot hold StyleX the text cannot spell: skip the walk.
  if (s.pristine && s.names.empty && cannotSpellStyleX(state.file.code, s)) {
    reportOptionDiagnostics(state);
    return;
  }
  // An untouched parse needs no walk if nothing beyond its imports is collected.
  const walkFree =
    s.pristine &&
    !s.names.empty &&
    visit == null &&
    declarations == null &&
    !impliesJsx(state.file.opts.parserOpts);
  if (
    !walkFree &&
    !mayUseStyleX(program.node, state.opts, syntax, declarations, visit)
  ) {
    reportOptionDiagnostics(state);
    return;
  }
  s.consumerCalls = consumerCalls;
  // The program-level binding of each reference, by Babel's scope at entry.
  if (s.recordShapes) {
    s.references = new Map();
    for (const binding of Object.values(program.scope.bindings))
      for (const reference of binding.referencePaths)
        s.references.set(reference.node, binding.path.node);
    // Each consumer's nodes and the program-level declarations it references.
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
  // Compile the file's text if the AST still matches it, else Babel's printing.
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
    // Report at exit unless visitors that run first change what this compile saw.
    s.deferred = { error, program: t.cloneNode(program.node, true, true) };
    return;
  }
  s.rules = metadataRules(result.metadata?.stylex || []);
  state.file.metadata.stylex = s.rules;
  s.warnings = result.warnings || [];
  s.errors = result.errors || [];
  for (const warning of s.warnings) logDiagnostic('warn', warning);
  for (const error of s.errors) logDiagnostic('error', error);
  // Never touch a no-op AST just to refresh its printed source.
  if (result.code !== input.code) {
    // Provisional: a plugin may still detach a definition this compile consumed.
    const names = s.names;
    // Whether a subtree contains a definition call (stops at the first one).
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
    // Definitions keep their source form until visited, and unused ones until exit.
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
    // Hoists for a held definition arrive with it, as the original plugin does.
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

    // What each definition read, recorded so its visit can confirm it.
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
      // Declarations are checked by initializer identity, others by structure.
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

// A definition earlier plugins created after entry: left for the exit compile.
function created(path, state) {
  const s = state.stylexFacade;
  if (s == null || s.names.empty) return;
  const node = path.node;
  const hold = s.holds.get(node);
  if (hold != null) {
    s.holds.delete(node);
    // A call rewritten here or edited in place by an earlier visitor is stale.
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
      // Compiled at exit, as visited here; later plugins see the compiled form.
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

// Counts hoist references that Babel's insertion misses, so they are not removed.
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

// StyleX the program gained after entry, classified as the original plugin's
// exit would: visited consumers compile; late definitions and sx elements stay.
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
  // Rebuild detached or unvisited definition sites from the surviving definitions.
  for (const site of s.sites) {
    const applied = found.positions.has(site.node);
    const waiting = site.held != null && found.positions.has(site.held);
    if (!applied && !waiting) found.compile = true;
    if (waiting && s.holds.has(site.held)) found.compile = true;
  }
  return found;
}

// Hides constructs from the exit compile behind placeholders, until `restore`.
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

// Removes unused definitions the entry compile dropped, wherever they are now.
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
  // With StyleX alone in its pass, unused definitions are where it left them.
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
  // A replaced initializer (coverage instrumentation) is a different definition.
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
  // Earlier plugins added StyleX the original plugin compiles: recompile once.
  const restored = revert(s.edits);
  for (const node of s.restored) restored.add(node);
  // The revert restores sx elements as written: they are source, not late.
  for (const element of s.sxElements || []) restored.add(element);
  s.restored = new Set();
  s.edits = [];
  // Definitions other plugins moved, and stale consumers, return to their places.
  const moved = lateStyleX(program.node, s, new Set()).positions;
  // A consumer's output can return only where an expression stood.
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
  // The entry compile was provisional: rules come from the surviving definitions.
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

// Queues nodes placed on exit on the file's traversal, as Babel does for inserts.
function requeue(program, nodes) {
  if (nodes.size === 0) return;
  // Create paths while walking, rather than resolving each from the program.
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

// Rewrites an sx element created after entry into a `props(...)` spread, as
// the original plugin does; without a StyleX namespace in scope it waits for exit.
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
