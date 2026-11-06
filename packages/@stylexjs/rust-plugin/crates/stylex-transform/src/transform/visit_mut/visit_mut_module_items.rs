use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_state::state_writers::fill_state_declarations;
use swc_core::{
  common::{DUMMY_SP, comments::Comments},
  ecma::{
    ast::{Decl, ExportDecl, ImportDecl, ModuleDecl, ModuleItem, Pat, Stmt, Str},
    visit::VisitMutWith,
  },
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;
use stylex_regex::regex::STYLEX_CONSTS_IMPORT_REGEX;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_module_items_impl(&mut self, module_items: &mut Vec<ModuleItem>) {
    match self.state.cycle {
      TransformationCycle::Discover => {
        // Pre-fill `state.declarations` for top-level ident var decls so any
        // subsequent lookup during the descent (or in later phases) can find
        // them. The visit_mut_var_declarator hook will re-fill them
        // per-decl, but `fill_state_declarations` is idempotent.
        module_items.iter().for_each(|module_item| {
          if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = module_item {
            var_decl.decls.iter().for_each(|decl| {
              if let Pat::Ident(_) = &decl.name {
                fill_state_declarations(&mut self.state, decl);
              }
            });
          }
        });

        // Single descent: discovers stylex imports, transforms compiled-JSX
        // sx attributes, and accumulates ident / member-expr counts.
        module_items.visit_mut_children_with(self);

        if !self.state.has_import_paths() {
          return;
        }

        if self.state.options.inject_stylex_side_effects {
          let side_effect_imports: Vec<_> = module_items
            .iter()
            .filter_map(|module_item| {
              if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
                let source_path = convert_atom_to_string(&import_decl.src.value);
                STYLEX_CONSTS_IMPORT_REGEX
                  .is_match(&source_path)
                  .unwrap_or(false)
                  .then(|| {
                    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                      span: DUMMY_SP,
                      specifiers: vec![],
                      src: Box::new(Str {
                        span: DUMMY_SP,
                        value: import_decl.src.value.clone(),
                        raw: None,
                      }),
                      type_only: false,
                      with: None,
                      phase: Default::default(),
                    }))
                  })
              } else {
                None
              }
            })
            .collect();

          module_items.extend(side_effect_imports);
        }
      },
      TransformationCycle::TransformProducers | TransformationCycle::TransformConsumers => {
        // Hoisted/queued items are merged once after the consumer walk
        // completes via `flush_pending_insertions`; the per-cycle
        // module-items hook only needs to descend into children.
        module_items.visit_mut_children_with(self);
      },
      TransformationCycle::Finalize => {
        // We need it twice for a clear dead code after declaration transforms
        module_items.visit_mut_children_with(self);

        // We remove `Stmt::Empty` from the statement list.
        // This is optional, but it's required if you don't want extra `;` in output.
        module_items.retain(|module_item| {
          !matches!(module_item, ModuleItem::Stmt(Stmt::Empty(..)))
            && !matches!(module_item, ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl: Decl::Var(var), .. })) if var.decls.is_empty())
        });
      },
    }
  }
}
