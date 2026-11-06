//! Record module declarations and call positions for later compiler phases.

use swc_core::ecma::{
  ast::{
    ArrowExpr, CallExpr, Constructor, Decl, Expr, ExprStmt, ForHead, Function, GetterProp,
    KeyValueProp, Module, ModuleDecl, ModuleItem, Prop, PropName, PropOrSpread, SetterProp, Stmt,
    TsModuleBlock, VarDeclOrExpr, VarDeclarator,
  },
  visit::{Visit, VisitWith},
};

use stylex_ast::ast::convertors::normalize_expr;
use stylex_enums::top_level_expression::TopLevelExpressionKind;
use stylex_structures::top_level_expression::TopLevelExpression;

use crate::{
  call_positions::{CallPositions, Position},
  state_manager::StateManager,
};

/// Records every top-level expression the module declares, so a later phase can find a call by the name it was bound to instead of walking the module again.
pub fn fill_top_level_expressions(module: &Module, state: &mut StateManager) {
  module.body.iter().for_each(|item| match item {
    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
      if let Decl::Var(decl_var) = &export_decl.decl {
        for decl in &decl_var.decls {
          record_top_level_declarator(state, TopLevelExpressionKind::NamedExport, decl);
        }
      }
    },
    ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export_decl))
      if !is_unmatchable_literal(&export_decl.expr) =>
    {
      match export_decl.expr.as_paren() {
        Some(paren) => {
          state.push_top_level_expression(TopLevelExpression(
            TopLevelExpressionKind::DefaultExport,
            paren.expr.as_ref().clone(),
            None,
          ));
        },
        _ => {
          state.push_top_level_expression(TopLevelExpression(
            TopLevelExpressionKind::DefaultExport,
            export_decl.expr.as_ref().clone(),
            None,
          ));
        },
      }
    },
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => {
      for decl in &var.decls {
        record_top_level_declarator(state, TopLevelExpressionKind::Stmt, decl);
      }
    },
    _ => {},
  });
}

/// Record one declarator of a top-level variable declaration, `kind` telling an exported one from a plain statement.
fn record_top_level_declarator(
  state: &mut StateManager,
  kind: TopLevelExpressionKind,
  decl: &VarDeclarator,
) {
  let Some(decl_init) = decl.init.as_ref() else {
    return;
  };

  let Some(ident) = decl.name.as_ident() else {
    return;
  };

  if !is_unmatchable_literal(decl_init) {
    state.push_top_level_expression(TopLevelExpression(
      kind,
      decl_init.as_ref().clone(),
      Some(ident.sym.clone()),
    ));
  }

  fill_state_declarations(state, decl);
}

/// Whether a top-level expression is a literal no reader of the recorded expressions can ask for.
fn is_unmatchable_literal(expr: &Expr) -> bool {
  match expr {
    Expr::Object(object) => {
      !object.props.is_empty()
        && object.props.iter().all(|prop| match prop {
          PropOrSpread::Spread(_) => false,
          PropOrSpread::Prop(prop) => match prop.as_ref() {
            Prop::KeyValue(key_value) => !could_be_compiled_namespace(&key_value.value),
            Prop::Shorthand(_) | Prop::Assign(_) => true,
            Prop::Getter(_) | Prop::Setter(_) | Prop::Method(_) => false,
          },
        })
    },
    _ => matches!(normalize_expr(expr), Expr::Array(_)),
  }
}

/// Whether a property value has a shape a compiled namespace has: a function, or an object literal holding a `$$css` key.
fn could_be_compiled_namespace(value: &Expr) -> bool {
  match value {
    Expr::Lit(_) | Expr::Ident(_) | Expr::Member(_) | Expr::Tpl(_) | Expr::Array(_) => false,
    Expr::Object(object) => object.props.iter().any(|prop| match prop {
      PropOrSpread::Spread(_) => true,
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(KeyValueProp { key, .. }) => match key {
          PropName::Ident(ident) => ident.sym == "$$css",
          PropName::Str(text) => text.value.as_str() == Some("$$css"),
          _ => true,
        },
        Prop::Shorthand(ident) => ident.sym == "$$css",
        _ => true,
      },
    }),
    _ => true,
  }
}

/// Records one declarator, unless the state already holds it.
pub fn fill_state_declarations(state: &mut StateManager, decl: &VarDeclarator) {
  if !state.holds_declaration(decl) {
    state.push_declaration(decl.clone());
  }
}

/// Records where every call in the module is written, so a later phase can ask the position of a call without walking the module again.
/// A pass that transforms `stylex.create` must call this.
pub fn fill_call_positions(module: &Module, state: &mut StateManager) {
  let mut walk = CallPositionWalk::default();

  module.visit_with(&mut walk);

  state.record_call_positions(walk.positions);
}

