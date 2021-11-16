use std::collections::hash_map::Entry;
use std::collections::{btree_map, BTreeMap, HashMap};
use std::mem;
use std::num::NonZeroU64;
use std::ops::Bound;

use gimli::{
    constants, AttributeValue, DebuggingInformationEntry, Dwarf, IncompleteLineProgram,
    LineProgramHeader, ReaderOffset, Unit, UnitOffset,
};

use super::*;
use crate::format::raw;
use crate::ErrorSink;

type Result<T, E = gimli::Error> = std::result::Result<T, E>;

impl Converter {
    fn offset_addr_range(&self, range: gimli::Range) -> Option<Range<u32>> {
        let start = self.offset_addr(range.begin)?;
        let end = self.offset_addr(range.end)?;
        Some(start..end)
    }

    /// Processes the given [`Dwarf`] file.
    ///
    /// This feeds any errors that were raised during processing into the given [`ErrorSink`].
    /// Currently, errors are being captured at the granularity of a DWARF compilation unit, but
    /// more fine grained errors may be raised in the future.
    pub fn process_dwarf<R: gimli::Reader, E: ErrorSink<gimli::Error>>(
        &mut self,
        dwarf: &Dwarf<R>,
        mut error_sink: E,
    ) {
        let error_sink = &mut error_sink;
        let mut reusable_cache = ReusableCaches::default();
        // Iterate over the compilation units.
        let mut iter = dwarf.units();
        while let Some(header) = iter.next().unwrap_or_else(|err| {
            error_sink.raise_error(err);
            None
        }) {
            let unit = match dwarf.unit(header) {
                Ok(unit) => unit,
                Err(err) => {
                    error_sink.raise_error(err);
                    continue;
                }
            };
            if let Err(err) = self.process_dwarf_cu(&mut reusable_cache, dwarf, &unit, error_sink) {
                error_sink.raise_error(err);
            }
        }
    }

    /// Process a single DWARF compilation unit.
    fn process_dwarf_cu<R: gimli::Reader, E: ErrorSink<gimli::Error>>(
        &mut self,
        reusable_cache: &mut ReusableCaches,
        dwarf: &Dwarf<R>,
        unit: &Unit<R>,
        error_sink: &mut E,
    ) -> Result<()> {
        let lang = find_cu_lang(&unit)
            .unwrap_or_else(|e| {
                error_sink.raise_error(e);
                Some(0)
            })
            .unwrap_or_default() as u8;
        // Construct LineRow Sequences.
        let line_program = match unit.line_program.clone() {
            Some(lp) => lp,
            None => return Ok(()),
        };
        let mut cu_cache = PerCuCache::new(
            reusable_cache,
            self,
            dwarf,
            unit,
            lang,
            line_program.header().clone(),
        )?;
        let sequences = parse_line_program(line_program)?;

        // TODO: figure out if we actually need to keep "sequences" separate?
        let mut line_program_ranges = BTreeMap::new();
        for seq in sequences {
            for row in seq.rows {
                let file_idx = cu_cache.insert_file(self, row.file_index as u64)?;

                line_program_ranges.insert(
                    self.offset_addr(row.address).unwrap_or(u32::MAX),
                    raw::SourceLocation {
                        file_idx,
                        line: row.line,
                        function_idx: u32::MAX,
                        inlined_into_idx: u32::MAX,
                    },
                );
            }
        }

        // Iterate over the Debugging Information Entries (DIEs) in the unit.
        let mut _depth = 0;
        let mut entries = unit.entries();
        while let Some((delta_depth, entry)) = entries.next_dfs()? {
            _depth += delta_depth;

            let is_inlined_subroutine = match entry.tag() {
                constants::DW_TAG_subprogram => false,
                constants::DW_TAG_inlined_subroutine => true,
                _ => continue,
            };
            let mut ranges = dwarf.die_ranges(unit, entry)?;
            while let Some(range) = ranges.next()? {
                let range = match self.offset_addr_range(range) {
                    Some(range) => range,
                    None => continue,
                };

                if is_inlined_subroutine {
                    let (caller_file, caller_line, function_idx) = match find_caller_info(entry)? {
                        Some(CallerInfo {
                            call_file,
                            call_line,
                            abstract_origin,
                        }) => {
                            let caller_file = cu_cache.insert_file(self, call_file)?;
                            let caller_idx = cu_cache.insert_function(self, abstract_origin)?;
                            (caller_file, call_line, caller_idx)
                        }
                        None => (u32::MAX, 0, u32::MAX),
                    };
                    for callee_source_location in sub_ranges(&mut line_program_ranges, range) {
                        let mut caller_source_location = callee_source_location.clone();
                        caller_source_location.file_idx = caller_file;
                        caller_source_location.line = caller_line;
                        callee_source_location.function_idx = function_idx;

                        callee_source_location.inlined_into_idx =
                            self.insert_source_location(caller_source_location);
                    }
                } else {
                    let function_idx = cu_cache.insert_function(self, entry.offset())?;
                    let entry_pc = self.functions[function_idx as usize].entry_pc;

                    // insert a dummy source location in case this function's start is not covered by the line program
                    if !line_program_ranges.contains_key(&entry_pc) {
                        line_program_ranges.insert(
                            entry_pc,
                            raw::SourceLocation {
                                function_idx,
                                line: 0,
                                file_idx: u32::MAX,
                                inlined_into_idx: u32::MAX,
                            },
                        );
                    }
                    for source_location in sub_ranges(&mut line_program_ranges, range) {
                        source_location.function_idx = function_idx;
                    }
                }
            }
        }

        for (addr, source_location) in line_program_ranges {
            match self.ranges.entry(addr) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(source_location);
                }
                btree_map::Entry::Occupied(_entry) => {
                    // TODO: figure out what to do in this case? Why does it happen?
                    // panic!(
                    //     "entry for addr 0x{:x} should not exist yet! {:?} =? {:?}",
                    //     addr,
                    //     entry.get(),
                    //     source_location_idx,
                    // );
                }
            }
        }

        Ok(())
    }
}

