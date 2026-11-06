//! Ported normalizer 6 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::{NodeKind, ValueParser, unit};
use stylex_utils::number::{parse_js_float, to_js_string};

/// Strips the zero in front of a decimal point: `0.5px` becomes `.5px`.
///
/// Values in `[0, 1)` only, re-spelled via JS number formatting (`0.50px` -> `.5px`).
pub fn normalize_leading_zero(ast: &mut ValueParser, _key: &str) {
  ast.walk(
    |node, _| {
      if node.kind == NodeKind::Function && node.value.eq_ignore_ascii_case("url") {
        return false;
      }
      if node.kind != NodeKind::Word {
        return true;
      }

      let value = match parse_js_float(&node.value) {
        Some(value) => value,
        None => return true,
      };

      let dimension = unit(&node.value);

      // Two comparisons to mirror upstream; a negative decimal keeps its zero.
      #[allow(clippy::manual_range_contains)]
      if value < 1.0 && value >= 0.0 {
        // Matches upstream: `\u{a0}0.5px` parses as a number but `unit` splits nothing, so the
        // unit is dropped.
        let trailing_unit = dimension
          .map(|dimension| dimension.unit)
          .unwrap_or_default();

        let respelled = format!(
          "{}{}",
          to_js_string(value).replacen("0.", ".", 1),
          trailing_unit
        );

        node.value = respelled;
      }

      true
    },
    false,
  );
}
