//! Ported normalizer 4 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::ValueParser;
use stylex_utils::number::{parse_js_float, to_js_string};

use super::dimensions::walk_dimensions_outside_urls;

/// Rewrites a duration of `10ms` or more as seconds; runs before the leading-zero pass.
pub fn normalize_timings(ast: &mut ValueParser, _key: &str) {
  walk_dimensions_outside_urls(ast, |word, dimension| {
    if dimension.unit != "ms" {
      return None;
    }

    // Matches upstream: an unreadable number compares false and would spell `NaNs`.
    let value = parse_js_float(word).unwrap_or(f64::NAN);

    if value < 10.0 {
      return None;
    }

    Some(format!("{}s", to_js_string(value / 1000.0)))
  });
}
