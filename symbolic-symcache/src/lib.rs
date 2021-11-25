//! Provides SymCache support.

#![warn(missing_docs)]

mod compat;
mod new;
mod old;
pub(crate) mod preamble;

// These are here for backwards compatibility:
pub use old::format;

pub use old::error::*;
pub use old::writer::*;

pub use compat::*;
