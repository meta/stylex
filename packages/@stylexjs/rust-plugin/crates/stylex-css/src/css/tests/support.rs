//! Fixtures shared by the CSS test modules.
//!
//! Value normalization is asserted from more than one module — the general
//! coverage in `common_test`, and the harness-verdicted coverage in
//! `value_normalization_parity_test`, `spacing_repair_parity_test` and
//! `corpus_normalization_test` — and all of them need the same option objects,
//! the same way of reading a rejection, and the same way of recording a
//! reference-compiler verdict. Kept here so a change to how the compiler is
//! configured, how it reports a rejection, or how a verdict is spelled lands in
//! one place.
//!
//! A case is built by one of two constructors: [`unchanged`] when the value
//! comes back as written, [`same`] when it is rewritten. Both carry the same
//! claim about the reference compiler — that it spells the value the way the
//! case says — because after the normalization pipeline was replaced there is
//! no value in the corpus the two compilers spell differently.
//!
//! There was a third, `diverges`, for a case the two disagreed on. It is gone
//! because nothing could construct it any more, and an empty vocabulary reads
//! as a claim that nothing was ever checked. A future divergence is not
//! recorded by reviving it: the harness is the oracle, and a case it reports as
//! divergent is a defect to fix, not a spelling to enshrine. Values the
//! compiler rejects have no spelling to compare and go through [`rejects`]
//! instead.
//!
//! Run this crate with `cargo nextest`. Under plain `cargo test` the sweeps
//! here route on the order of 140k caught panics through libtest's output
//! buffering, which takes minutes rather than seconds.

use std::{
  any::Any,
  panic::{AssertUnwindSafe, catch_unwind},
};

use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::css::common::normalize_css_property_value;

/// The compiler's own defaults, which is what almost every case runs under.
pub(super) fn default_options() -> StyleXStateOptions {
  StyleXStateOptions::default()
}

/// Defaults with font-size pixel-to-rem conversion switched on.
pub(super) fn rem_enabled_options() -> StyleXStateOptions {
  StyleXStateOptions::default().with_enable_font_size_px_to_rem(true)
}

/// The text of a rejection, from the result of a `catch_unwind` around a call
/// expected to reject.
///
/// A rejection is raised as a panic carrying a message, and asserting on that
/// message is the point: `is_err()` alone passes on *any* panic, including one
/// from an unrelated bug, so a test written that way keeps passing after the
/// behaviour it guards has gone.
///
/// Panics when the call did not reject at all, since a caller reaching for the
/// message has already decided that it should have.
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

/// One normalization case: an input, and the declaration text both this
/// compiler and the reference compiler produce for it.
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
///
/// By a wide margin the common shape: most of what these modules assert is that
/// a value is *not* rewritten. Writing that as [`same`] means spelling the value
/// out twice, which invites the two copies to drift and tells a reader nothing
/// the input did not already say — so "unchanged" is stated once, as itself.
pub(super) const fn unchanged(property: &'static str, value: &'static str) -> Case {
  same(property, value, value)
}

/// Runs a case table.
///
/// The assertion is only against this compiler; the claim about the reference
/// compiler is checked by running the harness, not by running these tests.
/// Nothing here can notice the reference compiler changing — only a fresh
/// harness run does that, which is why the parity modules say how to do one.
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

/// Asserts that every value in `values` is rejected for `property`, with a
/// diagnostic containing `expected`.
///
/// A rejection is raised as a panic, so it has no spelling for a [`Case`] to
/// compare and cannot go in a case table. It still has to be asserted on the
/// message rather than on the mere fact of a panic — `is_err()` alone passes on
/// any panic at all, including one from an unrelated bug elsewhere in the
/// pipeline, so a test written that way keeps passing after the guard it
/// watches has gone.
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
