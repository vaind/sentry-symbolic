//! Defines the SymCache [`Converter`].

use indexmap::{set::IndexSet, IndexMap};
use std::collections::BTreeMap;

mod breakpad;
mod dwarf;
mod serialize;

use symbolic_common::Language;

use crate::format::raw;
pub use serialize::*;

/// The SymCache Converter.
///
/// This can convert data in various source formats to an intermediate representation, which can
/// then be serialized to disk via its [`Converter::serialize`] method.
#[derive(Debug, Default)]
pub struct Converter {
    /// The concatenation of all strings that have been added to this `Converter`.
    string_bytes: Vec<u8>,
    /// A map from [`String`]s that have been added to this `Converter` to [`StringRef`]s, i.e.,
    /// indices into the `string_bytes` vector.
    strings: IndexMap<String, raw::String>,
    /// The set of all [`raw::File`]s that have been added to this `Converter`.
    files: IndexSet<raw::File>,
    /// The set of all [`raw::Function`]s that have been added to this `Converter`.
    functions: IndexSet<raw::Function>,
    /// The set of all [`raw::SourceLocation`]s that have been added to this `Converter` and that
    /// aren't directly associated with a code range.
    source_locations: IndexSet<raw::SourceLocation>,
    /// A map from code ranges to the [`raw::SourceLocation`]s they correspond to.
    ///
    /// Only the starting address of a range is saved, the end address is given implicitly
    /// by the start address of the next range.
    ranges: BTreeMap<u32, raw::SourceLocation>,
}

impl Converter {
    /// Creates a new Converter.
    pub fn new() -> Self {
        Self::default()
    }
    //     pub fn transform_strings<F: FnMut(String) -> String>(&mut self, _mapper: F) {
    //         // TODO: transform all the strings, for example to apply BCSymbolMaps.
    //     }

    /// Insert a string into this converter.
    ///
    /// If the string was already present, it is not added again. The returned `u32`
    /// is the string's index in insertion order.
    fn insert_string(&mut self, s: &str) -> u32 {
        if let Some(existing_idx) = self.strings.get_index_of(s) {
            return existing_idx as u32;
        }
        let string_offset = self.string_bytes.len() as u32;
        let string_len = s.len() as u32;
        self.string_bytes.extend(s.bytes());
        let (string_idx, _) = self.strings.insert_full(
            s.to_owned(),
            raw::String {
                string_offset,
                string_len,
            },
        );
        string_idx as u32
    }

    /// Insert a [`raw::SourceLocation`] into this converter.
    ///
    /// If the `SourceLocation` was already present, it is not added again. The returned `u32`
    /// is the `SourceLocation`'s index in insertion order.
    fn insert_source_location(&mut self, source_location: raw::SourceLocation) -> u32 {
        self.source_locations.insert_full(source_location).0 as u32
    }

    /// Insert a file into this converter.
    ///
    /// If the file was already present, it is not added again. The returned `u32`
    /// is the file's index in insertion order.
    fn insert_file(
        &mut self,
        path_name: &str,
        directory: Option<&str>,
        comp_dir: Option<&str>,
    ) -> u32 {
        let path_name_idx = self.insert_string(path_name);
        let directory_idx = directory.map_or(u32::MAX, |d| self.insert_string(d));
        let comp_dir_idx = comp_dir.map_or(u32::MAX, |cd| self.insert_string(cd));

        let (file_idx, _) = self.files.insert_full(raw::File {
            path_name_idx,
            directory_idx,
            comp_dir_idx,
        });

        file_idx as u32
    }

    /// Insert a function into this converter.
    ///
    /// If the function was already present, it is not added again. The returned `u32`
    /// is the function's index in insertion order.
    fn insert_function(&mut self, name: &str, entry_addr: u32, lang: Language) -> u32 {
        let name_idx = self.insert_string(name);
        let entry_addr = entry_addr;
        let lang = lang as u8;
        let (fun_idx, _) = self.functions.insert_full(raw::Function {
            name_idx,
            entry_addr,
            lang,
        });
        fun_idx as u32
    }
}
