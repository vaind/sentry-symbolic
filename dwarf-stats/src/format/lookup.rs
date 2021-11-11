use super::{raw, Error, Format, Result};

impl Format<'_> {
    pub fn lookup(&self, addr: u64) -> SourceLocationIter<'_> {
        let source_location_start = (self.source_locations.len() - self.ranges.len()) as u32;
        let source_location_idx = match self.ranges.binary_search_by_key(&(addr as u32), |r| r.0) {
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

#[derive(Debug)]
pub struct File<'data> {
    format: &'data Format<'data>,
    file: &'data raw::File,
}

impl<'data> File<'data> {
    pub fn comp_dir(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.file.comp_dir_idx)
    }
    pub fn directory(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.file.directory_idx)
    }
    pub fn path_name(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.file.path_name_idx)
    }

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

#[derive(Debug)]
pub struct Function<'data> {
    format: &'data Format<'data>,
    function: &'data raw::Function,
}

impl<'data> Function<'data> {
    pub fn name(&self) -> Result<Option<&'data str>> {
        self.format.get_string(self.function.name_idx)
    }
}

#[derive(Debug)]
pub struct SourceLocationIter<'data> {
    format: &'data Format<'data>,
    source_location_idx: u32,
}

impl<'data> SourceLocationIter<'data> {
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

#[derive(Debug)]
pub struct SourceLocation<'data> {
    format: &'data Format<'data>,
    source_location: &'data raw::SourceLocation,
}

impl SourceLocation<'_> {
    pub fn line(&self) -> u32 {
        self.source_location.line
    }

    pub fn file(&self) -> Result<Option<File<'_>>> {
        self.format.get_file(self.source_location.file_idx)
    }

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
}
