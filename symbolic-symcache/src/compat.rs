use symbolic_common::{Arch, AsSelf, DebugId};

use crate::{new, old, SymCacheError};

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
    pub fn functions(&self) -> Functions<'data> {
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

// impl From<new::Error> for old::SymCacheError {
//     fn from(new_error: new::Error) -> Self {
//         let kind = match new_error {
//             new::Error::BufferNotAligned => todo!(),
//             new::Error::HeaderTooSmall => old::SymCacheErrorKind::BadFileHeader,
//             new::Error::WrongEndianness => todo!(),
//             new::Error::WrongFormat => old::SymCacheErrorKind::BadFileMagic,
//             new::Error::WrongVersion => old::SymCacheErrorKind::UnsupportedVersion,
//             new::Error::BadFormatLength => todo!(),
//         };
//     }
// }

#[derive(Clone, Debug)]
enum FunctionsInner<'data> {
    Old(old::Functions<'data>),
    New(new::FunctionIter<'data>),
}

/// An iterator over all functions in a `SymCache`.
#[derive(Clone, Debug)]
pub struct Functions<'data>(FunctionsInner<'data>);

impl<'data> Iterator for Functions<'data> {
    type Item = Result<Function<'data>, SymCacheError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            LookupInner::Old(lookup) => lookup.next(),
            LookupInner::New { iter, lookup_addr } => {}
        }
    }
}

#[derive(Clone, Debug)]
enum FunctionInner<'data> {
    Old(old::Function<'data>),
    New,
}

#[derive(Clone, Debug)]
pub struct Function<'data>(FunctionInner<'data>);

impl<'data> Function<'data> {
    /// The ID of the function.
    pub fn id(&self) -> usize {
        self.index as usize
    }

    /// The ID of the parent function, if this function was inlined.
    pub fn parent_id(&self) -> Option<usize> {
        self.record.parent(self.id())
    }

    /// The address where the function starts.
    pub fn address(&self) -> u64 {
        self.record.addr_start()
    }

    /// The raw name of the function.
    pub fn symbol(&self) -> &'data str {
        read_symbol(self.data, self.symbols, self.record.symbol_id())
            .unwrap_or(None)
            .unwrap_or("?")
    }

    /// The language of the function.
    pub fn language(&self) -> Language {
        Language::from_u32(self.record.lang.into())
    }

    /// The name of the function suitable for demangling.
    ///
    /// Use `symbolic::demangle` for demangling this symbol.
    pub fn name(&self) -> Name<'_> {
        Name::new(self.symbol(), NameMangling::Unknown, self.language())
    }

    /// The compilation dir of the function.
    pub fn compilation_dir(&self) -> &str {
        self.record.comp_dir.read_str(self.data).unwrap_or("")
    }

    /// An iterator over all lines in the function.
    pub fn lines(&self) -> Lines<'data> {}
}

#[derive(Clone, Debug)]
enum LookupInner<'data, 'cache> {
    Old(old::Lookup<'data, 'cache>),
    New {
        iter: new::SourceLocationIter<'data, 'cache>,
        lookup_addr: u64,
    },
}

/// An iterator over line matches for an address lookup.
#[derive(Clone, Debug)]
pub struct Lookup<'data, 'cache>(LookupInner<'data, 'cache>);

impl<'data, 'cache> Lookup<'data, 'cache> {
    /// Collects all line matches into a collection.
    pub fn collect<B>(self) -> Result<B, SymCacheError>
    where
        B: std::iter::FromIterator<old::LineInfo<'data>>,
    {
        Iterator::collect(self)
    }
}

impl<'data, 'cache> Iterator for Lookup<'data, 'cache> {
    type Item = Result<old::LineInfo<'data>, old::SymCacheError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            LookupInner::Old(lookup) => lookup.next(),
            LookupInner::New { iter, lookup_addr } => {
                let sl = iter.next()?;

                Some(Ok(old::LineInfo {
                    arch: sl.cache.arch(),
                    debug_id: sl.cache.debug_id(),
                    sym_addr: sl
                        .function()
                        .map(|f| f.entry_pc() as u64)
                        .unwrap_or(u64::MAX),
                    line_addr: *lookup_addr,
                    instr_addr: *lookup_addr,
                    line: sl.line(),
                    lang: sl.function().map(|f| f.language()).unwrap_or_default(),
                    symbol: sl.function().and_then(|f| f.name()),
                    filename: sl.file().map(|f| f.path_name()).unwrap_or_default(),
                    base_dir: sl.file().and_then(|f| f.directory()).unwrap_or_default(),
                    comp_dir: sl.file().and_then(|f| f.comp_dir()).unwrap_or_default(),
                }))
            }
        }
    }
}
