//! Ported normalizer 9 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::ValueParser;
use stylex_constants::constants::common::ROOT_FONT_SIZE;
use stylex_utils::number::{parse_js_float, to_js_string};

use super::dimensions::walk_dimensions;

/// Restates a `fontSize` in pixels as rem; runs last, so `8px` stays `0.5rem`.
pub fn convert_font_size_to_rem(ast: &mut ValueParser, key: &str) {
  if key != "fontSize" {
    return;
  }

  walk_dimensions(ast, |_word, dimension| {
    if dimension.unit != "px" {
      return None;
    }

    // Matches upstream: an unreadable number would spell `NaNrem`.
    let number = parse_js_float(dimension.number).unwrap_or(f64::NAN);

    Some(format!(
      "{}rem",
      to_js_string(number / f64::from(ROOT_FONT_SIZE))
    ))
  });
}
