//! Fixtures shared by the CSS test modules: options and rejection reading.

use std::any::Any;

use stylex_structures::stylex_state_options::StyleXStateOptions;

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
