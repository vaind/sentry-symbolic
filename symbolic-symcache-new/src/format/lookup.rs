use super::{raw, Error, Format, Result};

impl Format<'_> {
    /// Looks up an instruction address in the SymCache, yielding an iterator of [`SourceLocation`]s.
    ///
    /// This always returns an iterator, however that iterator might be empty in case no [`SourceLocation`]
    /// was found for the given `addr`.
    pub fn lookup(&self, addr: u64) -> SourceLocationIter<'_> {
        use std::convert::TryFrom;
        let addr = match addr
            .checked_sub(self.range_offset)
            .and_then(|r| u32::try_from(r).ok())
        {
            Some(addr) => addr,
            None => {
                return SourceLocationIter {
                    format: self,
                    source_location_idx: u32::MAX,
                }
            }
        };

        let source_location_start = (self.source_locations.len() - self.ranges.len()) as u32;
        let source_location_idx = match self.ranges.binary_search_by_key(&addr, |r| r.0) {
            Ok(idx) => source_location_start + idx as u32,
            Err(idx) if idx == 0 => u32::MAX,
            Err(idx) => source_location_start + idx as u32 - 1,
        };
        SourceLocationIter {
            format: self,
            source_location_idx,
        }
    }

    fn get_file(&self, file_idx: u32) -> Result<Option<File<'_>>> {
        if file_idx == u32::MAX {
            return Ok(None);
        }
        match self.files.get(file_idx as usize) {
            Some(file) => Ok(Some(File { format: self, file })),
            None => Err(Error::InvalidFileReference(file_idx)),
        }
    }
}

/// A source File included in the SymCache.
///
/// Source files can have up to three path prefixes/fragments.
/// They are in the order of `comp_dir`, `directory`, `path_name`.
/// If a later fragment is an absolute path, it overrides the previous fragment.
///
/// The [`File::full_path`] method yields the final concatenated and resolved path.
///
/// # Examples
///
/// Considering that a C project is being compiled inside the `/home/XXX/sentry-native/` directory,
/// - The `/home/XXX/sentry-native/src/sentry_core.c` may have the following fragments:
///   - comp_dir: /home/XXX/sentry-native/
///   - directory: -
///   - path_name: src/sentry_core.c
/// - The included file `/usr/include/pthread.h` may have the following fragments:
///   - comp_dir: /home/XXX/sentry-native/ <- The comp_dir is defined, but overrided by the dir below
///   - directory: /usr/include/
///   - path_name: pthread.h
#[derive(Debug)]
pub struct File<'data> {
    format: &'data Format<'data>,
    file: &'data raw::File,
}

impl<'data> File<'data> {
    /// Resolves the compilation directory of this source file.
    pub fn comp_dir(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.file.comp_dir_idx)
    }

    /// Resolves the parent directory of this source file.
    pub fn directory(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.file.directory_idx)
    }

    /// Resolves the final path name fragment of this source file.
    pub fn path_name(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.file.path_name_idx)
    }

    /// Resolves and concatenates the full path based on its individual fragments.
    pub fn full_path(&self) -> Result<Option<String>> {
        let comp_dir = self.comp_dir()?.unwrap_or_default();
        let directory = self.directory()?.unwrap_or_default();
        let path_name = self.path_name()?.unwrap_or_default();

        let prefix = symbolic_common::join_path(comp_dir, directory);
        let full_path = symbolic_common::join_path(&prefix, path_name);
        Ok(if full_path.is_empty() {
            None
        } else {
            Some(full_path)
        })
    }
}

/// A Function definition as included in the SymCache.
#[derive(Debug)]
pub struct Function<'data> {
    format: &'data Format<'data>,
    function: &'data raw::Function,
}

impl<'data> Function<'data> {
    /// The possibly mangled name/symbol of this function.
    pub fn name(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.function.name_idx)
    }
}

/// An Iterator that yields [`SourceLocation`]s, representing an inlining hierarchy.
#[derive(Debug)]
pub struct SourceLocationIter<'data> {
    format: &'data Format<'data>,
    source_location_idx: u32,
}

impl<'data> SourceLocationIter<'data> {
    /// Yields the next [`SourceLocation`] in the inlining hierarchy.
    // We return a `Result` here, so its not a *real* `Iterator`
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<SourceLocation<'data>>> {
        if self.source_location_idx == u32::MAX {
            return Ok(None);
        }
        match self
            .format
            .source_locations
            .get(self.source_location_idx as usize)
        {
            Some(source_location) => {
                self.source_location_idx = source_location.inlined_into_idx;
                Ok(Some(SourceLocation {
                    format: self.format,
                    source_location,
                }))
            }
            None => Err(Error::InvalidSourceLocationReference(
                self.source_location_idx,
            )),
        }
    }
}

/// A Source Location as included in the SymCache.
///
/// The source location represents a `(function, file, line, inlined_into)` tuple corresponding to
/// an instruction in the executable.
#[derive(Debug)]
pub struct SourceLocation<'data> {
    format: &'data Format<'data>,
    source_location: &'data raw::SourceLocation,
}

impl SourceLocation<'_> {
    /// The source line corresponding to the instruction.
    ///
    /// This might return `0` when no line information can be found.
    pub fn line(&self) -> u32 {
        self.source_location.line
    }

    /// The source file corresponding to the instruction.
    pub fn file(&self) -> Result<Option<File<'_>>> {
        self.format.get_file(self.source_location.file_idx)
    }

    /// The function corresponding to the instruction.
    pub fn function(&self) -> Result<Option<Function<'_>>> {
        let function_idx = self.source_location.function_idx;
        if function_idx == u32::MAX {
            return Ok(None);
        }
        match self.format.functions.get(function_idx as usize) {
            Some(function) => Ok(Some(Function {
                format: self.format,
                function,
            })),
            None => Err(Error::InvalidFunctionReference(function_idx)),
        }
    }

    // TODO: maybe forward some of the `File` and `Function` accessors, such as:
    // `function_name` or `full_path` for convenience.
}
