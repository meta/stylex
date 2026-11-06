//! Ported normalizer 9 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::ValueParser;
use stylex_constants::constants::common::ROOT_FONT_SIZE;
use stylex_utils::number::{parse_js_float, to_js_string};

use super::dimensions::walk_dimensions;

/// Restates a font size given in pixels as a multiple of the root font size.
///
/// Appended to the list only when the option asking for it is on, and applied
/// only to `fontSize`. It runs last, after the leading zero has already been
/// stripped from everything else, which is why `8px` comes out as `0.5rem`
/// rather than `.5rem`: nothing runs after it to take that zero off.
pub fn convert_font_size_to_rem(ast: &mut ValueParser, key: &str) {
  if key != "fontSize" {
    return;
  }

  walk_dimensions(ast, |_word, dimension| {
    if dimension.unit != "px" {
      return None;
    }

    // `unit` splits on a leading number, so the number half always reads
    // back. The reference implementation does not guard this at all -- a
    // number that failed to read would be spelled into the value as `NaNrem`
    // -- so the same spelling stands in for the case, rather than a branch
    // that no input can take and no test can cover.
    let number = parse_js_float(dimension.number).unwrap_or(f64::NAN);

    Some(format!(
      "{}rem",
      to_js_string(number / f64::from(ROOT_FONT_SIZE))
    ))
  });
}
