use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_state::state_writers::{fill_call_positions, fill_top_level_expressions};
use swc_core::{
  common::{BytePos, Span, comments::Comments},
  ecma::{
    ast::{
      ArrayPat, ArrowExpr, AssignExpr, AssignTarget, AssignTargetPat, BindingIdent, BlockStmt,
      CallExpr, Callee, CatchClause, ClassDecl, ClassExpr, Expr, ExprOrSpread, FnDecl, FnExpr,
      ForHead, ForInStmt, ForOfStmt, Function, Id, Ident, ImportDecl, Lit, MemberExpr, MemberProp,
      Module, ModuleItem, ObjectPat, ObjectPatProp, OptChainBase, OptChainExpr, Pat, Program,
      Script, SimpleAssignTarget, UnaryExpr, UnaryOp, UpdateExpr, VarDecl, VarDeclKind,
    },
    visit::{Visit, VisitMutWith, VisitWith},
  },
};

use stylex_state_index::key_span_index::ModuleBase;

use crate::{
  StyleXTransform,
  shared::utils::live_declarations::{
    build_decl_use_graph, compute_live_set, mark_style_vars_to_keep,
  },
};
use rustc_hash::{FxHashMap, FxHashSet};
use stylex_ast::ast::imports::local_binding_of;
use stylex_constants::constants::common::{MUTATING_ARRAY_METHODS, MUTATING_OBJECT_METHODS};
use stylex_enums::core::TransformationCycle;
use stylex_state::state_manager::{BindingWrites, flush_pending_insertions};

/// Span covering the whole source, used for the module-level scope frame so
/// top-level bindings enclose every `sx` site. The scope stack is seeded with
/// this frame and never emptied, so the scope lookups always resolve to a real
/// frame and fall back to this span only in theory.
const MODULE_SCOPE_SPAN: Span = Span {
  lo: BytePos(0),
  hi: BytePos(u32::MAX),
};

/// The kind of lexical scope a frame on the [`ModuleBindingsCollector`] stack
/// represents. `var` declarations hoist to the nearest `Function` scope;
/// everything else (`let`/`const`/`class`, function declarations in module
/// code, params, catch bindings) belongs to the innermost scope, `Block` or
/// `Function`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
  Function,
  Block,
}

/// Which of the two write sets a recorded write belongs in. Both refuse a fold
/// with the same text; they are kept apart so the evaluator's resolution chain
/// can probe them as the two sequential steps the reference implementation
/// probes — `constantViolations` and `isMutated`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WriteKind {
  /// The binding itself is given a new value: an assignment, update,
  /// destructuring or loop target spelled as a bare name.
  Reassignment,
  /// The binding keeps pointing at the same value and that value changes:
  /// `obj.x = 1`, `arr.push(…)`, `delete obj.x`, `Object.assign(obj, …)`.
  ///
  /// One member hop from the binding, which is the shape the reference
  /// implementation's `isMutated` recognises: it asks that the reference's own
  /// parent be the member the write lands on.
  Mutation,
  /// The same, reached through more than one hop: `obj.a.b = 1`,
  /// `obj.items.push(…)`. Upstream calls this no mutation of `obj` at all, and
  /// folds the initializer. Recorded apart rather than folded into
  /// [`Self::Mutation`] so the chain can refuse it exactly where refusing
  /// protects something — see `StateManager::has_deep_binding_mutation`.
  DeepMutation,
}

/// The write kind after crossing one more member hop on the way to the binding
/// at the root of a chain.
///
/// The first hop makes a write a mutation of the object the reference names, and
/// that is the shape the reference implementation's `isMutated` recognises. A
/// second hop takes it past what upstream looks at: for `obj.a.b = 1` the
/// reference's parent is `obj.a`, whose own parent is another member rather than
/// the assignment, so upstream sees no mutation of `obj`. Every hop after the
/// first is therefore the same answer as the second, and the kind stops moving.
fn crossed_a_member_hop(kind: WriteKind) -> WriteKind {
  match kind {
    WriteKind::Reassignment => WriteKind::Mutation,
    WriteKind::Mutation | WriteKind::DeepMutation => WriteKind::DeepMutation,
  }
}

/// A lexical scope on the collector's stack, identified by the source span it
/// covers. Shadowing is decided by span containment: a binding shadows an
/// `sx` site when the binding's scope span encloses that site's span.
#[derive(Clone, Copy)]
struct ScopeFrame {
  span: Span,
  kind: ScopeKind,
}

