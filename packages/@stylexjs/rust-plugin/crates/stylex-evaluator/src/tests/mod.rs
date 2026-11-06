use ctor::ctor;

pub(crate) mod capturing_logger;
mod check_declaration_test;
mod convertors_tests;
mod evaluate_result_test;
mod evaluate_tests;
mod growable_stack_test;
pub(crate) mod scaffolding;
mod stylex_first_that_works_test;

/// Installs the test logger with thread-local capture and the builder's default Error filter.
#[ctor(unsafe)]
fn init_logger() {
  capturing_logger::install(pretty_env_logger::formatted_builder().build());
}