/// Returns an iterator of [`SourceLocation`]s that match the given [`gimli::Range`].
fn sub_ranges<'a>(
    ranges: &'a mut BTreeMap<u32, raw::SourceLocation>,
    range: std::ops::Range<u32>,
) -> impl Iterator<Item = &'a mut raw::SourceLocation> {
    let first_after = ranges.range(range.end..).next();
    let upper_bound = if let Some((first_after_start, _)) = first_after {
        Bound::Excluded(*first_after_start)
    } else {
        Bound::Unbounded
    };
    let lower_bound = Bound::Included(range.start);
    ranges.range_mut((lower_bound, upper_bound)).map(|(_, v)| v)
}

/// A collection of caches that are being re-used across compilation units.
///
/// Only the *allocation* is being reused, not the actual data. The data is cleared on each CU.
#[derive(Debug, Default)]
struct ReusableCaches {
    file_mapping: HashMap<u64, u32>,
    function_mapping: HashMap<u64, u32>,
}

impl ReusableCaches {
    fn clear(&mut self) {
        self.file_mapping.clear();
        self.function_mapping.clear();
    }
}

/// This is a per-compilation unit Cache which caches file and function conversions.
#[derive(Debug)]
struct PerCuCache<'dwarf, R: gimli::Reader> {
    dwarf: &'dwarf Dwarf<R>,
    unit: &'dwarf Unit<R>,
    lang: u8,
    header: LineProgramHeader<R>,
    reusable_cache: &'dwarf mut ReusableCaches,
    comp_dir_idx: u32,
}