/// One-time read-only pre-scan of the module performed at the start of the
/// `Discover` cycle. Captures the sources of every import declaration
/// (including type-only ones) in body order, the names of every bound
/// identifier in the module, and — for non-import bindings — the source span
/// of the scope each one occupies. It also records writes to bindings and
/// mutations of their referenced values for static evaluation. SWC visitors
/// have no parent pointers or scope chain, so this pre-scan supplies the
/// binding information used by both runtime-binding resolution and evaluation.
struct ModuleBindingsCollector {
  collect_sx_bindings: bool,
  import_sources: Vec<String>,
  bound_names: FxHashSet<String>,
  /// Every **declared binding** in the module, keyed by full SWC `Id`; the
  /// crate glossary defines the term. Collected in every mode, unlike
  /// [`Self::bound_names`], because the evaluator consumes it whether or not
  /// the `sx` prop is enabled.
  declared_bindings: FxHashSet<Id>,
  /// For each name bound by a non-import declaration, the spans of the scopes
  /// in which it is bound. A name shadows an `sx` site iff one of its scope
  /// spans encloses that site (see [`StateManager::is_locally_rebound_at`]).
  local_rebinding_scopes: FxHashMap<String, Vec<Span>>,
  /// Bindings rebound anywhere in the module. See
  /// [`StateManager::binding_reassignments`].
  binding_reassignments: FxHashSet<Id>,
  /// Bindings whose referenced value is mutated in place anywhere in the
  /// module. See [`StateManager::binding_mutations`].
  binding_mutations: FxHashSet<Id>,
  /// Bindings whose referenced value is mutated further down a member chain
  /// than the reference implementation looks. See
  /// [`StateManager::binding_deep_mutations`].
  binding_deep_mutations: FxHashSet<Id>,
  /// Stack of enclosing lexical scopes, outermost (module) first.
  scope_stack: Vec<ScopeFrame>,
  /// `VarDeclKind` of the `VarDecl` currently being visited, if any — needed
  /// to route `var` bindings to the nearest function scope (hoisting) while
  /// `let`/`const` stay in the innermost scope.
  current_var_kind: Option<VarDeclKind>,
}

impl ModuleBindingsCollector {
  /// The binding sets, moved out together.
  ///
  /// The one place the collector's four fields are matched to the state's four,
  /// so a transposition is a single visible mistake here rather than one
  /// repeated at every call site.
  fn take_binding_writes(&mut self) -> BindingWrites {
    BindingWrites {
      reassignments: std::mem::take(&mut self.binding_reassignments),
      mutations: std::mem::take(&mut self.binding_mutations),
      deep_mutations: std::mem::take(&mut self.binding_deep_mutations),
      declared: std::mem::take(&mut self.declared_bindings),
    }
  }

  /// Collects everything: the `sx` runtime-binding inputs (import sources,
  /// bound names, rebinding scopes) *and* the evaluator's inputs (the module's
  /// bindings and the writes against them), in one pass.
  fn for_sx() -> Self {
    Self::new(true)
  }

  /// Collects what the evaluator needs and nothing else — the bindings the
  /// module declares and the writes against them. Used when the `sx` prop is
  /// disabled and nothing consumes the import-source or name-keyed scope
  /// information.
  fn writes_only() -> Self {
    Self::new(false)
  }

  fn new(collect_sx_bindings: bool) -> Self {
    Self {
      collect_sx_bindings,
      import_sources: Vec::new(),
      bound_names: FxHashSet::default(),
      declared_bindings: FxHashSet::default(),
      local_rebinding_scopes: FxHashMap::default(),
      binding_reassignments: FxHashSet::default(),
      binding_mutations: FxHashSet::default(),
      binding_deep_mutations: FxHashSet::default(),
      // Seed a module-level function scope spanning the whole source so
      // top-level bindings enclose every `sx` site.
      scope_stack: vec![ScopeFrame {
        span: MODULE_SCOPE_SPAN,
        kind: ScopeKind::Function,
      }],
      current_var_kind: None,
    }
  }

  /// Span of the innermost enclosing function scope (where `var`/function
  /// declarations hoist to). Falls back to the module scope, always present.
  fn nearest_function_scope(&self) -> Span {
    self
      .scope_stack
      .iter()
      .rev()
      .find(|frame| frame.kind == ScopeKind::Function)
      .map(|frame| frame.span)
      .unwrap_or(MODULE_SCOPE_SPAN)
  }

