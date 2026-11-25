use ctor::ctor;

/// Prepares the whole test binary before any test runs.
///
/// One constructor rather than one per concern: each `#[ctor]` is a second
/// `unsafe` entry point running before `main`, and everything here is the same
/// job — putting the process into the state every test in this crate assumes.
///
/// The logger prints exactly what the builder below decides, as it always did —
/// that builder reads no environment, so its own filter is the default `Error`
/// — and keeps a copy of what a case asked to read back, see
/// [`capturing_logger`](capturing_logger).
#[ctor(unsafe)]
fn prepare_test_binary() {
  capturing_logger::install(pretty_env_logger::formatted_builder().build());
  disable_diagnostic_colours();
}

/// Renders every `StyleXError` without styling for the whole test binary.
///
/// `StyleXError`'s `Display` colours the `[StyleX]` prefix and the message, and
/// whether it does is decided at runtime by whether stderr is a terminal and by
/// `NO_COLOR` / `CLICOLOR_FORCE`. Any test that asserts on a rendered
/// diagnostic therefore passes with output piped to a file and fails in a
/// terminal — a real defect this suite has already had once.
///
/// Fixed once here rather than by stripping escape codes at each assertion: a
/// helper only protects the tests that remember to call it, and the next
/// diagnostic assertion written is the one that will not.
fn disable_diagnostic_colours() {
  stylex_macros::stylex_error::disable_colour_output();
}

pub(crate) mod capturing_logger;
pub(crate) mod support;
