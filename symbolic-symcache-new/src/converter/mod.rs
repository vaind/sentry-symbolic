//! Defines the SymCache [`Converter`].

use indexmap::{set::IndexSet, IndexMap};
use std::collections::BTreeMap;

mod breakpad;
mod dwarf;
mod object;
mod serialize;

use symbolic_common::Language;

use crate::cache::raw;
pub use serialize::*;

/// The SymCache Converter.
///
/// This can convert data in various source formats to an intermediate representation, which can
/// then be serialized to disk via its [`Converter::serialize`] method.
#[derive(Debug, Default)]
pub struct Converter {
    /// The minimum addr for all the ranges in the debug file.
    /// This is used to ignore ranges that are below this threshold, as linkers leave range data
    /// intact, but rather set removed ranges to 0 (or below this threshold).
    /// Also, this is used as an offset for the saved ranges, to decrease the likelyhood they
    /// overflow `u32`.
    // TODO: figure out a better name. is this the *load bias*? where do we get this from?
    range_threshold: u64,

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

    /// Tries to convert the given `addr`, compressing it into 32-bits and applying the
    /// `range_threshold` (TODO: find better name for that), rejecting any addr that is below the
    /// threshold or exceeds 32-bits.
    fn offset_addr(&self, addr: u64) -> Option<u32> {
        use std::convert::TryFrom;
        addr.checked_sub(self.range_threshold)
            .and_then(|r| u32::try_from(r).ok())
    }

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
    fn insert_function(&mut self, name: &str, entry_pc: u32, lang: Language) -> u32 {
        let name_idx = self.insert_string(name);
        let lang = lang as u8;
        let (fun_idx, _) = self.functions.insert_full(raw::Function {
            name_idx,
            entry_pc,
            lang,
        });
        fun_idx as u32
    }
}