  /// Span of the innermost enclosing scope (where `let`/`const`, params, etc.
  /// belong). Always present — the module scope sits at the stack bottom.
  fn innermost_scope(&self) -> Span {
    self
      .scope_stack
      .last()
      .map(|frame| frame.span)
      .unwrap_or(MODULE_SCOPE_SPAN)
  }

  /// Record a binding produced by a non-import declaration in `bindings`
  /// (always), `bound_names` (every binding) and `local_rebinding_scopes`
  /// (non-import only), scoping the latter to the function scope when `hoisted`
  /// (`var` declarations) or the innermost scope otherwise.
  ///
  /// `bindings` is filled ahead of the mode gate: the two name-keyed maps below
  /// serve the `sx` prop and are collected only where it is enabled, where the
  /// `Id`-keyed set serves the evaluator, which runs either way.
  fn add_local_binding(&mut self, ident: &Ident, hoisted: bool) {
    self.declared_bindings.insert(ident.to_id());

    if !self.collect_sx_bindings {
      return;
    }

    let name = ident.sym.to_string();
    let scope = if hoisted {
      self.nearest_function_scope()
    } else {
      self.innermost_scope()
    };
    self.bound_names.insert(name.clone());
    self
      .local_rebinding_scopes
      .entry(name)
      .or_default()
      .push(scope);
  }

  /// Record a write to `ident`'s binding under the kind it is: a rebinding or
  /// an in-place mutation of the value it references. Both make the
  /// declaration initializer unsafe to inline at a use site and both refuse
  /// with the same text; they are kept apart because the evaluator's
  /// resolution chain probes them as two sequential steps, mirroring the
  /// reference implementation's `constantViolations` and `isMutated`.
  ///
  /// Note the deliberate limit on what counts as a write: only mutations
  /// spelled out syntactically in this module are recorded. A binding that
  /// *escapes* into a call (`const a = []; mutate(a);`) is not, because
  /// deopting on every identifier passed as an argument would disable
  /// evaluation for nearly every StyleX module. The escape case is therefore a
  /// known unsoundness, accepted deliberately rather than overlooked.
  fn add_binding_write(&mut self, ident: &Ident, kind: WriteKind) {
    let target = match kind {
      WriteKind::Reassignment => &mut self.binding_reassignments,
      WriteKind::Mutation => &mut self.binding_mutations,
      WriteKind::DeepMutation => &mut self.binding_deep_mutations,
    };

    target.insert(ident.to_id());
  }

  /// Record the write performed by an expression used as a write target: a
  /// bare identifier rebinds itself, a member expression mutates the object at
  /// the *root* of its chain (`obj.a.b = 1` and `obj.a.b.push(…)` both
  /// invalidate `obj`). Walks past intermediate members and the parenthesised,
  /// optional-chain and TypeScript wrappers that can sit anywhere along the
  /// chain, stopping at anything that owns no binding to invalidate — a call
  /// result, `this`, `super`, or a literal.
  ///
  /// `kind` is what the caller's shape writes when it writes a bare name;
  /// crossing a member hop overrides it to a mutation, because the root object
  /// keeps its binding and only the value it references changes. So `n = 1`
  /// and `n++` reassign while `o.x = 1` and `o.x++` mutate `o`, out of one
  /// walk.
  ///
  /// This is the single entry point for every write shape the collector
  /// recognises (assignment targets, update and `delete` operands, mutating
  /// method receivers, `Object.assign` targets). Keeping one walk means a
  /// wrapper handled for one shape is handled for all of them; the earlier
  /// split between a member-only walk and a shallow expression match silently
  /// missed `(n)++`, `((a)) = 2` and `Object.assign((o), …)`.
  fn add_target_root_write(&mut self, expression: &Expr, kind: WriteKind) {
    let mut current = expression;
    let mut kind = kind;

    loop {
      match current {
        Expr::Ident(ident) => {
          self.add_binding_write(ident, kind);
          return;
        },
        Expr::Member(inner) => {
          kind = crossed_a_member_hop(kind);
          current = inner.obj.as_ref();
        },
        Expr::Paren(paren) => current = paren.expr.as_ref(),
        Expr::OptChain(opt_chain) => match opt_chain.base.as_ref() {
          OptChainBase::Member(inner) => {
            kind = crossed_a_member_hop(kind);
            current = inner.obj.as_ref();
          },
          // `f()?.x = 1` writes into a call result, which owns no binding.
          OptChainBase::Call(_) => return,
        },
        Expr::TsNonNull(non_null) => current = non_null.expr.as_ref(),
        Expr::TsAs(ts_as) => current = ts_as.expr.as_ref(),
        Expr::TsSatisfies(satisfies) => current = satisfies.expr.as_ref(),
        Expr::TsTypeAssertion(assertion) => current = assertion.expr.as_ref(),
        Expr::TsInstantiation(instantiation) => current = instantiation.expr.as_ref(),
        _ => return,
      }
    }
  }

