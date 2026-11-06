use postcss_value_parser::{ValueParser, stringify};
use stylex_macros::stylex_panic;

use crate::css::common::build_error_css_rule;

pub mod convert_camel_case_values;
pub mod detect_unclosed_fns;
pub mod detect_unclosed_strings;
mod dimensions;
pub mod font_size_px_to_rem;
pub mod leading_zero;
pub mod quotes;
pub mod timings;
pub mod unprefixed_custom_properties;
pub mod whitespace;
pub mod zero_dimensions;

/// Rejects the value under normalization, quoting the rule it would have built.
///
/// `#[track_caller]` so the diagnostic points at the rejecting pass.
#[track_caller]
pub(super) fn reject_value(ast: &ValueParser, key: &str, message: &str) -> ! {
  let value = stringify(&ast.nodes);

  stylex_panic!(
    "{}, css rule: {}",
    message,
    build_error_css_rule(key, &value)
  )
}