/// The walk behind [`fill_call_positions`].
#[derive(Default)]
struct CallPositionWalk {
  positions: CallPositions,
  /// How many statements enclose the node the walk is at.
  statement_depth: u32,
  /// Whether a function or a namespace encloses the node.
  below_program_level: bool,
  /// Whether a type assertion encloses the node, which only the call directly under one is read for.
  under_type_assertion: bool,
}

impl CallPositionWalk {
  /// The deepest a call can be and still stand at program level.
  const PROGRAM_LEVEL_DEPTH: u32 = 1;

  fn at_program_level(&self) -> bool {
    self.statement_depth <= Self::PROGRAM_LEVEL_DEPTH && !self.below_program_level
  }

  /// Walks what a function or a namespace holds, with everything in it read as below program level.
  fn enclosed(&mut self, walk: impl FnOnce(&mut Self)) {
    let enclosing = std::mem::replace(&mut self.below_program_level, true);

    walk(self);

    self.below_program_level = enclosing;
  }

  /// Walks one more statement deep.
  fn deeper(&mut self, walk: impl FnOnce(&mut Self)) {
    self.statement_depth += 1;

    walk(self);

    self.statement_depth -= 1;
  }

  /// Walks what a type assertion holds.
  fn asserted(&mut self, walk: impl FnOnce(&mut Self)) {
    let enclosing = std::mem::replace(&mut self.under_type_assertion, true);

    walk(self);

    self.under_type_assertion = enclosing;
  }
}

impl Visit for CallPositionWalk {
  fn visit_stmt(&mut self, stmt: &Stmt) {
    if let Stmt::Expr(ExprStmt { expr, .. }) = stmt
      && let Expr::Call(call) = normalize_expr(expr)
    {
      self.positions.record(call.span, Position::BareStatement);
    }

    self.deeper(|walk| stmt.visit_children_with(walk));
  }

  /// The initializer of a `for` head is a declaration of its own below the loop, and a declaration is a statement.
  fn visit_var_decl_or_expr(&mut self, init: &VarDeclOrExpr) {
    match init {
      VarDeclOrExpr::VarDecl(var_decl) => self.deeper(|walk| var_decl.visit_with(walk)),
      VarDeclOrExpr::Expr(expr) => expr.visit_with(self),
    }
  }

  fn visit_for_head(&mut self, head: &ForHead) {
    match head {
      ForHead::VarDecl(_) | ForHead::UsingDecl(_) => {
        self.deeper(|walk| head.visit_children_with(walk));
      },
      ForHead::Pat(pat) => pat.visit_with(self),
    }
  }

  fn visit_call_expr(&mut self, call: &CallExpr) {
    if self.at_program_level() {
      self.positions.record(call.span, Position::ProgramLevel);
    }

    if self.under_type_assertion {
      self.positions.record(call.span, Position::TypeAsserted);
    }

    let enclosing = std::mem::replace(&mut self.under_type_assertion, false);

    call.visit_children_with(self);

    self.under_type_assertion = enclosing;
  }

  fn visit_ts_as_expr(&mut self, assertion: &swc_core::ecma::ast::TsAsExpr) {
    self.asserted(|walk| assertion.expr.visit_with(walk));
  }

  fn visit_ts_satisfies_expr(&mut self, assertion: &swc_core::ecma::ast::TsSatisfiesExpr) {
    self.asserted(|walk| assertion.expr.visit_with(walk));
  }

  fn visit_ts_type_assertion(&mut self, assertion: &swc_core::ecma::ast::TsTypeAssertion) {
    self.asserted(|walk| assertion.expr.visit_with(walk));
  }

  fn visit_ts_const_assertion(&mut self, assertion: &swc_core::ecma::ast::TsConstAssertion) {
    self.asserted(|walk| assertion.expr.visit_with(walk));
  }

  /// A namespace holds module items, so its statements would read as the statements of the module itself and its exports would read as the module's own.
  fn visit_ts_module_block(&mut self, block: &TsModuleBlock) {
    self.enclosed(|walk| block.visit_children_with(walk));
  }

  fn visit_function(&mut self, function: &Function) {
    self.enclosed(|walk| function.visit_children_with(walk));
  }

  fn visit_arrow_expr(&mut self, arrow: &ArrowExpr) {
    self.enclosed(|walk| arrow.visit_children_with(walk));
  }

  fn visit_constructor(&mut self, constructor: &Constructor) {
    self.enclosed(|walk| constructor.visit_children_with(walk));
  }

  fn visit_getter_prop(&mut self, getter: &GetterProp) {
    self.enclosed(|walk| getter.visit_children_with(walk));
  }

  fn visit_setter_prop(&mut self, setter: &SetterProp) {
    self.enclosed(|walk| setter.visit_children_with(walk));
  }
}