  /// Record a write reached through a member expression (`obj.x = 1`,
  /// `obj.a.b.push(…)`, `delete obj.x`) by invalidating the binding at the
  /// root of the chain.
  fn add_member_root_write(&mut self, member_expression: &MemberExpr) {
    self.add_target_root_write(member_expression.obj.as_ref(), WriteKind::Mutation);
  }

  /// Record every binding written by an assignment or `for-in`/`for-of`
  /// pattern. Identifier targets rebind (`[a, b] = …`); `Pat::Expr` targets
  /// are member writes (`({ x: obj.x } = …)`) and invalidate the member's
  /// root object instead, through whatever wrappers sit around them.
  fn add_pattern_writes(&mut self, pattern: &Pat) {
    match pattern {
      Pat::Ident(binding_ident) => {
        self.add_binding_write(&binding_ident.id, WriteKind::Reassignment)
      },
      Pat::Array(array_pattern) => self.add_array_pattern_writes(array_pattern),
      Pat::Object(object_pattern) => self.add_object_pattern_writes(object_pattern),
      Pat::Rest(rest_pattern) => self.add_pattern_writes(&rest_pattern.arg),
      Pat::Assign(assign_pattern) => self.add_pattern_writes(&assign_pattern.left),
      Pat::Expr(expression) => {
        // Read through the wrappers, as every other write shape is: `[(o.x)]`
        // and `[o!.x]` are the target `[o.x]` is. Anything that is not a member
        // is a call result or a `this`, which owns no binding to invalidate.
        if let Some(member_expression) = member_target(expression) {
          self.add_member_root_write(member_expression);
        }
      },
      Pat::Invalid(_) => {},
    }
  }

  /// `[a, obj.x, ...rest] = …`, shared by `Pat` and `AssignTargetPat`.
  fn add_array_pattern_writes(&mut self, array_pattern: &ArrayPat) {
    for element in array_pattern.elems.iter().flatten() {
      self.add_pattern_writes(element);
    }
  }

  /// `{ a, b: obj.x, c = 1, ...rest } = …`, shared by `Pat` and
  /// `AssignTargetPat`.
  fn add_object_pattern_writes(&mut self, object_pattern: &ObjectPat) {
    for property in &object_pattern.props {
      match property {
        ObjectPatProp::KeyValue(key_value) => self.add_pattern_writes(&key_value.value),
        ObjectPatProp::Assign(assign) => {
          self.add_binding_write(&assign.key.id, WriteKind::Reassignment)
        },
        ObjectPatProp::Rest(rest) => self.add_pattern_writes(&rest.arg),
      }
    }
  }

  /// Record the write performed by an assignment target. Every arm that
  /// carries an expression defers to [`Self::add_target_root_write`], which
  /// unwraps the parenthesised and TypeScript wrappers that can nest between
  /// the target and the identifier or member it resolves to.
  fn add_simple_target_write(&mut self, target: &SimpleAssignTarget) {
    match target {
      SimpleAssignTarget::Ident(ident) => {
        self.add_binding_write(&ident.id, WriteKind::Reassignment)
      },
      SimpleAssignTarget::Member(member_expression) => {
        self.add_member_root_write(member_expression)
      },
      SimpleAssignTarget::Paren(paren) => {
        self.add_target_root_write(&paren.expr, WriteKind::Reassignment)
      },
      SimpleAssignTarget::OptChain(opt_chain) => {
        if let OptChainBase::Member(member_expression) = opt_chain.base.as_ref() {
          self.add_member_root_write(member_expression);
        }
      },
      SimpleAssignTarget::TsAs(ts_as) => {
        self.add_target_root_write(&ts_as.expr, WriteKind::Reassignment)
      },
      SimpleAssignTarget::TsSatisfies(satisfies) => {
        self.add_target_root_write(&satisfies.expr, WriteKind::Reassignment)
      },
      SimpleAssignTarget::TsNonNull(non_null) => {
        self.add_target_root_write(&non_null.expr, WriteKind::Reassignment)
      },
      SimpleAssignTarget::TsTypeAssertion(assertion) => {
        self.add_target_root_write(&assertion.expr, WriteKind::Reassignment)
      },
      SimpleAssignTarget::TsInstantiation(instantiation) => {
        self.add_target_root_write(&instantiation.expr, WriteKind::Reassignment)
      },
      // `super.x = 1` and invalid targets bind no module-level name.
      SimpleAssignTarget::SuperProp(_) | SimpleAssignTarget::Invalid(_) => {},
    }
  }

