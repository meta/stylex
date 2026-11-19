use ctor::ctor;

mod candidate_index_test;
mod key_span_index_test;

#[ctor(unsafe)]
fn init_logger() {
  pretty_env_logger::formatted_builder().try_init().ok();
}
