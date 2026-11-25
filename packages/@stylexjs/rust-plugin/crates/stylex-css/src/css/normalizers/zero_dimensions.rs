//! Ported normalizer 5 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::{NodeKind, ValueParser, unit};

const ANGLES: [&str; 4] = ["deg", "grad", "turn", "rad"];
const TIMINGS: [&str; 2] = ["ms", "s"];
const FRACTION: &str = "fr";
const PERCENTAGE: &str = "%";

/// Canonicalizes a zero with a unit: angles to `0deg`, durations to `0s`, other units dropped
/// outside functions only.
///
/// The function window extends at every function (#1750); URL bodies are skipped (#1983). Custom
/// properties are exempt.
pub fn normalize_zero_dimensions(ast: &mut ValueParser, key: &str) {
  if key.starts_with("--") {
    return;
  }

  let mut end_function = 0;

  ast.walk(
    |node, _| {
      if node.kind == NodeKind::Function {
        end_function = end_function.max(node.source_end_index);
      }

      if end_function > 0 && node.source_index > end_function {
        end_function = 0;
      }
      if node.kind == NodeKind::Function && node.value.eq_ignore_ascii_case("url") {
        return false;
      }

      if node.kind != NodeKind::Word {
        return true;
      }

      let dimension = match unit(&node.value) {
        Some(dimension) => dimension,
        None => return true,
      };

      if dimension.number != "0" {
        return true;
      }

      // Units are ASCII case-insensitive (upstream #2000): `0MS` is still a duration and `0FR`
      // a flex value, and dropping their unit would change the value.
      let unit = dimension.unit.to_lowercase();
      if ANGLES.contains(&unit.as_str()) {
        node.value = "0deg".to_owned();
      } else if TIMINGS.contains(&unit.as_str()) {
        node.value = "0s".to_owned();
      } else if unit == FRACTION {
        node.value = "0fr".to_owned();
      } else if unit == PERCENTAGE {
        node.value = "0%".to_owned();
      } else if end_function == 0 {
        node.value = "0".to_owned();
      }

      true
    },
    false,
  );
}