  /// Record the mutations performed by a method call, shared by plain calls
  /// (`arr.push(1)`) and optional calls (`arr?.push(1)`) — both reach the same
  /// receiver, so both must invalidate it.
  fn add_call_mutation_writes(&mut self, callee: &Expr, args: &[ExprOrSpread]) {
    let Some(member_expression) = member_target(callee) else {
      return;
    };

    let Some(property) = member_property_name(&member_expression.prop) else {
      // A dynamic property (`arr[method](…)`) names no known method. Nothing
      // is recorded: guessing would deopt every computed call in the module.
      return;
    };

    // `arr.push(…)`, `arr.sort()`, … mutate the receiver in place. The
    // receiver may itself be a member chain (`state.items.push(…)`), in
    // which case the binding to invalidate is the chain's root object.
    if MUTATING_ARRAY_METHODS.contains(property) {
      self.add_member_root_write(member_expression);
    }

    // `Object.assign(target, …)` and friends mutate their first argument.
    // Matching on the `Object` name is a deliberate over-approximation: a
    // shadowed `Object` only costs a deopt, never a wrong inline.
    if MUTATING_OBJECT_METHODS.contains(property)
      && member_expression
        .obj
        .as_ident()
        .is_some_and(|object| object.sym == "Object")
      && let Some(first_argument) = args.first()
      && first_argument.spread.is_none()
    {
      self.add_target_root_write(first_argument.expr.as_ref(), WriteKind::Mutation);
    }
  }
}

/// The `MemberExpr` an expression resolves to, looking through parentheses,
/// TypeScript wrappers and the optional-chain link of `obj?.x`. `None` when
/// the expression is not a member access — used where only a member access can
/// signal a mutation (a mutating method's callee, a `delete` operand), so that
/// `arr.push`, `(arr).push` and `arr?.push` are all recognised alike.
fn member_target(expression: &Expr) -> Option<&MemberExpr> {
  match expression {
    Expr::Member(member_expression) => Some(member_expression),
    Expr::Paren(paren) => member_target(paren.expr.as_ref()),
    Expr::TsNonNull(non_null) => member_target(non_null.expr.as_ref()),
    Expr::TsAs(ts_as) => member_target(ts_as.expr.as_ref()),
    Expr::TsSatisfies(satisfies) => member_target(satisfies.expr.as_ref()),
    Expr::TsTypeAssertion(assertion) => member_target(assertion.expr.as_ref()),
    Expr::TsInstantiation(instantiation) => member_target(instantiation.expr.as_ref()),
    Expr::OptChain(opt_chain) => match opt_chain.base.as_ref() {
      OptChainBase::Member(member_expression) => Some(member_expression),
      OptChainBase::Call(_) => None,
    },
    _ => None,
  }
}

/// The statically known name of a member property: `obj.push` and
/// `obj["push"]` both yield `push`. `None` for a genuinely dynamic property,
/// whose name is unknowable at build time.
fn member_property_name(property: &MemberProp) -> Option<&str> {
  match property {
    MemberProp::Ident(ident) => Some(ident.sym.as_ref()),
    MemberProp::Computed(computed) => match computed.expr.as_ref() {
      // A lone-surrogate property name cannot be one of the method names being
      // matched, so treat it as unknown rather than panicking on the decode.
      Expr::Lit(Lit::Str(string)) => string.value.as_atom().map(|atom| &**atom),
      _ => None,
    },
    MemberProp::PrivateName(_) => None,
  }
}

