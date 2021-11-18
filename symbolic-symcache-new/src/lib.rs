//! The SymCache format.

#![warn(missing_docs)]

pub mod cache;
pub mod converter;
pub mod error;

pub use cache::SymCache;
pub use converter::Converter;
pub use error::ErrorSink;

// TODO: this is only used for comparisons/benchmarks, and should rather live inside a
// testing-focused utility.
#[allow(missing_docs)]
pub mod lookups;
