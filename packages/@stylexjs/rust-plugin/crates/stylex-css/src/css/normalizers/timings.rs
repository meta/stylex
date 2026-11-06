//! Ported normalizer 4 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::ValueParser;
use stylex_utils::number::{parse_js_float, to_js_string};

use super::dimensions::walk_dimensions;

/// Rewrites a duration of ten milliseconds or more as seconds.
///
/// Runs before the leading-zero normalizer, and the order is load-bearing:
/// `100ms` becomes `0.1s` here and `.1s` there. Swapping them leaves the
/// leading zero in place and changes the class name.
///
/// Anything under ten milliseconds is left alone, so `9ms` stays `9ms` — the
/// conversion would spell it `.009s`, which is longer than what it replaced.
pub fn normalize_timings(ast: &mut ValueParser, _key: &str) {
  walk_dimensions(ast, |word, dimension| {
    if dimension.unit != "ms" {
      return None;
    }

    // `unit` splits on a leading number, so the word always reads back as one.
    // The reference implementation does not guard this either — a number that
    // failed to read would compare false against `10` and be spelled into the
    // value as `NaNs` — so the same spelling stands in for the case, rather
    // than a branch that no input can take and no test can cover. The sibling
    // pass [`super::font_size_px_to_rem`] answers it the same way.
    let value = parse_js_float(word).unwrap_or(f64::NAN);

    if value < 10.0 {
      return None;
    }

    Some(format!("{}s", to_js_string(value / 1000.0)))
  });
}
