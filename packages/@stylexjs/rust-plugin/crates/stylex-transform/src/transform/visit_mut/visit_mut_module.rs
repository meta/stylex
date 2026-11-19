use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_state::state_writers::{fill_call_positions, fill_top_level_expressions};
use swc_core::{
  common::{BytePos, Span, comments::Comments},
  ecma::{
    ast::{
      ArrayPat, ArrowExpr, AssignExpr, AssignTarget, AssignTargetPat, BindingIdent, BlockStmt,
      CallExpr, Callee, CatchClause, ClassDecl, ClassExpr, Decl, DefaultDecl, Expr, ExprOrSpread,
      FnDecl, FnExpr, ForHead, ForInStmt, ForOfStmt, Function, Id, Ident, ImportDecl, Lit,
      MemberExpr, MemberProp, Module, ModuleDecl, ModuleItem, ObjectPat, ObjectPatProp,
      OptChainBase, OptChainExpr, Pat, Program, Script, SimpleAssignTarget, TsModuleName,
      UnaryExpr, UnaryOp, UpdateExpr, VarDecl, VarDeclKind,
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
use stylex_state::state_manager::{
  BindingWrites, OpaqueDeclarationFault, OpaqueDeclarationKind, flush_pending_insertions,
};

/// Span covering the whole source, used for the module-level scope frame.
const MODULE_SCOPE_SPAN: Span = Span {
  lo: BytePos(0),
  hi: BytePos(u32::MAX),
};

/// Kind of lexical scope on the [`ModuleBindingsCollector`] stack. `var` hoists to the nearest
/// `Function` scope; all other bindings go to the innermost scope.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
  Function,
  Block,
}

/// Which write set a recorded write belongs in; kept apart so the evaluator probes them in
/// the upstream order (`constantViolations`, then `isMutated`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum WriteKind {
  /// The binding gets a new value (assignment, update, destructuring or loop target).
  Reassignment,
  /// The referenced value changes one member hop away (`obj.x = 1`, `arr.push(…)`), matching
  /// upstream `isMutated`.
  Mutation,
  /// A mutation more than one hop away (`obj.a.b = 1`), which upstream ignores; see
  /// `StateManager::has_deep_binding_mutation`.
  DeepMutation,
}

/// The write kind after one more member hop: the first makes a [`WriteKind::Mutation`],
/// later ones a [`WriteKind::DeepMutation`] (beyond what upstream `isMutated` sees).
fn crossed_a_member_hop(kind: WriteKind) -> WriteKind {
  match kind {
    WriteKind::Reassignment => WriteKind::Mutation,
    WriteKind::Mutation | WriteKind::DeepMutation => WriteKind::DeepMutation,
  }
}

/// A lexical scope identified by its span; a binding shadows an `sx` site whose span its
/// scope encloses.
#[derive(Clone, Copy)]
struct ScopeFrame {
  span: Span,
  kind: ScopeKind,
}

/// Read-only pre-scan at the start of `Discover`: import sources, bound names, scope spans of
/// non-import bindings, and binding writes for static evaluation (SWC has no scope chain).
/// Only calls with a known receiver add argument-to-parameter write edges.
#[derive(Clone)]
enum KnownCallee {
  Binding(Id),
  Parameters(Vec<Option<Id>>),
}
struct KnownCall {
  callee: KnownCallee,
  arguments: Vec<Option<Id>>,
  array_receiver: Option<Id>,
}

struct ModuleBindingsCollector {
  collect_sx_bindings: bool,
  import_sources: Vec<String>,
  bound_names: FxHashSet<String>,
  /// Every **declared binding**, keyed by SWC `Id`; collected in every mode, unlike
  /// [`Self::bound_names`].
  declared_bindings: FxHashSet<Id>,
  /// Source faults for bindings whose declaration is not value-folded.
  opaque_declaration_faults: FxHashMap<Id, OpaqueDeclarationFault>,
  /// Scope spans of each non-import binding name (see [`StateManager::is_locally_rebound_at`]).
  local_rebinding_scopes: FxHashMap<String, Vec<Span>>,
  /// Bindings rebound anywhere. See [`StateManager::binding_reassignments`].
  binding_reassignments: FxHashSet<Id>,
  /// Bindings whose value is mutated in place. See [`StateManager::binding_mutations`].
  binding_mutations: FxHashSet<Id>,
  /// Bindings mutated deeper than upstream checks. See
  /// [`StateManager::binding_deep_mutations`].
  binding_deep_mutations: FxHashSet<Id>,
  binding_aliases: FxHashMap<Id, FxHashSet<Id>>,
  array_aliases: FxHashMap<Id, FxHashSet<Id>>,
  array_bindings: FxHashSet<Id>,
  functions: FxHashMap<Id, Vec<Option<Id>>>,
  known_calls: Vec<KnownCall>,
  iterations: Vec<(Vec<Id>, Id)>,
  /// Stack of enclosing lexical scopes, outermost (module) first.
  scope_stack: Vec<ScopeFrame>,
  /// Kind of the `VarDecl` being visited, to hoist `var` bindings to the function scope.
  current_var_kind: Option<VarDeclKind>,
}

impl ModuleBindingsCollector {
  /// Pattern bindings only; initializer/default expressions take the normal discovery path.
  fn record_opaque_pattern(&mut self, pattern: &Pat, fault: OpaqueDeclarationFault) {
    struct PatternFaults<'a> {
      faults: &'a mut FxHashMap<Id, OpaqueDeclarationFault>,
      fault: OpaqueDeclarationFault,
    }
    impl Visit for PatternFaults<'_> {
      fn visit_binding_ident(&mut self, binding: &BindingIdent) {
        self.faults.entry(binding.id.to_id()).or_insert(self.fault);
      }
      fn visit_expr(&mut self, _: &Expr) {}
      fn visit_ts_type(&mut self, _: &swc_core::ecma::ast::TsType) {}
    }
    pattern.visit_with(&mut PatternFaults {
      faults: &mut self.opaque_declaration_faults,
      fault,
    });
  }

  fn record_parameter_fault(&mut self, pattern: &Pat) {
    if let Pat::Ident(binding) = pattern {
      self
        .opaque_declaration_faults
        .entry(binding.id.to_id())
        .or_insert(OpaqueDeclarationFault {
          kind: OpaqueDeclarationKind::Identifier,
          span: binding.id.span,
        });
    }
  }

  /// Moves the binding sets out, matching collector fields to state fields in one place.
  fn take_binding_writes(&mut self) -> BindingWrites {
    fn propagate(writes: &mut FxHashSet<Id>, aliases: &FxHashMap<Id, FxHashSet<Id>>) {
      let mut work = writes.iter().cloned().collect::<Vec<_>>();
      while let Some(id) = work.pop() {
        if let Some(links) = aliases.get(&id) {
          for link in links {
            if writes.insert(link.clone()) {
              work.push(link.clone());
            }
          }
        }
      }
    }
    // Resolve forward named callees after discovery. Edges carry only object writes from
    // parameters to concrete arguments; rebinding a parameter never mutates the caller's object.
    propagate(&mut self.array_bindings, &self.array_aliases);
    for (parameters, receiver) in &self.iterations {
      if self.array_bindings.contains(receiver) {
        for parameter in parameters {
          self
            .binding_aliases
            .entry(parameter.clone())
            .or_default()
            .insert(receiver.clone());
        }
      }
    }
    for call in &self.known_calls {
      if call
        .array_receiver
        .as_ref()
        .is_some_and(|receiver| !self.array_bindings.contains(receiver))
      {
        continue;
      }
      let parameters = match &call.callee {
        KnownCallee::Parameters(parameters) => Some(parameters),
        KnownCallee::Binding(id) => {
          let mut visited = FxHashSet::default();
          let mut work = vec![id.clone()];
          let mut found = None;
          while let Some(id) = work.pop() {
            if !visited.insert(id.clone()) {
              continue;
            }
            if let Some(parameters) = self.functions.get(&id) {
              found = Some(parameters);
              break;
            }
            if let Some(aliases) = self.array_aliases.get(&id) {
              work.extend(aliases.iter().cloned());
            }
          }
          found
        },
      };
      if let Some(parameters) = parameters {
        for (parameter, argument) in parameters.iter().zip(&call.arguments) {
          if let (Some(parameter), Some(argument)) = (parameter, argument) {
            self
              .binding_aliases
              .entry(parameter.clone())
              .or_default()
              .insert(argument.clone());
          }
        }
      }
    }
    // Object writes affect every alias of the object; reassigning an alias does not.
    propagate(&mut self.binding_mutations, &self.binding_aliases);
    propagate(&mut self.binding_deep_mutations, &self.binding_aliases);
    BindingWrites {
      reassignments: std::mem::take(&mut self.binding_reassignments),
      mutations: std::mem::take(&mut self.binding_mutations),
      deep_mutations: std::mem::take(&mut self.binding_deep_mutations),
      declared: std::mem::take(&mut self.declared_bindings),
    }
  }

  /// Collects both the `sx` runtime-binding inputs and the evaluator's inputs.
  fn for_sx() -> Self {
    Self::new(true)
  }

  /// Collects only the evaluator's inputs; used when the `sx` prop is disabled.
  fn writes_only() -> Self {
    Self::new(false)
  }

  fn new(collect_sx_bindings: bool) -> Self {
    Self {
      collect_sx_bindings,
      import_sources: Vec::new(),
      bound_names: FxHashSet::default(),
      declared_bindings: FxHashSet::default(),
      opaque_declaration_faults: FxHashMap::default(),
      local_rebinding_scopes: FxHashMap::default(),
      binding_reassignments: FxHashSet::default(),
      binding_mutations: FxHashSet::default(),
      binding_deep_mutations: FxHashSet::default(),
      binding_aliases: FxHashMap::default(),
      array_aliases: FxHashMap::default(),
      array_bindings: FxHashSet::default(),
      functions: FxHashMap::default(),
      known_calls: Vec::new(),
      iterations: Vec::new(),
      scope_stack: vec![ScopeFrame {
        span: MODULE_SCOPE_SPAN,
        kind: ScopeKind::Function,
      }],
      current_var_kind: None,
    }
  }

  /// Span of the innermost function scope (where `var` hoists), else the module scope.
  fn nearest_function_scope(&self) -> Span {
    self
      .scope_stack
      .iter()
      .rev()
      .find(|frame| frame.kind == ScopeKind::Function)
      .map(|frame| frame.span)
      .unwrap_or(MODULE_SCOPE_SPAN)
  }

  /// Span of the innermost enclosing scope.
  fn innermost_scope(&self) -> Span {
    self
      .scope_stack
      .last()
      .map(|frame| frame.span)
      .unwrap_or(MODULE_SCOPE_SPAN)
  }

  /// Record a non-import binding: always in `bindings`; in `bound_names` and
  /// `local_rebinding_scopes` (function scope if `hoisted`, else innermost) only for `sx`.
  fn add_local_binding(&mut self, ident: &Ident, hoisted: bool) {
    self.declared_bindings.insert(ident.to_id());
    self.bound_names.insert(ident.sym.to_string());

    if !self.collect_sx_bindings {
      return;
    }

    let name = ident.sym.to_string();
    let scope = if hoisted {
      self.nearest_function_scope()
    } else {
      self.innermost_scope()
    };
    // A module-scope declaration can't shadow a module-level `stylex` binding: it either is
    // that binding (a top-level `require`) or would be a redeclaration error.
    if scope == MODULE_SCOPE_SPAN {
      return;
    }
    self.bound_names.insert(name.clone());
    self
      .local_rebinding_scopes
      .entry(name)
      .or_default()
      .push(scope);
  }

  /// Record a write to `ident`'s binding (rebinding or mutation); either makes the initializer
  /// unsafe to inline. Known unsoundness: escapes into calls (`mutate(a)`) are not recorded.
  fn add_binding_write(&mut self, ident: &Ident, kind: WriteKind) {
    let target = match kind {
      WriteKind::Reassignment => &mut self.binding_reassignments,
      WriteKind::Mutation => &mut self.binding_mutations,
      WriteKind::DeepMutation => &mut self.binding_deep_mutations,
    };

    target.insert(ident.to_id());
  }

  /// Record a write through a target expression: a bare name writes with `kind`; a member chain
  /// mutates its root object. Looks through parens, optional chains and TS wrappers; stops at
  /// call results, `this`, `super` and literals. The single entry point for every write shape.
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

  /// Record a member write (`obj.x = 1`, `delete obj.x`) against the chain's root binding.
  fn add_member_root_write(&mut self, member_expression: &MemberExpr) {
    self.add_target_root_write(member_expression.obj.as_ref(), WriteKind::Mutation);
  }

  /// Record writes from an assignment or `for-in`/`for-of` pattern: names rebind; member
  /// targets (`({ x: obj.x } = …)`) mutate their root object.
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
        // Look through wrappers: `[(o.x)]` and `[o!.x]` are `[o.x]`.
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

  /// `{ a, b: obj.x, c = 1, ...rest } = …`, shared by `Pat` and `AssignTargetPat`.
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

  /// Record the write performed by an assignment target via [`Self::add_target_root_write`].
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

  fn record_known_call(&mut self, callee: &Expr, arguments: &[ExprOrSpread]) {
    let callee = stylex_ast::ast::convertors::normalize_expr(callee);
    if let Some(known) = known_callee(callee) {
      self.known_calls.push(KnownCall {
        callee: known,
        arguments: arguments
          .iter()
          .map(|arg| {
            if arg.spread.is_none() {
              expression_root_id(&arg.expr)
            } else {
              None
            }
          })
          .collect(),
        array_receiver: None,
      });
      return;
    }
    let Some(member) = member_target(callee) else {
      return;
    };
    let Some(method) = member_property_name(&member.prop) else {
      return;
    };
    let callback = if matches!(
      method,
      "map"
        | "forEach"
        | "filter"
        | "some"
        | "every"
        | "find"
        | "findIndex"
        | "flatMap"
        | "reduce"
        | "reduceRight"
    ) {
      arguments
        .first()
        .filter(|arg| arg.spread.is_none())
        .and_then(|arg| known_callee(&arg.expr))
    } else {
      None
    };
    if let (Some(callback), Some(receiver)) = (callback, expression_root_id(&member.obj)) {
      let mapped = if matches!(method, "reduce" | "reduceRight") {
        vec![
          arguments
            .get(1)
            .filter(|arg| arg.spread.is_none())
            .and_then(|arg| expression_root_id(&arg.expr)),
          Some(receiver.clone()),
          None,
          Some(receiver.clone()),
        ]
      } else {
        vec![Some(receiver.clone()), None, Some(receiver.clone())]
      };
      self.known_calls.push(KnownCall {
        callee: callback,
        arguments: mapped,
        array_receiver: Some(receiver),
      });
    }
  }

  /// Record mutations by a method call, plain (`arr.push(1)`) or optional (`arr?.push(1)`).
  fn add_call_mutation_writes(&mut self, callee: &Expr, args: &[ExprOrSpread]) {
    let Some(member_expression) = member_target(callee) else {
      return;
    };

    let Some(property) = member_property_name(&member_expression.prop) else {
      // A computed method name is unknown; recording it would deopt every computed call.
      return;
    };

    // Mutating array methods invalidate the receiver chain's root binding.
    if MUTATING_ARRAY_METHODS.contains(property) {
      self.add_member_root_write(member_expression);
    }

    // `Object.assign(target, …)` and friends mutate their first argument. Matching the `Object`
    // name over-approximates: a shadowed `Object` only costs a deopt.
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

fn expression_root_id(expression: &Expr) -> Option<Id> {
  let expression = stylex_ast::ast::convertors::normalize_expr(expression);
  if let Expr::Ident(ident) = expression {
    return Some(ident.to_id());
  }
  if let Some(member) = member_target(expression) {
    return expression_root_id(&member.obj);
  }
  match expression {
    Expr::TsNonNull(value) => expression_root_id(&value.expr),
    Expr::TsInstantiation(value) => expression_root_id(&value.expr),
    _ => None,
  }
}
fn parameter_id(parameter: &Pat) -> Option<Id> {
  match parameter {
    Pat::Ident(binding) => Some(binding.id.to_id()),
    Pat::Assign(assign) => parameter_id(&assign.left),
    _ => None,
  }
}
fn known_callee(expression: &Expr) -> Option<KnownCallee> {
  match stylex_ast::ast::convertors::normalize_expr(expression) {
    Expr::Ident(ident) => Some(KnownCallee::Binding(ident.to_id())),
    Expr::Arrow(arrow) => Some(KnownCallee::Parameters(
      arrow.params.iter().map(parameter_id).collect(),
    )),
    Expr::Fn(function) => Some(KnownCallee::Parameters(
      function
        .function
        .params
        .iter()
        .map(|param| parameter_id(&param.pat))
        .collect(),
    )),
    _ => None,
  }
}

/// The `MemberExpr` an expression resolves to through parens, TS wrappers and `obj?.x`.
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

/// The static name of a member property (`obj.push`, `obj["push"]`), or `None` if dynamic.
fn member_property_name(property: &MemberProp) -> Option<&str> {
  match property {
    MemberProp::Ident(ident) => Some(ident.sym.as_ref()),
    MemberProp::Computed(computed) => match computed.expr.as_ref() {
      // A lone-surrogate name can't match any method name; treat it as unknown.
      Expr::Lit(Lit::Str(string)) => string.value.as_atom().map(|atom| &**atom),
      _ => None,
    },
    MemberProp::PrivateName(_) => None,
  }
}

impl Visit for ModuleBindingsCollector {
  fn visit_labeled_stmt(&mut self, node: &swc_core::ecma::ast::LabeledStmt) {
    // Function-local labels do not belong to the program's UID namespace.
    if self.nearest_function_scope() == MODULE_SCOPE_SPAN {
      self.bound_names.insert(node.label.sym.to_string());
    }
    node.body.visit_with(self);
  }
  fn visit_break_stmt(&mut self, _: &swc_core::ecma::ast::BreakStmt) {}
  fn visit_continue_stmt(&mut self, _: &swc_core::ecma::ast::ContinueStmt) {}

  fn visit_var_declarator(&mut self, decl: &swc_core::ecma::ast::VarDeclarator) {
    if !matches!(&decl.name, Pat::Ident(_)) {
      self.record_opaque_pattern(
        &decl.name,
        OpaqueDeclarationFault {
          kind: OpaqueDeclarationKind::VariableDeclarator,
          span: decl.span,
        },
      );
    }
    if let Pat::Ident(binding) = &decl.name
      && let Some(initializer) = decl
        .init
        .as_deref()
        .map(stylex_ast::ast::convertors::normalize_expr)
    {
      let id = binding.id.to_id();
      if let Some(source) = expression_root_id(initializer) {
        self
          .binding_aliases
          .entry(id.clone())
          .or_default()
          .insert(source.clone());
        self
          .binding_aliases
          .entry(source.clone())
          .or_default()
          .insert(id.clone());
        if initializer.is_ident() {
          self
            .array_aliases
            .entry(id.clone())
            .or_default()
            .insert(source.clone());
          self
            .array_aliases
            .entry(source)
            .or_default()
            .insert(id.clone());
        }
      }
      match initializer {
        Expr::Array(_) => {
          self.array_bindings.insert(id);
        },
        Expr::Arrow(arrow) => {
          self
            .functions
            .insert(id, arrow.params.iter().map(parameter_id).collect());
        },
        Expr::Fn(function) => {
          self.functions.insert(
            id,
            function
              .function
              .params
              .iter()
              .map(|param| parameter_id(&param.pat))
              .collect(),
          );
        },
        _ => {},
      }
    }
    decl.visit_children_with(self);
  }

  fn visit_ident(&mut self, ident: &Ident) {
    self.bound_names.insert(ident.sym.to_string());
  }

  fn visit_import_decl(&mut self, import_decl: &ImportDecl) {
    // Import locals are bindings in every mode; imports contain no write targets.
    for specifier in &import_decl.specifiers {
      let local = local_binding_of(specifier);

      self.declared_bindings.insert(local.to_id());
      self.bound_names.insert(local.sym.to_string());

      if self.collect_sx_bindings {
        // Import locals never count as a re-binding that shadows another import.
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
    for parameter in &function.params {
      self.record_parameter_fault(&parameter.pat);
    }
    self.scope_stack.push(ScopeFrame {
      span: function.span,
      kind: ScopeKind::Function,
    });
    function.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr) {
    for parameter in &arrow_expr.params {
      self.record_parameter_fault(parameter);
    }
    self.scope_stack.push(ScopeFrame {
      span: arrow_expr.span,
      kind: ScopeKind::Function,
    });
    arrow_expr.visit_children_with(self);
    self.scope_stack.pop();
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr) {
    for parameter in &fn_expr.function.params {
      self.record_parameter_fault(&parameter.pat);
    }
    self.scope_stack.push(ScopeFrame {
      span: fn_expr.function.span,
      kind: ScopeKind::Function,
    });

    if let Some(ident) = &fn_expr.ident {
      // A named function expression's name is bound only inside its body.
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
    self.functions.insert(
      fn_decl.ident.to_id(),
      fn_decl
        .function
        .params
        .iter()
        .map(|param| parameter_id(&param.pat))
        .collect(),
    );
    // Function declarations in modules are block-scoped.
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
    // `delete obj.x` mutates `obj`; `delete someBinding` changes nothing.
    if unary_expression.op == UnaryOp::Delete
      && let Some(member_expression) = member_target(unary_expression.arg.as_ref())
    {
      self.add_member_root_write(member_expression);
    }

    unary_expression.visit_children_with(self);
  }

  fn visit_call_expr(&mut self, call_expression: &CallExpr) {
    if let Callee::Expr(callee) = &call_expression.callee {
      self.record_known_call(callee.as_ref(), &call_expression.args);
      self.add_call_mutation_writes(callee.as_ref(), &call_expression.args);
    }

    call_expression.visit_children_with(self);
  }

  fn visit_opt_chain_expr(&mut self, opt_chain: &OptChainExpr) {
    // `arr?.push(1)` parses as an optional call, not a `CallExpr`, so needs its own hook.
    if let OptChainBase::Call(call) = opt_chain.base.as_ref() {
      self.record_known_call(call.callee.as_ref(), &call.args);
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
    if let Some(receiver) = expression_root_id(&for_of_statement.right) {
      let ids = match &for_of_statement.left {
        ForHead::VarDecl(vars) => vars
          .decls
          .iter()
          .filter_map(|decl| parameter_id(&decl.name))
          .collect(),
        ForHead::Pat(pat) => parameter_id(pat).into_iter().collect(),
        ForHead::UsingDecl(using) => using
          .decls
          .iter()
          .filter_map(|decl| parameter_id(&decl.name))
          .collect(),
      };
      self.iterations.push((ids, receiver));
    }
    if let ForHead::Pat(pattern) = &for_of_statement.left {
      self.add_pattern_writes(pattern);
    }

    for_of_statement.visit_children_with(self);
  }
}

fn module_root_bindings(module: &Module) -> FxHashSet<Id> {
  #[derive(Default)]
  struct PatternBindings(FxHashSet<Id>);
  impl Visit for PatternBindings {
    fn visit_binding_ident(&mut self, binding: &BindingIdent) {
      self.0.insert(binding.id.to_id());
    }
    fn visit_expr(&mut self, _: &Expr) {}
    fn visit_function(&mut self, _: &Function) {}
    fn visit_ts_type(&mut self, _: &swc_core::ecma::ast::TsType) {}
  }
  fn declaration(decl: &Decl, ids: &mut FxHashSet<Id>) {
    match decl {
      Decl::Var(vars) => {
        for variable in &vars.decls {
          let mut pattern = PatternBindings::default();
          variable.name.visit_with(&mut pattern);
          ids.extend(pattern.0);
        }
      },
      Decl::Fn(function) => {
        ids.insert(function.ident.to_id());
      },
      Decl::Class(class) => {
        ids.insert(class.ident.to_id());
      },
      Decl::TsTypeAlias(alias) => {
        ids.insert(alias.id.to_id());
      },
      Decl::TsInterface(interface) => {
        ids.insert(interface.id.to_id());
      },
      Decl::TsEnum(enumeration) => {
        ids.insert(enumeration.id.to_id());
      },
      Decl::TsModule(namespace) => {
        if let TsModuleName::Ident(ident) = &namespace.id {
          ids.insert(ident.to_id());
        }
      },
      Decl::Using(using) => {
        for variable in &using.decls {
          let mut pattern = PatternBindings::default();
          variable.name.visit_with(&mut pattern);
          ids.extend(pattern.0);
        }
      },
    }
  }
  let mut ids = FxHashSet::default();
  for item in &module.body {
    match item {
      ModuleItem::Stmt(swc_core::ecma::ast::Stmt::Decl(decl)) => declaration(decl, &mut ids),
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => declaration(&export.decl, &mut ids),
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
        for specifier in &import.specifiers {
          ids.insert(local_binding_of(specifier).to_id());
        }
      },
      ModuleItem::ModuleDecl(ModuleDecl::TsImportEquals(import)) => {
        ids.insert(import.id.to_id());
      },
      ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => match &export.decl {
        DefaultDecl::Class(class) => {
          if let Some(ident) = &class.ident {
            ids.insert(ident.to_id());
          }
        },
        DefaultDecl::Fn(function) => {
          if let Some(ident) = &function.ident {
            ids.insert(ident.to_id());
          }
        },
        DefaultDecl::TsInterfaceDecl(interface) => {
          ids.insert(interface.id.to_id());
        },
      },
      _ => {},
    }
  }
  ids
}

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_program_impl(&mut self, program: &mut Program) {
    match program {
      Program::Module(module) => self.visit_mut_module_impl(module),
      Program::Script(script) => {
        // A `Script` can't hold an injected import: promote it to a `Module`, run the pipeline, and
        // restore the `Script` if no module declaration was added.
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
          // No `ModuleDecl` here, so every item is a `Stmt`.
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
    self
      .state
      .adopt_module_root_bindings(module_root_bindings(module));
    // Recorded unconditionally: `memoize_module` can re-parse on demand, so key-span lookups
    // need the base even on configurations that skip the memo.
    self.state.set_input_module_base(ModuleBase::of(module));

    // Deep clone; the state decides whether release builds need it (file unreadable from disk).
    self
      .state
      .keep_module_source_copy(module, cfg!(debug_assertions));

    self.discover_module(module);

    if !self.state.has_import_paths() && self.state.atom_imports.is_empty() {
      return;
    }

    // Discovery collects lexical binding writes before CommonJS loaders or sx bindings are
    // classified.

    self.transform_producers(module);
    self.transform_atoms(module);
    self.transform_consumers(module);
    self.finalize_module(module);
  }

  /// Discovery pass: one `Discover` walk that populates import state, rewrites compiled-JSX `sx`
  /// attributes, counts references and pre-fills top-level declarations.
  pub(crate) fn discover_module(&mut self, module: &mut Module) {
    self.state.cycle = TransformationCycle::Discover;

    // Require-loader identity is read during discovery even when sx is disabled.
    let mut collector = if self.state.options.sx_prop_name.is_some() {
      ModuleBindingsCollector::for_sx()
    } else {
      ModuleBindingsCollector::writes_only()
    };
    module.visit_with(&mut collector);
    self
      .state
      .adopt_opaque_declaration_faults(std::mem::take(&mut collector.opaque_declaration_faults));
    self
      .state
      .adopt_binding_writes(collector.take_binding_writes());
    self.state.bound_names = collector.bound_names;
    if self.state.options.sx_prop_name.is_some() {
      self.state.existing_import_sources = collector.import_sources;
      self.state.local_rebinding_scopes = collector.local_rebinding_scopes;
    }

    module.visit_mut_children_with(self);

    if self.state.has_import_paths() {
      fill_top_level_expressions(module, &mut self.state);
      fill_call_positions(module, &mut self.state);
    }
  }

  /// Producer pass: transforms `stylex.create`, `defineVars`, `keyframes`, etc.
  pub(crate) fn transform_producers(&mut self, module: &mut Module) {
    self.state.cycle = TransformationCycle::TransformProducers;
    module.visit_mut_children_with(self);
  }

  /// Consumer pass (`stylex.props` / `stylex.attrs`), then merges pending insertions into the
  /// body. Runtime helpers need `runtime_injection`; hoisted dynamic-style consts always emit.
  pub(crate) fn transform_consumers(&mut self, module: &mut Module) {
    self.state.cycle = TransformationCycle::TransformConsumers;
    module.visit_mut_children_with(self);

    let runtime_injection = self.state.options.runtime_injection.is_some();
    flush_pending_insertions(&mut self.state, &mut module.body, runtime_injection);
  }

  /// Cleanup: materialize deferred JSX spreads, build the decl-reference graph, compute the live
  /// set, then sweep unused declarations in one forward pass.
  pub(crate) fn finalize_module(&mut self, module: &mut Module) {
    // Runs first so the graph sees the final JSX shape.
    mark_style_vars_to_keep(module, &mut self.state);

    // Captured after inlining, so it holds only references that survive into the output.
    build_decl_use_graph(module, &mut self.state);

    // Compute the live set up front for one membership check per declarator.
    self.state.live_set = compute_live_set(&self.state);

    self.state.cycle = TransformationCycle::Finalize;

    module.visit_mut_children_with(self);
  }
}

#[cfg(test)]
#[path = "tests/known_writes.rs"]
mod known_writes;

#[cfg(test)]
#[path = "tests/module_bindings_collector_tests.rs"]
mod tests;
