use std::rc::Rc;

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_macros::stylex_panic;
use stylex_utils::identifier::gen_file_based_identifier;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::{
  StyleXTransform,
  shared::utils::{
    core::js_to_ast::convert_values_to_ast,
    validators::{find_and_validate_stylex_define_marker, is_define_marker_call},
  },
};
use stylex_constants::constants::{
  api_names::STYLEX_DEFINE_MARKER,
  common::COMPILED_KEY,
  messages::{cannot_generate_hash, unbound_call_value},
};
use stylex_state::flat_compiled_styles_value::FlatCompiledStylesValue;
use stylex_utils::hash::create_hash;

/// `index`, or the refusal for a marker bound to no declarator.
/// Excluded from coverage: only picks between precomputed answers (`guidelines/stack/RUST.md`).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_unbound_marker(index: Option<usize>) -> usize {
  match index {
    Some(index) => index,
    None => stylex_panic!("{}", unbound_call_value(STYLEX_DEFINE_MARKER)),
  }
}

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_define_marker_call(&mut self, call: &CallExpr) -> Option<Expr> {
    let is_define_marker_call = is_define_marker_call(call, &self.state);

    // The check already proved the call is bound to a named export; reuse that name.
    let export_name = if is_define_marker_call {
      find_and_validate_stylex_define_marker(call, &mut self.state)
    } else {
      return None;
    };

    // Locate the declarator by span: every `defineMarker()` is the same expression, so a
    // span-insensitive lookup would resolve them all to the first declarator.
    let parent_var_decl_index =
      or_refuse_unbound_marker(self.state.find_call_declaration_index_by_span(call));

    let file_name = match self
      .state
      .get_filename_for_hashing(&mut FxHashMap::default())
    {
      Some(name) => name,
      None => stylex_panic!("{}", cannot_generate_hash(STYLEX_DEFINE_MARKER)),
    };

    let export_id = gen_file_based_identifier(&file_name, &export_name, None);

    let hash = create_hash(&export_id);
    let mut id = String::with_capacity(self.state.options.class_name_prefix.len() + hash.len());
    id.push_str(&self.state.options.class_name_prefix);
    id.push_str(&hash);

    let mut marker_result = IndexMap::new();

    marker_result.insert(id.clone(), Rc::new(FlatCompiledStylesValue::String(id)));

    marker_result.insert(
      COMPILED_KEY.to_string(),
      Rc::new(FlatCompiledStylesValue::Bool(true)),
    );
    let marker_obj_ast = convert_values_to_ast(&marker_result);

    // Replace the recorded call with the marker object so same-file `when` selectors resolve it.
    // The index is still valid: nothing since took `&mut self.state`.
    self
      .state
      .set_declaration_init(parent_var_decl_index, marker_obj_ast.clone());

    Some(marker_obj_ast)
  }
}
