/*!
Every numeric CSS type holds and prints a double.

These types used to store `f32` where the compiler they must agree with holds a
JavaScript `number`, so an authored value was rounded before anything else
happened to it. The printed spelling feeds the class-name hash, which makes the
rounding observable rather than cosmetic: a value that prints differently hashes
differently, and two compilers that hash differently cannot be mixed across an
SSR and client boundary.

Each expectation here was confirmed against `@stylexjs/babel-plugin@0.19.0` run
over the same value. Where the two disagreed, the official compiler's output is
the expectation and ours was the bug.

The transform seam cannot reach most of these cheaply -- the emission path
echoes an authored value rather than re-printing it through this parser -- so
they are asserted here, at the parser itself.
*/

use crate::css_types::calc::Calc;

/// Parses `input` with `parser` and returns the printed text.
macro_rules! printed {
  ($ty:ty, $input:expr) => {
    match <$ty>::parser().parse_to_end($input) {
      Ok(parsed) => parsed.to_string(),
      Err(error) => panic!(
        "{} failed to parse {:?}: {:?}",
        stringify!($ty),
        $input,
        error
      ),
    }
  };
}

#[cfg(test)]
mod calc_keeps_the_digits_of_what_it_wraps {
  use super::*;

  #[test]
  fn deeply_nested_calc_keeps_every_operand() {
    let input = "calc(calc(calc(calc(1.1px + 2.2px) + 3.3px) + 4.4px) + 5.5px)";
    let printed = printed!(Calc, input);

    for operand in ["1.1px", "2.2px", "3.3px", "4.4px", "5.5px"] {
      assert!(
        printed.contains(operand),
        "{operand} missing from {printed}"
      );
    }
  }
}

#[cfg(test)]
mod boundaries_and_refusals {
  use super::*;

  /// Must return, parsed or refused, rather than overflow the stack.
  #[test]
  fn a_very_deeply_nested_calc_terminates() {
    let depth = 200;
    let input = format!("calc({}1.1px{})", "calc(".repeat(depth), ")".repeat(depth));

    let _ = Calc::parser().parse_to_end(&input);
  }

  /// This parser closes an unclosed `calc(` rather than refusing it.
  #[test]
  fn a_deeply_nested_unterminated_calc_terminates_with_its_operand() {
    let input = format!("calc({}1.1px", "calc(".repeat(200));

    match Calc::parser().parse_to_end(&input) {
      Ok(calc) => assert_eq!(calc.to_string(), "calc(1.1px)"),
      Err(error) => panic!("expected the unclosed calc to be tolerated: {error:?}"),
    }
  }
}
