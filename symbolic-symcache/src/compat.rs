pub use crate::old::SymCacheError;
use crate::{new, old};

impl From<new::Error> for old::SymCacheError {
    fn from(new_error: new::Error) -> Self {
        let kind = match new_error {
            new::Error::BufferNotAligned => todo!(),
            new::Error::HeaderTooSmall => old::SymCacheErrorKind::BadFileHeader,
            new::Error::WrongEndianness => todo!(),
            new::Error::WrongFormat => old::SymCacheErrorKind::BadFileMagic,
            new::Error::WrongVersion => old::SymCacheErrorKind::UnsupportedVersion,
            new::Error::BadFormatLength => todo!(),
        }
    }
}

pub(crate) enum FunctionsInner<'data, 'cache> {
    Old(old::Functions<'data>),
    New(new::FunctionIter<'data, 'cache>),
}

pub struct Functions<'data, 'cache>(pub(crate) FunctionsInner<'data, 'cache>);

pub(crate) enum LookupInner<'data, 'cache> {
    Old(old::Lookup<'data, 'cache>),
    New {
        iter: new::SourceLocationIter<'data, 'cache>,
        lookup_addr: u64,
    },
}

pub struct Lookup<'data, 'cache>(pub(crate) LookupInner<'data, 'cache>);

impl<'data: 'cache, 'cache> Iterator for Lookup<'data, 'cache> {
    type Item = Result<old::LineInfo<'data>, old::SymCacheError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            LookupInner::Old(lookup) => lookup.next(),
            LookupInner::New {
                ref iter,
                lookup_addr,
            } => {
                let sl = iter.next()?;
                Some(Ok(old::LineInfo {
                    arch: sl.arch(),
                    debug_id: sl.debug_id(),
                    sym_addr: sl
                        .function()
                        .map(|f| f.entry_pc() as u64)
                        .unwrap_or(u64::MAX),
                    line_addr: lookup_addr,
                    instr_addr: lookup_addr,
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
