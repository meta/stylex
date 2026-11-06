//! Resolving a reference to the binding it names.
//!
//! One ordered question with one answer, asked once per identifier the
//! evaluator's dispatch could not fold from the injected function map. The
//! steps run in the reference implementation's order and each cites the
//! `evaluate-path.js` 0.19.0 line range it mirrors, so the two can be read side
//! by side; `docs/adr/0003-one-ordered-chain-resolves-a-reference.md` records
//! why the order is the reference implementation's rather than this compiler's.
//!
//! JS-parity: `utils/evaluate-path.js:595-690` (0.19.0) — the whole file mirrors
//! that range, so the marker sits here once rather than on each of the eight
//! steps, which carry their own line ranges in the banner that opens them.

use super::*;
use swc_core::common::Span;

// A note on what the unit tests beside this file do and do not cover. The
// resolved-import arm is exercised only by `validation_stylex_create_test`,
// which runs a real transform: `resolution_order.rs` builds a `StateManager`
// with no filename, so every import there resolves `Unresolved` and the arm is
// reached but never taken. A reader adding a case about a *resolved* import
// belongs in the transform tests rather than here.

/// Whether a reference reads a binding the program does not hold yet, because
/// the declarator naming it ends after the reference begins.
///
/// Mirrors the position comparison in `evaluate-path.js`'s
/// `isReferencedIdentifier` branch (`path.node.start < binding.path.node.end`,
/// 0.19.0 line 664), which is the reference implementation's whole answer to the
/// question. Declarations here are collected module-wide with no notion of
/// position, so without this the initializer is reachable from above its own
/// declaration and folds into CSS for a value that has not been assigned.
///
/// A dummy span on either side answers `false` rather than being compared. A
/// synthesized node carries no authored position, so it sits at byte zero,
/// before every authored declarator's end, and would be refused for having no
/// position rather than for being early. `expanded_shorthand_prop` is the
/// producer that reaches here, which builds the pair a shorthand name stands
/// for and gives it no position of its own.
///
/// The injected function mappers `get_var_decl_by_ident` also folds are not
/// expected to, because `nodes::identifier::evaluate` answers for every name in
/// `functions.identifiers` before this chain is entered — and that expectation
/// is not what keeps them safe. `create_var_declarator` gives every mapper's
/// declarator `DUMMY_SP`, so one arriving here is not compared at all; the guard
/// above is the reason, not the ordering. `used_before_declaration.rs`'s
/// `an_authored_reference_against_a_synthesized_declarator_folds` is that case.
///
/// Asked of a hoisted `function` or `class` declaration as well as of a
/// `VarDeclarator`, because upstream asks it of whatever the binding is: the
/// comparison at line 664 runs before the declaration-kind refusals at 685-690,
/// so a reference *above* a `function` is refused for being early and only a
/// reference below it is refused for its kind. Measured on 0.19.0:
///
/// ```text
/// create({ a: { color: f() } }); function f() {}   Referenced value is used before declaration.
/// function g() {} create({ a: { color: g() } })    Unsupported expression: FunctionDeclaration
/// ```
///
/// A hoisted declaration is compared against the end of its *name* rather than
/// the end of its body, which is what upstream's `binding.path.node.end` is. The
/// two part company for a reference from inside the declaration's own body:
/// `function f() { return create({ a: { color: f } }) }` sits after the name
/// ends and before the function does, so upstream calls it early where this
/// falls through to step 8's `FunctionDeclaration` refusal. Both refuse, so only
/// the sentence differs; closing it means recording a declaration's whole span
/// beside its name, which `declarations_state` does not carry.
fn reads_before_its_declaration(reference: &Ident, declaration: Span) -> bool {
  !reference.span.is_dummy() && !declaration.is_dummy() && reference.span.lo < declaration.hi
}