impl Visit for ModuleBindingsCollector {
  fn visit_import_decl(&mut self, import_decl: &ImportDecl) {
    // An import local is a binding like any other, and the evaluator asks about
    // it in every mode -- so `bindings` is filled before the mode gate. An
    // import declaration contains no write targets, so nothing below the
    // specifiers is worth descending into either way.
    for specifier in &import_decl.specifiers {
      let local = local_binding_of(specifier);

      self.declared_bindings.insert(local.to_id());

      if self.collect_sx_bindings {
        // Import locals are bindings, but never count as a re-binding that
        // would shadow another import.
        self.bound_names.insert(local.sym.to_string());
      }
    }

    if !self.collect_sx_bindings {
      return;
    }

    self
      .import_sources
      .push(convert_atom_to_string(&import_decl.src.value));
  }

  fn visit_function(&mut self, function: &Function) {
    self.scope_stack.push(ScopeFrame {
      span: function.span,
      kind: ScopeKind::Function,
    });
    function.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
    self.scope_stack.push(ScopeFrame {
      span: arrow_expr.span,
      kind: ScopeKind::Function,
    });
    arrow_expr.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
    self.scope_stack.push(ScopeFrame {
      span: fn_expr.function.span,
      kind: ScopeKind::Function,
    });

    if let Some(ident) = &fn_expr.ident {
      // A named function expression's name is bound only inside the function
      // body, where it can shadow an imported `stylex` namespace.
      self.add_local_binding(ident, false);
    }

    fn_expr.function.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_class_expr(&mut self, class_expr: &ClassExpr) {
    self.scope_stack.push(ScopeFrame {
      span: class_expr.class.span,
      kind: ScopeKind::Block,
    });

    if let Some(ident) = &class_expr.ident {
      // A named class expression's name is visible inside the class body.
      self.add_local_binding(ident, false);
    }

    class_expr.class.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt) {
    self.scope_stack.push(ScopeFrame {
      span: block_stmt.span,
      kind: ScopeKind::Block,
    });
    block_stmt.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_catch_clause(&mut self, catch_clause: &CatchClause) {
    self.scope_stack.push(ScopeFrame {
      span: catch_clause.span,
      kind: ScopeKind::Block,
    });
    catch_clause.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl) {
    let previous = self.current_var_kind.replace(var_decl.kind);
    var_decl.visit_children_with(self);
    self.current_var_kind = previous;
  }

  fn visit_binding_ident(&mut self, binding_ident: &BindingIdent) {
    let hoisted = self.current_var_kind == Some(VarDeclKind::Var);
    self.add_local_binding(&binding_ident.id, hoisted);
    binding_ident.visit_children_with(self);
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
    // Function declarations in modules are block-scoped; top-level ones still
    // land in the module scope because it is the innermost frame there.
    self.add_local_binding(&fn_decl.ident, false);
    fn_decl.visit_children_with(self);
  }

  fn visit_class_decl(&mut self, class_decl: &ClassDecl) {
    // Class declarations are block-scoped, not hoisted.
    self.add_local_binding(&class_decl.ident, false);
    class_decl.visit_children_with(self);
  }

  fn visit_assign_expr(&mut self, assign_expression: &AssignExpr) {
    match &assign_expression.left {
      AssignTarget::Simple(target) => self.add_simple_target_write(target),
      AssignTarget::Pat(pattern) => match pattern {
        AssignTargetPat::Array(array_pattern) => self.add_array_pattern_writes(array_pattern),
        AssignTargetPat::Object(object_pattern) => self.add_object_pattern_writes(object_pattern),
        AssignTargetPat::Invalid(_) => {},
      },
    }

    assign_expression.visit_children_with(self);
  }

  fn visit_update_expr(&mut self, update_expression: &UpdateExpr) {
    self.add_target_root_write(update_expression.arg.as_ref(), WriteKind::Reassignment);

    update_expression.visit_children_with(self);
  }

  fn visit_unary_expr(&mut self, unary_expression: &UnaryExpr) {
    // `delete obj.x` mutates `obj`. Only a member operand is a mutation:
    // `delete someBinding` is a no-op on a declared binding's value.
    if unary_expression.op == UnaryOp::Delete
      && let Some(member_expression) = member_target(unary_expression.arg.as_ref())
    {
      self.add_member_root_write(member_expression);
    }

    unary_expression.visit_children_with(self);
  }

  fn visit_call_expr(&mut self, call_expression: &CallExpr) {
    if let Callee::Expr(callee) = &call_expression.callee {
      self.add_call_mutation_writes(callee.as_ref(), &call_expression.args);
    }

    call_expression.visit_children_with(self);
  }

  fn visit_opt_chain_expr(&mut self, opt_chain: &OptChainExpr) {
    // `arr?.push(1)` parses as an optional *call*, not a `CallExpr`, so it
    // needs its own hook — it mutates the receiver exactly as `arr.push(1)`
    // does whenever the receiver is non-nullish.
    if let OptChainBase::Call(call) = opt_chain.base.as_ref() {
      self.add_call_mutation_writes(call.callee.as_ref(), &call.args);
    }

    opt_chain.visit_children_with(self);
  }

  fn visit_for_in_stmt(&mut self, for_in_statement: &ForInStmt) {
    if let ForHead::Pat(pattern) = &for_in_statement.left {
      self.add_pattern_writes(pattern);
    }

    for_in_statement.visit_children_with(self);
  }

  fn visit_for_of_stmt(&mut self, for_of_statement: &ForOfStmt) {
    if let ForHead::Pat(pattern) = &for_of_statement.left {
      self.add_pattern_writes(pattern);
    }

    for_of_statement.visit_children_with(self);
  }
}

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_program_impl(&mut self, program: &mut Program) {
    match program {
      Program::Module(module) => self.visit_mut_module_impl(module),
      Program::Script(script) => {
        // A `Script` body holds plain statements, not `ModuleItem`s, so it
        // cannot carry the injected `import * as stylex` declaration that the
        // `sx` runtime binding may need. Promote it to a `Module`, run the
        // standard pipeline, then restore the `Script` form when no
        // module-level declaration was added — preserving non-module output
        // verbatim for inputs that need no injection.
        let mut module = Module {
          span: script.span,
          body: std::mem::take(&mut script.body)
            .into_iter()
            .map(ModuleItem::Stmt)
            .collect(),
          shebang: script.shebang.take(),
        };

        self.visit_mut_module_impl(&mut module);

        if module
          .body
          .iter()
          .any(|item| matches!(item, ModuleItem::ModuleDecl(_)))
        {
          *program = Program::Module(module);
        } else {
          // The `any(ModuleDecl)` check above is false here, so every item is
          // a `Stmt`; `ModuleItem::stmt` extracts each one.
          *script = Script {
            span: module.span,
            shebang: module.shebang,
            body: module
              .body
              .into_iter()
              .filter_map(ModuleItem::stmt)
              .collect(),
          };
        }
      },
    }
  }

  pub(crate) fn visit_mut_module_impl(&mut self, module: &mut Module) {
    // Recorded unconditionally, unlike the memo below it. A key-span lookup
    // compares this module's positions against candidates indexed from a
    // re-parse into the code frame's own source map, and only offsets into the
    // file compare -- but `memoize_module` will re-parse and memoize on demand,
    // so that lookup is reachable on configurations this branch skips. Left
    // inside it, the base went unset exactly there and every offset silently
    // became the raw position again.
    self.state.set_input_module_base(ModuleBase::of(module));

    // The copy is a deep clone of the whole module, so a release build makes one
    // only where the compiler cannot read the file back off disk. The state
    // owns that choice, and the build this pass was compiled for is the only
    // half of it this walk knows.
    self
      .state
      .keep_module_source_copy(module, cfg!(debug_assertions));

    self.discover_module(module);

    if !self.state.has_import_paths() && self.state.atom_imports.is_empty() {
      return;
    }

    // Binding writes are only read by the evaluator, which runs from here on,
    // so modules that never reach this point pay no pre-scan. When the `sx`
    // feature is on, `discover_module` already scanned (its output is needed
    // mid-walk) and this is a no-op.
    self.collect_binding_writes(module);

    self.transform_producers(module);
    self.transform_atoms(module);
    self.transform_consumers(module);
    self.finalize_module(module);
  }

  /// Run the discovery pass.
  ///
  /// Walks the module once under the `Discover` cycle, populating import
  /// state, transforming compiled-JSX `sx` attributes, counting variable /
  /// member-expression references, and pre-filling top-level declarations —
  /// all the work the legacy `Initializing` + `StateFilling` two-pass split
  /// used to do separately. Whenever stylex was imported, also captures the
  /// top-level expressions consumed by later phases.
  pub(crate) fn discover_module(&mut self, module: &mut Module) {
    self.state.cycle = TransformationCycle::Discover;

    // The `sx` runtime-binding injection runs mid-walk in this same cycle and
    // consults the pre-scan, so with `sx` enabled the scan has to happen up
    // front. It collects binding writes in the same pass; without `sx` the
    // scan is deferred to `collect_binding_writes`, which only runs for
    // modules that actually reach evaluation.
    if self.state.options.sx_prop_name.is_some() {
      let mut collector = ModuleBindingsCollector::for_sx();
      module.visit_with(&mut collector);

      self
        .state
        .adopt_binding_writes(collector.take_binding_writes());
      self.state.existing_import_sources = collector.import_sources;
      self.state.bound_names = collector.bound_names;
      self.state.local_rebinding_scopes = collector.local_rebinding_scopes;
    }

    module.visit_mut_children_with(self);

    if self.state.has_import_paths() {
      fill_top_level_expressions(module, &mut self.state);
      fill_call_positions(module, &mut self.state);
    }
  }

  /// Record every binding the module declares, and every one it rebinds or
  /// mutates, so the evaluator can ask which binding a reference names and
  /// never inlines a declaration initializer that no longer holds at the use
  /// site. No-op when `discover_module` already collected them for `sx`.
  pub(crate) fn collect_binding_writes(&mut self, module: &Module) {
    if self.state.options.sx_prop_name.is_some() {
      return;
    }

    let mut collector = ModuleBindingsCollector::writes_only();
    module.visit_with(&mut collector);

    self
      .state
      .adopt_binding_writes(collector.take_binding_writes());
  }

  /// Run the producer transformation pass.
  ///
  /// Transforms `stylex.create` / `defineVars` / `keyframes` / etc. — the calls
  /// that *produce* style namespaces consumed by later phases.
  pub(crate) fn transform_producers(&mut self, module: &mut Module) {
    self.state.cycle = TransformationCycle::TransformProducers;
    module.visit_mut_children_with(self);
  }

  /// Run the consumer transformation pass plus pending-insertion flush.
  ///
  /// Transforms `stylex.props` / `stylex.attrs` (which consume the style
  /// namespaces produced by the prior phase). After the consumer walk
  /// completes, drains the pending-insertion buffer with a single
  /// linear merge into the module body. Runtime helpers and per-decl
  /// metadata are gated on `options.runtime_injection.is_some()`,
  /// matching the legacy gate; hoisted dynamic-style consts always
  /// emit.
  pub(crate) fn transform_consumers(&mut self, module: &mut Module) {
    self.state.cycle = TransformationCycle::TransformConsumers;
    module.visit_mut_children_with(self);

    let runtime_injection = self.state.options.runtime_injection.is_some();
    flush_pending_insertions(&mut self.state, &mut module.body, runtime_injection);
  }

  /// Run the cleanup phase: materialize deferred JSX-spread
  /// replacements, build the decl-reference graph, compute the live
  /// set, then sweep unused declarations in a single forward pass.
  ///
  /// The mark step (`mark_style_vars_to_keep`) walks the module once
  /// and populates `state.style_vars_to_keep` plus materializes any
  /// deferred JSX-spread replacements. The graph is then captured at
  /// G-PostHoc against the post-mark AST, fixing the live set used by
  /// the sweep. The sweep itself runs under `TransformationCycle::Finalize`
  /// in original module-body order — the live-set is computed up
  /// front, so no body reversal is needed to handle transitive removal.
  pub(crate) fn finalize_module(&mut self, module: &mut Module) {
    // The mark phase materializes any deferred JSX-spread replacements
    // accumulated during the consumer walk; running it first ensures the
    // subsequent graph capture sees the final JSX shape (no leftover
    // `stylex.props(styles.X)` SpreadElement nodes).
    mark_style_vars_to_keep(module, &mut self.state);

    // Capture the decl-reference graph after producer + consumer
    // transforms — and the JSX-spread materialization above — have run.
    // At this point inlined references (e.g. `styles.container` in JSX,
    // replaced with literal class strings) are gone from the AST, so
    // the graph reflects only the references that survive into the
    // emitted module.
    build_decl_use_graph(module, &mut self.state);

    // Compute the live-set up front so the sweep filter has a single
    // immutable membership check per declarator. Computation depends on
    // `state.roots` and `state.decl_uses`, both finalized by the
    // mark-phase pass above.
    self.state.live_set = compute_live_set(&self.state);

    self.state.cycle = TransformationCycle::Finalize;

    module.visit_mut_children_with(self);
  }
}

#[cfg(test)]
#[path = "tests/module_bindings_collector_tests.rs"]
mod tests;
