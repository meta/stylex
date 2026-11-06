use std::rc::Rc;

use stylex_ast::ast::convertors::{convert_key_value_to_str, get_key_values_from_object};
use stylex_macros::stylex_panic;

use stylex_constants::constants::messages::{EXPORT_ID_NOT_SET, VALUES_MUST_BE_OBJECT};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  folded_value::folded_value,
  state_manager::StateManager,
  types::{FlatCompiledStyles, InjectableStylesMap},
};
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableConstStyle,
};
use stylex_utils::hash::create_authored_or_hashed_key;

pub(crate) fn stylex_define_consts(
  constants: &EvaluateResultValue,
  state: &mut StateManager,
) -> (FlatCompiledStyles, InjectableStylesMap) {
  let Some(constants) = constants.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let class_name_prefix = state.options.class_name_prefix.clone();
  let export_id = match state.export_id.clone() {
    Some(id) => id,
    None => stylex_panic!("{}", EXPORT_ID_NOT_SET),
  };

  let key_values = get_key_values_from_object(constants);

  let mut js_output = FlatCompiledStyles::with_capacity(key_values.len());
  let mut injectable_types = InjectableStylesMap::with_capacity(key_values.len());

  for key_value in key_values.iter() {
    let key = convert_key_value_to_str(key_value);
    // The constant keeps the kind the author gave it. A reader of the answer
    // and a reader of the injected rule both see what was written, rather than
    // text that each of them has to guess a kind back out of.
    let value = Rc::new(folded_value(&key_value.value));

    let const_key = create_authored_or_hashed_key(&class_name_prefix, &export_id, &key);

    injectable_types.insert(
      const_key.clone().into(),
      Rc::new(InjectableStyleKind::Const(InjectableConstStyle {
        ltr: String::default(),
        rtl: None,
        priority: Some(0.0),
        const_key,
        const_value: value.to_json_text(),
      })),
    );

    js_output.insert(key, value);
  }

  (js_output, injectable_types)
}