/// Resolve `reference` to the binding it names, and fold that binding to a
/// value — or refuse.
///
/// `path` is the expression as the caller received it and `normalized_path` the
/// same expression with its parenthesis and TypeScript wrappers peeled; the two
/// differ only in which one a diagnostic points at, and each step keeps the one
/// it reported against before this chain had a home of its own.
///
/// Every step but the last reports against the *declaration*, which is what
/// upstream's `deopt(binding.path, …)` does at 626, 647, 653, 657, 661, 665 and
/// 673 — so a refused build sends the reader to the line they have to change
/// rather than to a read that is correct as written. `deopt_at_declaration`
/// records the binding's name for the code frame to resolve; the reasoning for
/// recording a name rather than a span is on `stylex_diagnostics`. The
/// tail refusal at 687 stays on the reference, as upstream's does, because a
/// reference that resolved to itself has no declaration to name.
///
/// The steps, in `evaluate-path.js` 0.19.0 order:
///
/// 1. an import specifier resolves to a theme reference (599-650) — a *named*
///    one; the other two specifier kinds are answered inside this step
/// 2. a default-import specifier (652-654)
/// 3. a reassigned binding is not a constant (656-658)
/// 4. a binding mutated in place is not a constant (660-662)
/// 5. a reference above its own declaration is early (664-666)
/// 6. a binding carrying a folded value (668-669) — absent, see below
/// 7. `undefined` / `Infinity` / `NaN` (670-683)
/// 8. the declarator's initializer, else the class / function refusals
///    (685-690)
pub(super) fn resolve_reference(
  ident: &Ident,
  path: &Expr,
  normalized_path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  // ── 1. an import specifier resolves to a theme reference (599-650) ────────
  //
  // Steps 1 and 2 read one lookup, so they are nested rather than sequential:
  // both ask what kind of specifier binds this reference, and upstream's step 1
  // guard (`!bindingPath.isImportDefaultSpecifier() &&
  // !bindingPath.isImportNamespaceSpecifier() && bindingPath.isImportSpecifier()`,
  // 0.19.0 lines 600-605) is the same question steps 2 and this one's own
  // namespace arm answer. The order is upstream's either way — neither of the
  // other two specifier kinds resolves a theme reference here.
  if let Some((import_path, specifier)) = traversal_state.import_binding(ident) {
    // Which single export of the imported file this reference names, or `None`
    // where the specifier names no single export and the step therefore does
    // not apply.
    let imported: Option<ModuleExportName> = match specifier {
      // ── 2. a default-import specifier (652-654) ─────────────────────────
      //
      // A theme file is read through its named exports, so there is no theme
      // reference a default binding could answer with. Measured on both
      // compilers as `modules-1266-default-theme-import`: upstream refuses, and
      // resolving one here emitted a variable the theme file does not define.
      //
      // Deliberately outside the `disable_imports` guard below, as upstream is:
      // it gates only the resolution on `state.functions.disableImports` and
      // refuses a default import either way.
      //
      // Upstream does not return from its refusal — the reference falls through
      // the rest of the chain and lands on `UNDEFINED_CONST`, which deopts a
      // second time. The first refusal wins on both sides, so the fall-through
      // is unobservable; returning here says so.
      ImportSpecifier::Default(_) => {
        return deopt_at_declaration(
          path,
          &ident.sym,
          state,
          traversal_state,
          IMPORT_FILE_EVAL_ERROR,
        );
      },

      ImportSpecifier::Named(named) => Some(
        named
          .imported
          .clone()
          .unwrap_or_else(|| ModuleExportName::Ident(named.local.clone())),
      ),

      // A namespace specifier binds the module's whole export object, so it
      // names no export for a theme reference to be built from — which is why
      // upstream's step 1 excludes it: the step reads `importSpecifierNode.
      // imported`, a field an `ImportNamespaceSpecifier` does not carry. It is
      // a guard on the step's input rather than a verdict on the import kind,
      // and unlike a default specifier it is given no refusal of its own; the
      // reference falls through to `UNDEFINED_CONST` at the tail of the chain.
      //
      // Resolving one here instead — by synthesizing the reference's own local
      // alias as the export name — is what this arm gives up, and giving it up
      // is a decision rather than a mirror: it refuses input that compiled
      // before. `docs/adr/0007-a-namespace-import-of-a-theme-file-resolves-
      // nothing.md` argues it from the measurement that removed the trade — the
      // same token read through both import kinds in one module emitted two
      // custom properties, one of which nothing defines — and
      // `modules-1266-a-namespace-theme-import` is where that measurement is
      // executed rather than described.
      ImportSpecifier::Namespace(_) => None,
    };

    if let Some(imported) = imported
      && !state.functions.disable_imports
    {
      let abs_path = traversal_state.import_path_resolver(
        convert_atom_to_str_ref(&import_path.src.value),
        &mut FxHashMap::default(),
      );

      let imported_name = match imported {
        ModuleExportName::Ident(ident) => ident.sym.to_string(),
        ModuleExportName::Str(strng) => convert_atom_to_string(&strng.value),
      };

      let return_value = match abs_path {
        ImportPathResolution::Resolved { path: value } => {
          evaluate_theme_ref(&value, imported_name, traversal_state)
        },
        ImportPathResolution::Unresolved => {
          return deopt_at_declaration(
            path,
            &ident.sym,
            state,
            traversal_state,
            IMPORT_PATH_RESOLUTION_ERROR,
          );
        },
      };

      // No confidence check around what follows. Upstream gives
      // `IMPORT_FILE_EVAL_ERROR` a second time where a resolution came back
      // *unconfident* (0.19.0 line 647), and it reaches that branch only out of
      // `evaluateImportedFile`, which parses the imported module and folds the
      // named export out of it. This compiler has no such arm —
      // `ImportPathResolution` resolves to a path or to nothing — and
      // `evaluate_theme_ref` is a constructor over `&StateManager` that cannot
      // clear confidence. So a resolution reached from here always leaves the
      // evaluation confident, and a check would be a question with one answer.
      //
      // It comes back the day this compiler evaluates an imported file in its
      // own right, which is the same capability the globals step's cross-file
      // gap waits on.
      let import_path_src = convert_atom_to_string(&import_path.src.value);

      if !state.added_imports.contains(&import_path_src)
        && traversal_state.get_treeshake_compensation()
      {
        let prepend_import_module_item = add_import_expression(&import_path_src);
        // Theme side-effect imports go under ThemeImports — the slot whose
        // flush position matches the legacy `prepend_import_module_items`
        // placement (between the runtime helpers and the existing import block,
        // regardless of producer queue order). Dedup is by stable hash on the
        // StateManager so it survives across evaluations.
        traversal_state.queue_theme_import_if_absent(prepend_import_module_item);

        state.added_imports.insert(import_path_src);
      }

      return Some(EvaluateResultValue::ThemeRef(return_value));
    }
  }

  // A binding whose value can differ from its declaration initializer at this
  // use site — rebound or mutated — makes inlining that initializer bake in a
  // stale value. Both steps below are a hash probe guarded by the question
  // upstream guards them with (`binding &&`, 0.19.0 lines 656 and 660): does
  // this module declare the binding the write was recorded against.
  //
  // Asked of the *binding* rather than of a `VarDeclarator`, which is the wider
  // question and the right one. Every kind of declaration can be written to, and
  // a write makes the initializer stale whichever kind it was — so a name bound
  // by destructuring, or a reassigned `function` or `class`, is refused for the
  // write here rather than falling through to a refusal about something else.
  // Measured on 0.19.0: `let { primary } = …; primary = 'blue'` answers
  // `Referenced value is not a constant.` there, where the declarator-shaped
  // guard used to reach the tail refusal and say the constant was not defined,
  // and a reassigned `function` used to be refused for being a `function`.
  //
  // `declares_binding` is a hash probe keyed by full `Id`, so the guard costs no
  // scan of the declaration list at all — where the lookup it replaces walked
  // one — and a write to a shadowing binding still cannot refuse the binding it
  // shadows. Each step is spelled with its own probe first so step 4 costs
  // nothing once step 3 has refused.
  //
  // It is not *literally* upstream's question, and the difference is worth
  // knowing. Upstream asks its scope chain (`path.scope.getBinding(name)`), so a
  // binding out of scope at the reference answers nothing there; this asks a
  // module-wide set, so it answers for a binding declared anywhere. What keeps
  // the two together is the syntax context inside the `Id`: the resolver gives a
  // shadowing binding its own, and the write sets are keyed the same way, so a
  // reference only meets writes recorded against the binding it actually names.
  // Wider by construction, equal in practice, and it fails towards refusing.
  //
  // Neither probe can be answered by an injected function mapper the way a
  // declarator lookup could: `get_var_decl_by_ident` also matches the mappers
  // `create_var_declarator` builds, which are regenerated per evaluation and can
  // never hold a stale value, while these sets hold only what the module's own
  // pre-scan saw written. The mappers are answered before this chain is entered
  // at all, by `nodes::identifier::evaluate` reading `functions.identifiers`.
  //
  // Both are asked before the cloning lookup further down, which deep-clones the
  // `VarDeclarator` it finds and throws the clone away on the refusal path.

  // ── 3. a reassigned binding is not a constant (656-658) ───────────────────
  if traversal_state.has_binding_reassignment(ident) && traversal_state.declares_binding(ident) {
    return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
  }

  // ── 4. a binding mutated in place is not a constant (660-662) ─────────────
  if traversal_state.has_binding_mutation(ident) && traversal_state.declares_binding(ident) {
    return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
  }

  // The same step, for the writes upstream does not count as mutations at all:
  // `isMutated` asks that the reference's own parent be the member the write
  // lands on, so `obj.a.b = 1` is no mutation of `obj` there and its initializer
  // folds — with whatever the initializer said, which is now stale. This
  // compiler refuses instead, and that is a deliberate divergence rather than a
  // mirror: `docs/adr/0003` argues it, and it only ever refuses input upstream
  // compiles.
  //
  // What the extra reach is *not* allowed to do is change an answer that already
  // agreed. A deeper write is therefore asked of a `VarDeclarator` rather than of
  // the binding: a declarator is the only shape whose initializer this chain
  // would inline, so it is the only shape where a stale value could reach the
  // stylesheet. A `function`, a `class` or a destructured binding keeps the
  // refusal it had before — measured on 0.19.0, `function paint() {}` beside
  // `paint.a.b = 1` is `Unsupported expression: FunctionDeclaration` on both
  // sides, where refusing it for the write here would have diverged.
  let declarator = get_var_decl_parts_by_ident(ident, traversal_state, &state.functions);

  // ── 5. a reference above its own declaration is early (664-666) ───────────
  //
  // Asked of the declarator already looked up, so the answer costs a
  // comparison rather than a second scan of the declaration list.
  if let Some((declarator_span, _)) = declarator.as_ref()
    && reads_before_its_declaration(ident, *declarator_span)
  {
    return deopt_at_declaration(
      path,
      &ident.sym,
      state,
      traversal_state,
      USED_BEFORE_DECLARATION,
    );
  }

  // And of a hoisted `function` or `class`, which upstream asks here too — its
  // position comparison precedes the declaration-kind refusals step 8 reaches,
  // so a reference above one of these is early rather than unsupported. Only the
  // reference above is taken from this step; one below still falls through to
  // step 8 and keeps its kind's wording.
  // Both kinds are asked, not the first that answers: the walk this replaces
  // ran over the two lists joined, so a `class` whose position does not read
  // early must not hide a `function` of the same binding that does.
  if [
    traversal_state.class_name_declaration(ident),
    traversal_state.function_name_declaration(ident),
  ]
  .into_iter()
  .flatten()
  .any(|declared| reads_before_its_declaration(ident, declared))
  {
    return deopt_at_declaration(
      path,
      &ident.sym,
      state,
      traversal_state,
      USED_BEFORE_DECLARATION,
    );
  }

  // Placed here rather than beside step 4, where the paragraph above it sits,
  // because it is the one step upstream does not have — and the rule that
  // paragraph states is that the extra reach must not change an answer the two
  // compilers already agree on. Asked before step 5 it did exactly that for one
  // shape: `const theme = {…}` declared below a `create` that reads
  // `theme.a.b`, with `theme.a.b = 'blue'` after it. Upstream counts no
  // mutation there, so its position comparison wins and it says the reference
  // is used before its declaration; asked first, this said the value is not
  // constant. Both refuse and both frame the same declaration, so no build
  // changes — but the sentence the author reads was wrong about why.
  //
  // Asked of the `declarator` above rather than of the binding, for the reason
  // the paragraph above gives, and reusing it rather than scanning the
  // declaration list a second time.
  if declarator.is_some() && traversal_state.has_deep_binding_mutation(ident) {
    return deopt_at_declaration(path, &ident.sym, state, traversal_state, NON_CONSTANT);
  }

  // ── 6. a binding carrying a folded value (668-669) ────────────────────────
  //
  // Deliberately absent, and it is a comment rather than dead code because
  // there is nothing to write: upstream reads `binding.hasValue`, which Babel
  // sets only through `setValue` / `clearValue`, and the reference plugin never
  // calls either. The field is always false there, so this step never fires
  // there either, and every input falls through to the two below exactly as it
  // does here.

  // ── 7. `undefined` / `Infinity` / `NaN` (670-683) ─────────────────────────
  //
  // This step used to be asked before the import step, and moving it behind one
  // is the single outcome the reorder is not inert on: `NaN` is a legal binding
  // name, so `import { zIndex as NaN }` is the one shape where an import
  // specifier and one of these globals name the same binding, and a syntax
  // context cannot keep those two apart. The import now answers it, as it does
  // upstream — measured on both compilers as
  // `modules-1266-import-aliased-to-a-global-name`, which the reorder turns from
  // a divergence into agreement.
  //
  // The three names are ordinary bindings to the language, so the question is
  // which of the two this reference is — the global, or something in scope that
  // took the name over. The binding decides, and nothing below it does: the
  // step answers for all three names either way, and a reference that names a
  // binding never reaches the initializer read below.
  //
  // A binding that took one of these names over carries no value the evaluator
  // holds — the reference implementation reads `binding.hasValue`, which is step
  // 6, deliberately absent above and always false there — so there is nothing
  // to fold and the step refuses. That refusal is what turns a dynamic style's
  // parameter named `NaN` into an inline style: the value falls through to the
  // inline-style path, which is where the parameter comes from.
  //
  // The binding question is asked of the module being compiled, where upstream
  // asks the scope chain of whatever file the reference sits in. The two part
  // company on a reference read out of an *imported* file, whose bindings carry
  // a syntax context this module's pre-scan never saw: the name misses and the
  // global stands. It fails in the safe direction — a fold that should have
  // been refused, never a refusal of something that should fold — and closing
  // it means evaluating an imported file in its own right, which this compiler
  // does not do at all yet.
  //
  // What the step answers *with* is the value rather than the name, for the
  // reason written on `global_identifier_to_value`: two of the three names are
  // numbers, and a consumer reading the expression's shape cannot see that
  // through an identifier. Only the shadowing question is decided here.
  //
  // Reported against the declaration, as upstream reports it (line 673) and as
  // every step above does. `declared_bindings` has no position to hand over --
  // it answers whether a name is bound, keyed by `Id` -- and it does not need
  // one: what a refusal records is the binding's *name*, which the code frame
  // resolves against the module it re-parses. Measured on 0.19.0: `let NaN;`
  // above a `zIndex: NaN` frames the `NaN` in the declaration on both sides.
  if let Some(value) = global_identifier_to_value(ident) {
    return if traversal_state.declares_binding(ident) {
      deopt_at_declaration(
        path,
        &ident.sym,
        state,
        traversal_state,
        UNINITIALIZED_CONST,
      )
    } else {
      Some(EvaluateResultValue::Expr(value))
    };
  }

  // ── 8. the declarator's initializer, else the class / function refusals ───
  //      (685-690)
  if let Some(init) = declarator.and_then(|(_, init)| init) {
    return evaluate_cached(&init, state, traversal_state, fns);
  }

  // Asked of the state, which owns both lists, and answered as a `Copy` verdict
  // before the refusal below borrows it mutably. Cloning the two lists to hold
  // them open across that write is what this replaces -- and the comment which
  // justified those clones called this the refusal path as though it were rare.
  // It is not: a dynamic style's parameters are not registered when its body is
  // folded, so every parameter reference in every dynamic style arrives here.
  let declared_as = traversal_state.declared_as(ident);

  check_ident_declaration(ident, declared_as, state, traversal_state, normalized_path)
}

#[cfg(test)]
#[path = "tests/resolution_order.rs"]
mod resolution_order;

#[cfg(test)]
#[path = "tests/used_before_declaration.rs"]
mod used_before_declaration;
