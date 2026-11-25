mod visit_mut_computed_prop_name;
mod visit_mut_decl;
mod visit_mut_export_decl;
mod visit_mut_export_default_expr;
mod visit_mut_expr;
mod visit_mut_import_decl;
mod visit_mut_jsx_attr_or_spread;
mod visit_mut_jsx_opening_element;
mod visit_mut_member_prop;
mod visit_mut_module;
mod visit_mut_module_items;
mod visit_mut_named_export;
mod visit_mut_prop_name;
mod visit_mut_stmt;
mod visit_mut_stmts;
mod visit_mut_var_declarator;
mod visit_mut_var_declarators;

use super::StyleXTransform;
use swc_core::{
  common::comments::Comments,
  ecma::{
    ast::{
      ComputedPropName, Decl, ExportDecl, ExportDefaultExpr, Expr, ImportDecl, JSXOpeningElement,
      MemberProp, Module, ModuleItem, NamedExport, Program, PropName, Stmt, VarDeclarator,
    },
    visit::{VisitMut, noop_visit_mut_type},
  },
};

impl<C> VisitMut for StyleXTransform<C>
where
  C: Comments,
{
  noop_visit_mut_type!();

  fn visit_mut_program(&mut self, program: &mut Program) {
    self.visit_mut_program_impl(program);
  }

  fn visit_mut_module(&mut self, module: &mut Module) {
    self.visit_mut_module_impl(module);
  }

  fn visit_mut_module_items(&mut self, module_items: &mut Vec<ModuleItem>) {
    self.visit_mut_module_items_impl(module_items);
  }

  fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
    self.visit_mut_import_decl_impl(import_decl);
  }

  fn visit_mut_var_declarators(&mut self, var_declarators: &mut Vec<VarDeclarator>) {
    self.visit_mut_var_declarators_impl(var_declarators);
  }

  fn visit_mut_var_declarator(&mut self, var_declarator: &mut VarDeclarator) {
    self.visit_mut_var_declarator_impl(var_declarator);
  }

  fn visit_mut_export_decl(&mut self, export_decl: &mut ExportDecl) {
    self.visit_mut_export_decl_impl(export_decl);
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    self.visit_mut_stmts_impl(stmts);
  }

  fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
    self.visit_mut_stmt_impl(stmt);
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    self.visit_mut_expr_impl(expr);
  }

  fn visit_mut_prop_name(&mut self, prop_name: &mut PropName) {
    self.visit_mut_prop_name_impl(prop_name);
  }

  fn visit_mut_member_prop(&mut self, member_prop: &mut MemberProp) {
    self.visit_mut_member_prop_impl(member_prop);
  }

  fn visit_mut_computed_prop_name(&mut self, computed_prop_name: &mut ComputedPropName) {
    self.visit_mut_computed_prop_name_impl(computed_prop_name);
  }

  fn visit_mut_export_default_expr(&mut self, export_default_expr: &mut ExportDefaultExpr) {
    self.visit_mut_export_default_expr_impl(export_default_expr);
  }

  fn visit_mut_named_export(&mut self, named_export: &mut NamedExport) {
    self.visit_mut_named_export_impl(named_export);
  }

  fn visit_mut_decl(&mut self, decl: &mut Decl) {
    self.visit_mut_decl_impl(decl);
  }

  fn visit_mut_jsx_opening_element(&mut self, jsx_opening_element: &mut JSXOpeningElement) {
    self.visit_mut_jsx_opening_element_impl(jsx_opening_element);
  }

  fn visit_mut_jsx_attr_or_spreads(
    &mut self,
    jsx_attrs: &mut Vec<swc_core::ecma::ast::JSXAttrOrSpread>,
  ) {
    self.visit_mut_jsx_attr_or_spreads_impl(jsx_attrs);
  }
}
