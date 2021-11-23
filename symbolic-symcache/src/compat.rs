use thiserror::*;

use symbolic_common::{Arch, DebugId, Language, Name, NameMangling};

use crate::{new, old};

#[derive(Debug, Error)]
pub enum SymCacheError {
    #[error("{0}")]
    Old(old::SymCacheError),
    #[error("{0}")]
    New(new::Error),
}

impl From<old::SymCacheError> for SymCacheError {
    fn from(old: old::SymCacheError) -> Self {
        Self::Old(old)
    }
}

impl From<new::Error> for SymCacheError {
    fn from(new: new::Error) -> Self {
        Self::New(new)
    }
}

enum FunctionsInner<'data> {
    Old(old::Functions<'data>),
    New(new::FunctionIter<'data>),
}

pub struct Functions<'data>(FunctionsInner<'data>);

enum LookupInner<'data, 'cache> {
    Old(old::Lookup<'data, 'cache>),
    New(new::SourceLocationIter<'data>),
}

pub struct Lookup<'data, 'cache>(LookupInner<'data, 'cache>);

#[derive(Clone, Debug, Eq, PartialEq)]
enum LineInfoInner<'data> {
    Old(old::LineInfo<'data>),
    New(new::SourceLocation<'data>),
}

/// Information on a matched source line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineInfo<'data>(LineInfoInner<'data>);

impl<'data> LineInfo<'data> {
    /// Architecture of the image referenced by this line.
    pub fn arch(&self) -> Arch {
        match self.0 {
            LineInfoInner::Old(li) => li.arch(),
            LineInfoInner::New(sl) => sl.cache.header.arch,
        }
    }

    /// Debug identifier of the image referenced by this line.
    pub fn debug_id(&self) -> DebugId {
        match self.0 {
            LineInfoInner::Old(li) => li.debug_id(),
            LineInfoInner::New(sl) => sl.cache.header.debug_id,
        }
    }

    /// The instruction address where the enclosing function starts.
    pub fn function_address(&self) -> u64 {
        todo!()
    }

    /// The instruction address where the line starts.
    pub fn line_address(&self) -> u64 {
        todo!()
    }

    /// The actual instruction address.
    pub fn instruction_address(&self) -> u64 {
        match self.0 {
            LineInfoInner::Old(li) => li.instruction_address(),
            LineInfoInner::New(sl) => todo!(),
        }
    }

    /// The compilation directory of the function.
    pub fn compilation_dir(&self) -> &'data str {
        match self.0 {
            LineInfoInner::Old(li) => li.compilation_dir(),
            LineInfoInner::New(sl) => {
                let file = sl.file().unwrap();
                let comp_dir = file.and_then(|f| f.comp_dir().unwrap());
                comp_dir.unwrap_or_default()
            }
        }
    }

    /// The base dir of the current line.
    pub fn base_dir(&self) -> &'data str {
        match self.0 {
            LineInfoInner::Old(li) => li.base_dir(),
            LineInfoInner::New(sl) => {
                let file = sl.file().unwrap();
                let base_dir = file.and_then(|f| f.directory().unwrap());
                base_dir.unwrap_or_default()
            }
        }
    }

    /// The filename of the current line.
    pub fn filename(&self) -> &'data str {
        match self.0 {
            LineInfoInner::Old(li) => li.filename(),
            LineInfoInner::New(sl) => {
                let file = sl.file().unwrap();
                let name = file.and_then(|f| f.path_name().unwrap());
                name.unwrap_or_default()
            }
        }
    }

    /// The joined path and file name relative to the compilation directory.
    pub fn path(&self) -> String {
        let joined = symbolic_common::join_path(self.base_dir, self.filename);
        symbolic_common::clean_path(&joined).into_owned()
    }

    /// The fully joined absolute path including the compilation directory.
    pub fn abs_path(&self) -> String {
        let joined_path = symbolic_common::join_path(self.base_dir, self.filename);
        let joined = symbolic_common::join_path(self.comp_dir, &joined_path);
        symbolic_common::clean_path(&joined).into_owned()
    }

    /// The line number within the file.
    pub fn line(&self) -> u32 {
        match self.0 {
            LineInfoInner::Old(li) => li.line(),
            LineInfoInner::New(sl) => sl.line(),
        }
    }

    /// The source code language.
    pub fn language(&self) -> Language {
        match self.0 {
            LineInfoInner::Old(li) => li.line(),
            LineInfoInner::New(sl) => sl
                .function()
                .unwrap()
                .map(|f| f.language())
                .unwrap_or_default(),
        }
    }

    /// The string value of the symbol (mangled).
    pub fn symbol(&self) -> &'data str {
        match self.0 {
            LineInfoInner::Old(li) => li.symbol(),
            LineInfoInner::New(sl) => sl
                .function()
                .unwrap()
                .and_then(|f| f.name().unwrap())
                .unwrap_or("?"),
        }
    }

    /// The name of the function suitable for demangling.
    ///
    /// Use `symbolic::demangle` for demangling this symbol.
    pub fn function_name(&self) -> Name<'data> {
        Name::new(self.symbol(), NameMangling::Unknown, self.language())
    }
}

impl<'data, 'cache> Iterator for Lookup<'data, 'cache> {
    type Item = Result<LineInfo<'data>, SymCacheError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            LookupInner::Old(lookup) => {
                let line_info = lookup.next()?;
                let result = match line_info {
                    Ok(li) => Ok(LineInfo(LineInfoInner::Old(li))),
                    Err(e) => Err(SymCacheError::Old(e)),
                };
                Some(result)
            }
            LookupInner::New(source_location_iter) => {
                let source_location = source_location_iter.next()?;
                let result = match source_location {
                    Ok(sl) => Ok(LineInfo(LineInfoInner::New(sl))),
                    Err(e) => Err(SymCacheError::New(e)),
                };
                Some(result)
            }
        }
    }
}
