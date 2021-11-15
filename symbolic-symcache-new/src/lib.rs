//! The SymCache format.

#![warn(missing_docs)]

pub mod converter;
pub mod error;
pub mod format;

pub use converter::Converter;
pub use error::ErrorSink;
pub use format::Format;

// TODO: this is only used for comparisons/benchmarks, and should rather live inside a
// testing-focused utility.
#[allow(missing_docs)]
pub mod lookups;
