//! Fixtures shared by the CSS test modules: options, rejection reading, and parity cases.
//!
//! [`unchanged`] and [`same`] both claim the reference compiler spells the value as the case says;
//! rejected values go through [`rejects`]. Run with `cargo nextest`: under `cargo test` the ~140k
//! caught panics make the sweeps take minutes.

use std::{
  any::Any,
  panic::{AssertUnwindSafe, catch_unwind},
};

use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::css::common::normalize_css_property_value;

pub(super) fn default_options() -> StyleXStateOptions {
  StyleXStateOptions::default()
}

/// Defaults with font-size pixel-to-rem conversion switched on.
pub(super) fn rem_enabled_options() -> StyleXStateOptions {
  StyleXStateOptions::default().with_enable_font_size_px_to_rem(true)
}

/// The message of a rejection caught by `catch_unwind`; panics if the call did not reject.
pub(super) fn panic_message<T>(result: Result<T, Box<dyn Any + Send>>) -> String {
  let Err(panic) = result else {
    panic!("expected the call to be rejected, but it returned normally");
  };

  panic
    .downcast_ref::<String>()
    .map(String::as_str)
    .or_else(|| panic.downcast_ref::<&str>().copied())
    .unwrap_or_default()
    .to_string()
}

/// One normalization case; `expected` is what both compilers produce.
pub(super) struct Case {
  pub(super) property: &'static str,
  pub(super) value: &'static str,
  pub(super) expected: &'static str,
}

/// A case both compilers spell as `expected`.
pub(super) const fn same(
  property: &'static str,
  value: &'static str,
  expected: &'static str,
) -> Case {
  Case {
    property,
    value,
    expected,
  }
}

/// A case both compilers return byte for byte.
pub(super) const fn unchanged(property: &'static str, value: &'static str) -> Case {
  same(property, value, value)
}

/// Runs a case table against this compiler; reference parity is checked by the harness.
pub(super) fn check(cases: &[Case], options: &StyleXStateOptions) {
  for case in cases {
    let actual = normalize_css_property_value(case.property, case.value, options);

    assert_eq!(
      actual, case.expected,
      "normalizing `{}: {}`",
      case.property, case.value
    );
  }
}

/// Asserts every value is rejected for `property` with a message containing `expected`.
pub(super) fn rejects(
  property: &str,
  values: &[&str],
  expected: &str,
  options: &StyleXStateOptions,
) {
  for value in values {
    let result = catch_unwind(AssertUnwindSafe(|| {
      normalize_css_property_value(property, value, options)
    }));

    let message = panic_message(result);

    assert!(
      message.contains(expected),
      "expected `{property}: {value}` to be rejected with `{expected}`, got: {message}"
    );
  }
}
