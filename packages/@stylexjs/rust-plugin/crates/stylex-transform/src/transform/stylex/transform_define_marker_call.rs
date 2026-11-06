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

/// `index`, or the refusal a marker bound to no declarator is reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. The check above the call proves the call is a top-level expression
/// bound to a named export, and a named top-level entry has one writer:
/// `stylex_state::state_writers::record_top_level_declarator` records the
/// declarator in both lists or in neither.
/// `a_top_level_declarator_is_recorded_in_both_lists` measures that, and
/// `the_position_names_the_declarator_the_call_initialises` measures that the
/// position names the right one. `guidelines/stack/RUST.md` describes the
/// allowance.
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

    // The marker's identity is its export name, which the check answers: it
    // proves the call is bound to a named export, so the name is read once
    // there rather than read again off the declarator here.
    //
    // Kept as the interned `Atom` the binding already carries: it only ever
    // feeds `gen_file_based_identifier`, which takes `&str`, so re-allocating
    // it as a `String` would buy nothing.
    let export_name = if is_define_marker_call {
      find_and_validate_stylex_define_marker(call, &mut self.state)
    } else {
      return None;
    };

    // The call also has to be tied back to the declarator it initialises, so
    // the marker object can replace the call there. Nothing in the AST carries
    // that link, and a span-insensitive lookup resolves every `defineMarker()`
    // in the module — they are all the same expression — to the first
    // declarator, hashing every export to one class. The span does carry it.
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

    // The recorded declaration still holds the `defineMarker()` call this
    // returns a marker object in place of. A `when` selector in the same file
    // resolves the marker through that declaration, so it has to see the
    // object rather than the call it can no longer evaluate.
    //
    // The index taken above still addresses that declarator: nothing between
    // here and there takes `&mut self.state` — `get_filename_for_hashing`
    // borrows it shared, and the rest only reads `options` — so the vector
    // cannot have been pushed to or reordered.
    self
      .state
      .set_declaration_init(parent_var_decl_index, marker_obj_ast.clone());

    Some(marker_obj_ast)
  }
}
