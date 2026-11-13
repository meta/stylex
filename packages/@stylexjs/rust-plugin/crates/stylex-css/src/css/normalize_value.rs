//! Rewrites a declaration value into the canonical text that gets hashed.
//!
//! The value is scanned into tokens, a fixed sequence of passes is folded over it, and it is
//! spelled back out; tokens no pass names survive byte for byte. The order is behaviour:
//! timings run before leading-zero (`100ms` -> `0.1s` -> `.1s`), and the caller's injection
//! guard runs after the unclosed-function/string rejections so both compilers report the same
//! fault first.

use postcss_value_parser::{ValueParser, stringify};
use stylex_macros::stylex_panic;
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::css::normalizers::{
  convert_camel_case_values::convert_camel_cased_values, detect_unclosed_fns::detect_unclosed_fns,
  detect_unclosed_strings::detect_unclosed_strings, font_size_px_to_rem::convert_font_size_to_rem,
  leading_zero::normalize_leading_zero, quotes::normalize_quotes, timings::normalize_timings,
  unprefixed_custom_properties::detect_unprefixed_custom_properties,
  whitespace::normalize_whitespace, zero_dimensions::normalize_zero_dimensions,
};

/// One pass over the token list, rewriting or rejecting it for the named property.
type Pass = fn(&mut ValueParser, &str);

/// The rejections upstream also makes; the caller's injection guard runs right after these.
const SHARED_REJECTIONS: [Pass; 2] = [detect_unclosed_fns, detect_unclosed_strings];

/// The passes that always run once the shared rejections have had their say.
const PASSES: [Pass; 7] = [
  detect_unprefixed_custom_properties,
  normalize_whitespace,
  normalize_timings,
  normalize_zero_dimensions,
  normalize_leading_zero,
  normalize_quotes,
  convert_camel_cased_values,
];

/// Normalizes `value`, declared for property `key`, without the structural guards.
///
/// Panics (caught and reported) on an unclosed function or string, an unprefixed custom-property
/// reference, or an empty value. Production code wants
/// [`crate::css::common::normalize_css_property_value`]; this is `pub` for the benches.
pub fn normalize_value(value: &str, key: &str, options: &StyleXStateOptions) -> String {
  normalize_value_guarded(value, key, options, None)
}

/// Normalizes `value`, raising `deferred_refusal` (a guard's prebuilt complaint) after
/// `SHARED_REJECTIONS`.
pub(crate) fn normalize_value_guarded(
  value: &str,
  key: &str,
  options: &StyleXStateOptions,
  deferred_refusal: Option<&str>,
) -> String {
  let mut ast = ValueParser::new(value);

  for pass in SHARED_REJECTIONS {
    pass(&mut ast, key);
  }

  if let Some(complaint) = deferred_refusal {
    stylex_panic!("{}", complaint);
  }

  for pass in PASSES {
    pass(&mut ast, key);
  }

  if options.enable_font_size_px_to_rem {
    convert_font_size_to_rem(&mut ast, key);
  }

  // `stringify` avoids the extra copy `to_string` would make.
  stringify(&ast.nodes)
}
