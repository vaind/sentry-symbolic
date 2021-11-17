//! Defines the SymCache [`Converter`].

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::num::NonZeroU32;

use indexmap::{set::IndexSet, IndexMap};

mod breakpad;
mod dwarf;
mod serialize;

use symbolic_common::Language;

use crate::format::raw;
use crate::{Index, LineNumber, RelativeAddress};
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
    ranges: BTreeMap<RelativeAddress, raw::SourceLocation>,
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
    fn offset_addr(&self, addr: u64) -> Option<RelativeAddress> {
        addr.checked_sub(self.range_threshold)
            .and_then(|a| RelativeAddress::try_from(a).ok())
    }

    /// Insert a string into this converter.
    ///
    /// If the string was already present, it is not added again. The returned `u32`
    /// is the string's index in insertion order.
    fn insert_string(&mut self, s: &str) -> Option<Index> {
        if let Some(existing_idx) = self.strings.get_index_of(s) {
            return Some(Index::try_from(existing_idx).unwrap());
        }

        if self.strings.len() >= u32::MAX as usize {
            return None;
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
        Some(Index::try_from(string_idx).unwrap())
    }

    /// Insert a [`raw::SourceLocation`] into this converter.
    ///
    /// If the `SourceLocation` was already present, it is not added again. The returned `u32`
    /// is the `SourceLocation`'s index in insertion order.
    fn insert_source_location(&mut self, source_location: raw::SourceLocation) -> Option<Index> {
        if let Some(existing_idx) = self.strings.get_index_of(&source_location) {
            return Some(Index::try_from(existing_idx).unwrap());
        }

        if self.strings.len() >= u32::MAX as usize {
            return None;
        }

        Some(Index::try_from(self.source_locations.insert_full(source_location).0).unwrap())
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
    ) -> Option<Index> {
        let path_name_idx = self.insert_string(path_name)?;
        let directory_idx = if let Some(directory) = directory {
            match self.insert_string(directory) {
                Some(idx) => Some(idx),
                None => {
                    return None;
                }
            }
        } else {
            None
        };
        let comp_dir_idx = if let Some(comp_dir) = comp_dir {
            match self.insert_string(comp_dir) {
                Some(idx) => Some(idx),
                None => {
                    return None;
                }
            }
        } else {
            None
        };

        let file = raw::File {
            path_name_idx: Some(path_name_idx),
            directory_idx,
            comp_dir_idx,
        };

        if let Some(existing_idx) = self.files.get_index_of(&file) {
            return Some(Index::try_from(existing_idx).unwrap());
        }

        if self.strings.len() >= u32::MAX as usize {
            return None;
        }

        Some(Index::try_from(self.files.insert_full(file).0).unwrap())
    }

    /// Insert a function into this converter.
    ///
    /// If the function was already present, it is not added again. The returned `u32`
    /// is the function's index in insertion order.
    fn insert_function(
        &mut self,
        name: &str,
        entry_pc: Option<RelativeAddress>,
        lang: Language,
    ) -> Option<Index> {
        let name_idx = self.insert_string(name)?;
        let lang = lang as u8;

        let function = raw::Function {
            name_idx,
            entry_pc,
            lang,
        };

        if let Some(existing_idx) = self.functions.get_index_of(&function) {
            return Some(Index::try_from(existing_idx).unwrap());
        }

        if self.functions.len() >= u32::MAX as usize {
            return None;
        }

        Some(Index::try_from(self.functions.insert_full(function).0).unwrap())
    }
}
