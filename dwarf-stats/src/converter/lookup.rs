use std::ops::Bound;

use super::{Converter, SourceLocation};

impl Converter {
    pub fn lookup(&self, addr: u32) -> SourceLocationIter<'_> {
        let source_location_idx = self
            .ranges
            .range((Bound::Unbounded, Bound::Included(addr)))
            .next_back()
            .map(|(_, idx)| *idx);

        SourceLocationIter {
            symcache: self,
            source_location_idx,
        }
    }
}

pub struct SourceLocationIter<'symcache> {
    symcache: &'symcache Converter,
    source_location_idx: Option<u32>,
}

impl<'symcache> Iterator for SourceLocationIter<'symcache> {
    type Item = SourceLocationReference<'symcache>;

    fn next(&mut self) -> Option<Self::Item> {
        let source_location_idx = self.source_location_idx? as usize;
        let source_location = self
            .symcache
            .source_locations
            .get_index(source_location_idx)?;

        self.source_location_idx = source_location.inlined_into_idx;
        Some(SourceLocationReference {
            symcache: self.symcache,
            source_location,
        })
    }
}

pub struct SourceLocationReference<'symcache> {
    symcache: &'symcache Converter,
    source_location: &'symcache SourceLocation,
}

impl SourceLocationReference<'_> {
    pub fn line(&self) -> u32 {
        self.source_location.line
    }
    pub fn directory(&self) -> Option<&str> {
        let file = &self.symcache.files[self.source_location.file_idx as usize];

        file.directory_idx
            .map(|directory_idx| self.symcache.strings[directory_idx as usize].as_ref())
    }
    pub fn path_name(&self) -> &str {
        let file = &self.symcache.files[self.source_location.file_idx as usize];

        &self.symcache.strings[file.path_name_idx as usize]
    }

    pub fn function_name(&self) -> &str {
        if self.source_location.function_idx == u32::MAX {
            ""
        } else {
            let function = &self.symcache.functions[self.source_location.function_idx as usize];
            &self.symcache.strings[function.name_idx as usize]
        }
    }
}

impl<'symcache> std::fmt::Debug for SourceLocationReference<'symcache> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file = &self.symcache.files[self.source_location.file_idx as usize];
        let function = &self.symcache.functions[self.source_location.function_idx as usize];
        let line = self.source_location.line;

        f.debug_struct("SourceLocationReference")
            .field("function", function)
            .field("file", file)
            .field("line", &line)
            .finish()
    }
}
