use indexmap::set::IndexSet;
use std::collections::BTreeMap;

mod dwarf;
mod error;
mod lookup;
mod serialize;

#[derive(Debug, Default)]
pub struct Converter {
    strings: IndexSet<String>,
    files: IndexSet<File>,
    functions: IndexSet<Function>,
    source_locations: IndexSet<SourceLocation>,
    // TODO: save "unfinished" source locations directly here, and concat them in the serializer
    ranges: BTreeMap<u32, u32>,
}

impl Converter {
    pub fn transform_strings<F: FnMut(String) -> String>(&mut self, _mapper: F) {
        // TODO: transform all the strings, for example to apply BCSymbolMaps.
    }
}

// TODO: maybe later, move all the casting to `u32` from the processor to the serializer

#[derive(Debug, PartialEq, Eq, Hash)]
struct File {
    directory_idx: Option<u32>,
    path_name_idx: u32,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Function {
    name_idx: u32,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SourceLocation {
    file_idx: u32,
    line: u32,
    function_idx: u32,
    inlined_into_idx: Option<u32>,
}

impl Converter {
    pub fn new() -> Self {
        Self::default()
    }
}
