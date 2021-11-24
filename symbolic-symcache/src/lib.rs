//! Provides symcache support.

#![warn(missing_docs)]

mod compat;
mod new;
mod old;

use compat::{FunctionsInner, LookupInner};
use symbolic_common::{Arch, AsSelf, DebugId};

pub use crate::compat::{Functions, Lookup, SymCacheError};

/// The magic file preamble as individual bytes.
const SYMCACHE_MAGIC_BYTES: [u8; 4] = *b"SYMC";

/// The magic file preamble to identify SymCache files.
///
/// Serialized as ASCII "SYMC" on little-endian (x64) systems.
pub const SYMCACHE_MAGIC: u32 = u32::from_be_bytes(SYMCACHE_MAGIC_BYTES);
/// The byte-flipped magic, which indicates an endianness mismatch.
pub const SYMCACHE_MAGIC_FLIPPED: u32 = SYMCACHE_MAGIC.swap_bytes();

/// The start of a SymCache file.
#[repr(C, packed)]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Preamble {
    /// Magic bytes, see `SYMCACHE_MAGIC`.
    pub magic: u32,
    /// Version of the SymCache file format.
    pub version: u32,
}

impl Preamble {
    fn parse(data: &[u8]) -> Result<Self, SymCacheError> {}
}
/// A platform independent symbolication cache.
///
/// Use [`SymCacheWriter`](super::writer::SymCacheWriter) writer to create SymCaches,
/// including the conversion from object files.
#[derive(Debug)]
pub enum SymCache<'data> {
    Old(old::SymCache<'data>),
    New(new::SymCache<'data>),
}

impl<'data> SymCache<'data> {
    /// Parses a SymCache from a binary buffer.
    pub fn parse(data: &'data [u8]) -> Result<Self, SymCacheError> {
        new::SymCache::parse(data)
            .map(Self::New)
            .or_else(|_| old::SymCache::parse(data).map(Self::Old))
    }

    /// The version of the SymCache file format.
    pub fn version(&self) -> u32 {
        match self {
            Self::New(symc) => symc.version(),
            Self::Old(symc) => symc.version(),
        }
    }
    /// Returns whether this cache is up-to-date.
    pub fn is_latest(&self) -> bool {
        self.version() == new::raw::SYMCACHE_VERSION
    }

    /// The architecture of the symbol file.
    pub fn arch(&self) -> Arch {
        match self {
            Self::New(symc) => symc.arch(),
            Self::Old(symc) => symc.arch(),
        }
    }

    /// The debug identifier of the cache file.
    pub fn debug_id(&self) -> DebugId {
        match self {
            Self::New(symc) => symc.debug_id(),
            Self::Old(symc) => symc.debug_id(),
        }
    }

    /// Returns true if line information is included.
    pub fn has_line_info(&self) -> bool {
        match self {
            Self::New(symc) => symc.has_line_info(),
            Self::Old(symc) => symc.has_line_info(),
        }
    }

    /// Returns true if file information is included.
    pub fn has_file_info(&self) -> bool {
        match self {
            Self::New(symc) => symc.has_file_info(),
            Self::Old(symc) => symc.has_file_info(),
        }
    }

    /// Returns an iterator over all functions.
    pub fn functions(&self) -> Functions<'data, '_> {
        match self {
            Self::New(symc) => Functions(FunctionsInner::New(symc.functions())),
            Self::Old(symc) => Functions(FunctionsInner::Old(symc.functions())),
        }
    }

    /// Given an address this looks up the symbol at that point.
    ///
    /// Because of inline information this returns a vector of zero or
    /// more symbols.  If nothing is found then the return value will be
    /// an empty vector.
    pub fn lookup(&self, addr: u64) -> Result<Lookup<'data, '_>, SymCacheError> {
        match self {
            Self::New(symc) => Ok(Lookup(LookupInner::New {
                iter: symc.lookup(addr),
                lookup_addr: addr,
            })),
            Self::Old(symc) => {
                let lookup = symc.lookup(addr)?;
                Ok(Lookup(LookupInner::Old(lookup)))
            }
        }
    }
}

impl<'slf, 'd: 'slf> AsSelf<'slf> for SymCache<'d> {
    type Ref = SymCache<'slf>;

    fn as_self(&'slf self) -> &Self::Ref {
        self
    }
}
