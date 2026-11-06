pub(crate) mod diagnostics;
#[cfg(feature = "node")]
pub(crate) mod host;
#[cfg(feature = "node")]
pub(crate) mod js_host;
mod parser;
mod transform;

pub use parser::normalize_js;
#[cfg(feature = "node")]
pub(crate) use transform::{isolated, transform_with_host};
pub use transform::{
    process_metadata_to_css, transform_file, transform_source, transform_source_with_host,
    transform_source_without_fast_path,
};
