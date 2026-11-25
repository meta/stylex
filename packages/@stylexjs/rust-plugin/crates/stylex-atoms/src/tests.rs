use ctor::ctor;

#[ctor(unsafe)]
fn init_logger() {
  pretty_env_logger::formatted_builder().try_init().ok();
}

#[path = "tests/transform_test.rs"]
mod transform_test;