impl<'dwarf, R> PerCuCache<'dwarf, R>
where
    R: gimli::Reader,
    R::Offset: gimli::ReaderOffset,
{
    fn new(
        reusable_cache: &'dwarf mut ReusableCaches,
        converter: &mut Converter,
        dwarf: &'dwarf Dwarf<R>,
        unit: &'dwarf Unit<R>,
        lang: u8,
        header: LineProgramHeader<R>,
    ) -> Result<Self> {
        reusable_cache.clear();
        reusable_cache
            .file_mapping
            .reserve(header.file_names().len());

        let comp_dir_idx = if let Some(ref comp_dir) = unit.comp_dir {
            converter.insert_string(&comp_dir.to_string()?)
        } else {
            u32::MAX
        };

        Ok(Self {
            dwarf,
            unit,
            lang,
            header,
            reusable_cache,
            comp_dir_idx,
        })
    }

    /// Insert a string identified by the [`AttributeValue`] into the global string table.
    ///
    /// Returns the index of the string in the global string table.
    // TODO: use this function somehow without messing up the borrow checker
    #[allow(dead_code)]
    fn insert_string(&self, converter: &mut Converter, attr: AttributeValue<R>) -> Result<u32> {
        let attr = self.dwarf.attr_string(self.unit, attr)?;
        let s = attr.to_string()?;
        Ok(converter.insert_string(&s))
    }

    /// Inserts a function identified by the [`UnitOffset`] into the global function table.
    ///
    /// Returns the index of the function in the global function table.
    fn insert_function(
        &mut self,
        converter: &mut Converter,
        die_offset: UnitOffset<R::Offset>,
    ) -> Result<u32> {
        let entry = match self
            .reusable_cache
            .function_mapping
            .entry(die_offset.0.into_u64())
        {
            Entry::Occupied(e) => return Ok(*e.get()),
            Entry::Vacant(e) => e,
        };
        let die = self.unit.entry(die_offset)?;
        let FunctionInfo { name, entry_pc } = find_function_info(&die)?;
        let function_name_idx = match name {
            Some(name) => {
                let attr = self.dwarf.attr_string(self.unit, name)?;
                converter.insert_string(&attr.to_string()?)
            }
            None => u32::MAX,
        };

        let entry_pc = entry_pc
            .and_then(|epc| converter.offset_addr(epc))
            .unwrap_or(u32::MAX);

        let function_idx = converter
            .functions
            .insert_full(raw::Function {
                name_idx: function_name_idx,
                entry_pc,
                lang: self.lang,
            })
            .0 as u32;

        entry.insert(function_idx);

        Ok(function_idx)
    }

    /// Insert a file identified by the per-compilation unit file index into the global file table.
    ///
    /// Returns the index of the file in the global file table.
    fn insert_file(&mut self, converter: &mut Converter, file_index: u64) -> Result<u32> {
        let entry = match self.reusable_cache.file_mapping.entry(file_index) {
            Entry::Occupied(e) => return Ok(*e.get()),
            Entry::Vacant(e) => e,
        };
        let file = match self.header.file(file_index) {
            Some(file) => file,
            None => return Ok(u32::MAX),
        };

        // The `directory_index == 0` is defined to be the `comp_dir`.
        let directory_idx = if file.directory_index() != 0 {
            if let Some(dir) = file.directory(&self.header) {
                let directory = self.dwarf.attr_string(self.unit, dir)?;
                let idx = converter.insert_string(&directory.to_string()?);
                idx
            } else {
                u32::MAX
            }
        } else {
            u32::MAX
        };

        let path_name = self.dwarf.attr_string(self.unit, file.path_name())?;
        let path_name_idx = converter.insert_string(&path_name.to_string()?);

        let file_idx = converter
            .files
            .insert_full(raw::File {
                comp_dir_idx: self.comp_dir_idx,
                directory_idx,
                path_name_idx,
            })
            .0 as u32;

        entry.insert(file_idx);

        Ok(file_idx)
    }
}

/// Returns the caller information of [`constants::DW_TAG_inlined_subroutine`] DIE entry.
///
/// The caller information includes the [`constants::DW_AT_call_file`], [`constants::DW_AT_call_line`],
/// and the function metadata of the inlined function.
#[derive(Debug)]
struct CallerInfo<R: gimli::Reader> {
    call_file: u64,
    call_line: u32,
    abstract_origin: UnitOffset<R::Offset>,
}

fn find_caller_info<R: gimli::Reader>(
    entry: &DebuggingInformationEntry<R>,
) -> Result<Option<CallerInfo<R>>> {
    let mut call_file = None;
    let mut call_line = None;
    let mut abstract_origin = None;
    let mut attrs = entry.attrs();
    while let Some(attr) = attrs.next()? {
        match attr.name() {
            constants::DW_AT_call_file => {
                if let gimli::AttributeValue::FileIndex(fi) = attr.value() {
                    call_file = Some(fi);
                }
            }
            constants::DW_AT_call_line => {
                call_line = attr.udata_value().map(|val| val as u32);
            }
            constants::DW_AT_abstract_origin => {
                if let gimli::AttributeValue::UnitRef(ur) = attr.value() {
                    abstract_origin = Some(ur);
                }
            }
            _ => {}
        }
    }
    Ok(match (call_file, call_line, abstract_origin) {
        (Some(call_file), Some(call_line), Some(abstract_origin)) => Some(CallerInfo {
            call_file,
            call_line,
            abstract_origin,
        }),
        _ => None,
    })
}

