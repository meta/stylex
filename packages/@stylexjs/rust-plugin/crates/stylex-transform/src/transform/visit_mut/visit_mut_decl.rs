use swc_core::{
  common::comments::Comments,
  ecma::{ast::Decl, visit::VisitMutWith},
};

use crate::StyleXTransform;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_decl_impl(&mut self, decl: &mut Decl) {
    match &decl {
      Decl::Class(class_decl) => {
        self
          .state
          .add_class_name_declaration(class_decl.ident.clone());
      },
      Decl::Fn(fn_decl) => {
        self
          .state
          .add_function_name_declaration(fn_decl.ident.clone());
      },
      _ => {},
    }

    decl.visit_mut_children_with(self);
  }
}
