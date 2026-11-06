use ctor::ctor;

pub(crate) mod capturing_logger;
mod check_declaration_test;
mod convertors_tests;
mod evaluate_result_test;
mod evaluate_tests;
mod growable_stack_test;
pub(crate) mod scaffolding;
mod stylex_first_that_works_test;

/// The one logger this test binary has, installed before any case runs.
///
/// It prints exactly what the builder below decides, as it always did — that
/// builder reads no environment, so its own filter is the default `Error` — and
/// keeps a copy of what a case asked to read back, see
/// [`capturing_logger`](capturing_logger). One logger rather than two because
/// `log` admits one for the whole process.
#[ctor(unsafe)]
fn init_logger() {
  capturing_logger::install(pretty_env_logger::formatted_builder().build());
}