#[derive(Debug)]
struct FunctionInfo<R: gimli::Reader> {
    name: Option<AttributeValue<R>>,
    entry_pc: Option<u64>,
}

fn find_function_info<R: gimli::Reader>(
    entry: &DebuggingInformationEntry<R>,
) -> Result<FunctionInfo<R>> {
    let mut name = None;
    let mut linkage_name = None;
    let mut entry_pc = None;
    let mut attrs = entry.attrs();
    while let Some(attr) = attrs.next()? {
        match attr.name() {
            constants::DW_AT_name => {
                name = Some(attr.value());
            }
            constants::DW_AT_linkage_name => {
                linkage_name = Some(attr.value());
            }
            constants::DW_AT_entry_pc => {
                entry_pc = Some(attr.udata_value().unwrap());
            }
            _ => {}
        }
    }
    Ok(FunctionInfo {
        name: linkage_name.or(name),
        entry_pc,
    })
}

fn find_cu_lang<R: gimli::Reader>(unit: &Unit<R>) -> Result<Option<u16>> {
    let mut lang = None;
    let mut cursor = unit.header.entries(&unit.abbreviations);
    cursor.next_dfs()?;
    let root = cursor.current().ok_or(gimli::Error::MissingUnitDie)?;
    let mut attrs = root.attrs();

    while let Some(attr) = attrs.next()? {
        match attr.name() {
            constants::DW_AT_language => {
                lang = Some(attr.u16_value().unwrap());
            }
            _ => {}
        }
    }
    Ok(lang)
}

/// A sequence of contiguous [`LineProgramRow`]s spanning the address ranges `start` to `end`.
#[derive(Debug)]
pub struct LineSequence {
    start: u64,
    end: u64,
    rows: Vec<LineProgramRow>,
}

/// Represents a row in the DWARF line program.
///
/// A row is essentially a mapping from `address` to `file_index` and `line`.
/// The `line` can be `0` under some circumstances.
#[derive(Debug)]
pub struct LineProgramRow {
    address: u64,
    file_index: u32,
    line: u32,
}

/// Completely resolve the given [`IncompleteLineProgram`] into a list of [`LineSequence`]s.
// Adapted from: https://github.com/gimli-rs/addr2line/blob/ce1aa2c056c0f0164feafa1ef4d886e50a72b2d7/src/lib.rs#L563-L622
fn parse_line_program<R: gimli::Reader>(
    ilnp: IncompleteLineProgram<R>,
) -> Result<Vec<LineSequence>> {
    let mut sequences = Vec::new();
    let mut sequence_rows = Vec::<LineProgramRow>::new();
    let mut rows = ilnp.rows();
    while let Some((_, row)) = rows.next_row()? {
        if row.end_sequence() {
            if let Some(start) = sequence_rows.first().map(|x| x.address) {
                let end = row.address();
                // ignore 0-ranges
                if start == 0 {
                    sequence_rows.clear();
                    continue;
                }
                let mut rows = Vec::new();
                mem::swap(&mut rows, &mut sequence_rows);
                sequences.push(LineSequence { start, end, rows });
            }
            continue;
        }

        let address = row.address();
        let file_index = row.file_index() as u32;
        let line = row.line().map(NonZeroU64::get).unwrap_or(0) as u32;

        if let Some(last_row) = sequence_rows.last_mut() {
            if last_row.address == address {
                last_row.file_index = file_index;
                last_row.line = line;
                continue;
            }
            if last_row.file_index == file_index && last_row.line == line {
                continue;
            }
        }

        sequence_rows.push(LineProgramRow {
            address,
            file_index,
            line,
        });
    }
    sequences.sort_by_key(|x| x.start);

    Ok(sequences)
}
