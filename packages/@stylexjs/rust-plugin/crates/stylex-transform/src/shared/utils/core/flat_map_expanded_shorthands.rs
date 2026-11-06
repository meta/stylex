use std::borrow::Cow;

use log::warn;
use stylex_ast::ast::convertors::convert_lit_to_string;
use stylex_css::order::structures::{
  application_order::ApplicationOrder, legacy_expand_shorthands_order::LegacyExpandShorthandsOrder,
  property_specificity_order::PropertySpecificityOrder,
};
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
use stylex_macros::stylex_panic;
use stylex_structures::pre_rule_value::PreRuleValue;
use stylex_structures::{
  order::Order, order_pair::OrderPair, raw_value::TRawValue,
  stylex_state_options::StyleXStateOptions,
};
use swc_core::ecma::ast::{Expr, Lit};

/// The declarations one authored property stands for.
///
/// The key is borrowed where the caller has one to lend. A shorthand is read by
/// name and then expanded into keys of its own, so the name is only copied on
/// the one path that keeps it: a property that stands for itself.
pub(crate) fn flat_map_expanded_shorthands(
  obj_entry: (Cow<'_, str>, PreRuleValue),
  options: &StyleXStateOptions,
) -> Vec<OrderPair> {
  let (key, raw_value) = obj_entry;

  let value = match raw_value {
    PreRuleValue::Raw(value) => Some(value),
    PreRuleValue::Vec(_) => {
      let msg = "Cannot use fallbacks for shorthands. Use the expansion instead.";
      match options.property_validation_mode {
        PropertyValidationMode::Throw => {
          stylex_panic!("{}", msg);
        },
        PropertyValidationMode::Warn => {
          warn!("{}", msg);
          return vec![];
        },
        PropertyValidationMode::Silent => {
          return vec![];
        },
      }
    },
    PreRuleValue::Expr(expr) => match expr {
      // A numeric literal stays a number: the unit suffix is decided later, per
      // expanded property, by `transform_value`.
      Expr::Lit(Lit::Num(num)) => Some(TRawValue::Number(num.value)),
      // Two literals reach here with nothing to declare, for two different
      // reasons.
      //
      // `null` is an absent value everywhere: it declares nothing, and the
      // property it expands to survives carrying that absence, which is how a
      // later declaration of the same property gets unset rather than
      // shadowed.
      //
      // A boolean only reaches here from the calls that have no value
      // validator in front of them -- `keyframes`, `positionTry` and
      // `viewTransitionClass` -- where it is dropped. Inside `create` the
      // validator refuses it first, so this arm is not the decision about a
      // boolean; `is_style_value_literal` in `validators.rs` is.
      //
      // Every other literal that spells no string is a different thing again: a
      // regular expression is not an absent value, it is an unusable one, and
      // saying nothing about it would drop a declaration the author meant to
      // write.
      Expr::Lit(Lit::Null(_) | Lit::Bool(_)) => None,
      Expr::Lit(lit) => Some(TRawValue::String(match convert_lit_to_string(&lit) {
        Some(s) => s,
        None => stylex_panic!("Failed to convert literal value to string in shorthand expansion."),
      })),
      _ => {
        let msg = "Cannot use expressions for shorthands. Use the expansion instead.";
        match options.property_validation_mode {
          PropertyValidationMode::Throw => {
            stylex_panic!("{}", msg);
          },
          PropertyValidationMode::Warn => {
            warn!("{}", msg);
            return vec![];
          },
          PropertyValidationMode::Silent => {
            return vec![];
          },
        }
      },
    },
    PreRuleValue::Null => None,
  };

  // A key that is one variable reference and nothing else names the custom
  // property it declares. This is the wider of the two questions the compiler
  // asks about such a key: it takes any name, where the one in
  // `flatten_raw_style_object.rs` takes only a narrow class. Both are kept,
  // because the reference implementation asks both, and a key the narrow one
  // passes over reaches its property here.
  //
  // The slice is safe to count in bytes: both ends are proven ASCII by the two
  // checks beside it.
  let key = if key.starts_with("var(") && key.ends_with(')') {
    Cow::Owned(key[4..key.len() - 1].to_string())
  } else {
    key
  };

  let expansion_fn = match &options.style_resolution {
    StyleResolution::ApplicationOrder => ApplicationOrder::get_expansion_fn(key.as_ref()),
    StyleResolution::LegacyExpandShorthands => {
      LegacyExpandShorthandsOrder::get_expansion_fn(key.as_ref())
    },
    StyleResolution::PropertySpecificity => {
      PropertySpecificityOrder::get_expansion_fn(key.as_ref())
    },
  };

  if let Some(expansion_fn) = expansion_fn {
    return match (expansion_fn)(value) {
      Ok(expanded) => expanded,
      Err(error_message) => match options.property_validation_mode {
        PropertyValidationMode::Throw => {
          stylex_panic!("{}", error_message);
        },
        PropertyValidationMode::Warn => {
          warn!("{}", error_message);
          vec![]
        },
        PropertyValidationMode::Silent => {
          vec![]
        },
      },
    };
  }

  vec![OrderPair(key.into_owned().into(), value)]
}

#[cfg(test)]
#[path = "tests/flat_map_expanded_shorthands_tests.rs"]
mod tests;
