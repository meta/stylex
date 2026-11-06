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
      // Numbers stay numbers; `transform_value` picks the unit per expanded property.
      Expr::Lit(Lit::Num(num)) => Some(TRawValue::Number(num.value)),
      // `null` declares nothing but keeps the property, so a later declaration is unset.
      // Booleans only get here from `keyframes`, `positionTry` and `viewTransitionClass` (no
      // validator) and are dropped. Other non-string literals (e.g. regex) are errors below.
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

  // A lone `var(...)` key names its custom property. Wider than the check in
  // `flatten_raw_style_object.rs`; the reference has both. Both ends are ASCII, so slicing is safe.
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
