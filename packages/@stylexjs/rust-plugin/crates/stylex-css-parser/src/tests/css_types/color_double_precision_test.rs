/*!
Every colour channel holds and prints a double.

The channels used to store `f32` while every other numeric CSS type in the
crate held a double, so an authored alpha or saturation was rounded before
anything else happened to it. The printed spelling feeds the class-name hash,
which makes the rounding observable rather than cosmetic.

Each expectation here was confirmed against `@stylexjs/babel-plugin@0.19.0` run
over the same colour. The official compiler carries a colour's authored digits
through to the emitted rule untouched -- `rgba(255, 0, 0, 0.123456789012345)`
comes out with all fifteen of them -- so a channel that cannot hold those digits
is a divergence, and every assertion below is that this parser holds them too.

The companion file `double_precision_test.rs` covers the dimension and number
types.
*/

use crate::css_types::color::Color;

#[cfg(test)]
mod boundaries_and_refusals {
  use super::*;

  #[test]
  fn a_deeply_nested_colour_function_terminates() {
    let depth = 200;
    let input = format!(
      "rgba({}255{}, 0, 0, 0.5)",
      "rgba(".repeat(depth),
      ")".repeat(depth)
    );

    assert!(Color::parse().parse_to_end(&input).is_err());
  }
}

#[cfg(test)]
mod modern_space_boundaries_and_refusals {
  use super::*;

  // Constructed directly by `a_nan_alpha_prints_no_tail`; no value tokenizes to `NaN`.

  #[test]
  fn a_deeply_nested_modern_colour_function_terminates() {
    let depth = 200;
    let input = format!(
      "oklch({}0.7{} 0.1 200deg)",
      "oklch(".repeat(depth),
      ")".repeat(depth)
    );

    assert!(Color::parse().parse_to_end(&input).is_err());
  }
}
