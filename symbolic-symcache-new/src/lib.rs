//! The SymCache format.

#![warn(missing_docs)]

pub mod converter;
pub mod error;
pub mod format;

use std::convert::TryFrom;
use std::fmt;
use std::num::NonZeroU32;

use nonmax::NonMaxU32;

pub use converter::Converter;
pub use error::ErrorSink;
pub use format::Format;

// TODO: this is only used for comparisons/benchmarks, and should rather live inside a
// testing-focused utility.
#[allow(missing_docs)]
pub mod lookups;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RelativeAddress(pub NonMaxU32);

impl TryFrom<u64> for RelativeAddress {
    type Error = ();

    fn try_from(other: u64) -> Result<Self, Self::Error> {
        u32::try_from(other)
            .ok()
            .and_then(|x| NonMaxU32::try_from(x).ok())
            .map(RelativeAddress)
            .ok_or(())
    }
}

impl Into<u64> for RelativeAddress {
    fn into(self) -> u64 {
        u32::from(self.0) as u64
    }
}

impl fmt::Display for RelativeAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Index(pub NonMaxU32);

impl TryFrom<usize> for Index {
    type Error = ();

    fn try_from(other: usize) -> Result<Self, Self::Error> {
        u32::try_from(other)
            .ok()
            .and_then(|x| NonMaxU32::try_from(x).ok())
            .map(Index)
            .ok_or(())
    }
}

impl Into<usize> for Index {
    fn into(self) -> usize {
        u32::from(self.0) as usize
    }
}

impl TryFrom<u32> for Index {
    type Error = ();

    fn try_from(other: u32) -> Result<Self, Self::Error> {
        NonMaxU32::try_from(other).ok().map(Index).ok_or(())
    }
}

impl Into<u32> for Index {
    fn into(self) -> u32 {
        u32::from(self.0)
    }
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LineNumber(pub std::num::NonZeroU32);

impl TryFrom<u64> for LineNumber {
    type Error = ();

    fn try_from(other: u64) -> Result<Self, Self::Error> {
        u32::try_from(other)
            .ok()
            .and_then(|x| NonZeroU32::try_from(x).ok())
            .map(LineNumber)
            .ok_or(())
    }
}
impl TryFrom<u32> for LineNumber {
    type Error = ();

    fn try_from(other: u32) -> Result<Self, Self::Error> {
        NonZeroU32::try_from(other).ok().map(LineNumber).ok_or(())
    }
}

impl fmt::Display for LineNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
