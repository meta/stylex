#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod file_system;
pub mod package_json;
pub mod resolvers;
pub mod utils;

#[cfg(test)]
mod tests;
