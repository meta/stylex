//! The lookup structures the StyleX state manager composes.
//!
//! Both answer a position question -- "which recorded entry holds this?", "where
//! is this namespace key written?" -- with one hash probe instead of a scan of
//! the module. Neither holds the entries it points at, and neither decides what
//! a style means, so the answer a lookup gives is the answer the scan it
//! replaces gave.

pub mod candidate_index;
pub mod key_span_index;

#[cfg(test)]
mod tests;
