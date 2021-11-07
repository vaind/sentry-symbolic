use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::num::NonZeroU64;
use std::ops::Bound;

use gimli::{
    constants, DebuggingInformationEntry, Dwarf, IncompleteLineProgram, LineProgramHeader, Unit,
};

use super::*;

type Result<T, E = gimli::Error> = std::result::Result<T, E>;

impl Converter {
    pub fn process_dwarf<R: gimli::Reader>(&mut self, dwarf: &Dwarf<R>) -> Result<()> {
        let mut reusable_cache = ReusableCaches::default();
        // Iterate over the compilation units.
        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = dwarf.unit(header)?;
            self.process_dwarf_cu(&mut reusable_cache, dwarf, &unit)?;
        }
        Ok(())
    }

    fn process_dwarf_cu<R: gimli::Reader>(
        &mut self,
        reusable_cache: &mut ReusableCaches,
        dwarf: &Dwarf<R>,
        unit: &Unit<R>,
    ) -> Result<()> {
        // Construct LineRow Sequences.
        let line_program = match unit.line_program.clone() {
            Some(lp) => lp,
            None => return Ok(()),
        };
        let mut cu_cache =
            PerCuCache::new(reusable_cache, dwarf, unit, line_program.header().clone());
        let sequences = parse_line_program(line_program)?;

        // TODO: figure out if we actually need to keep "sequences" separate?
        for seq in sequences {
            for row in seq.rows {
                // TODO: figure out what to do in this case? Why does it happen?
                if self.ranges.contains_key(&(row.address as u32)) {
                    // panic!("entry for line program row {:?} should not exist yet!", row);
                    continue;
                }
                let file_idx = cu_cache.file(self, row.file_index as u64)?;

                let source_location_idx = self.source_locations.len() as u32;
                self.source_locations.push(SourceLocation {
                    file_idx,
                    line: row.line,
                    function_idx: u32::MAX,
                    inlined_into_idx: None,
                });
                self.ranges.insert(row.address as u32, source_location_idx);
            }
        }

        // Iterate over the Debugging Information Entries (DIEs) in the unit.
        let mut depth = 0;
        let mut entries = unit.entries();
        while let Some((delta_depth, entry)) = entries.next_dfs()? {
            depth += delta_depth;

            let is_inlined_subroutine = match entry.tag() {
                constants::DW_TAG_subprogram => false,
                constants::DW_TAG_inlined_subroutine => true,
                _ => continue,
            };
            let caller_info = find_caller_info(entry)?;
            let caller_file = match caller_info.0 {
                Some(file_id) => cu_cache.file(self, file_id)? as u32,
                None => 0,
            };
            let caller_line = caller_info.1.unwrap_or(0) as u32;

            let mut ranges = dwarf.die_ranges(unit, entry)?;
            while let Some(range) = ranges.next()? {
                if is_inlined_subroutine {
                    // TODO: insert function info
                    let function_idx = u32::MAX;

                    for source_location_idx in sub_ranges(&mut self.ranges, &range) {
                        let caller_source_location =
                            &mut self.source_locations[*source_location_idx as usize];
                        let mut own_source_location = caller_source_location.clone();
                        own_source_location.function_idx = function_idx;

                        caller_source_location.file_idx = caller_file;
                        caller_source_location.line = caller_line;

                        own_source_location.inlined_into_idx = Some(*source_location_idx);

                        let own_source_location_idx = self.source_locations.len() as u32;
                        self.source_locations.push(own_source_location);

                        *source_location_idx = own_source_location_idx;
                    }
                } else {
                    // TODO: insert function info
                    let function_idx = u32::MAX;

                    for source_location_idx in sub_ranges(&mut self.ranges, &range) {
                        let source_location =
                            &mut self.source_locations[*source_location_idx as usize];
                        source_location.function_idx = function_idx;
                    }
                }
            }
        }

        Ok(())
    }
}

fn sub_ranges<'a>(
    ranges: &'a mut BTreeMap<u32, u32>,
    range: &gimli::Range,
) -> impl Iterator<Item = &'a mut u32> {
    let first_after = ranges.range(range.end as u32..).next();
    let upper_bound = if let Some((first_after_start, _)) = first_after {
        Bound::Excluded(*first_after_start)
    } else {
        Bound::Unbounded
    };
    let lower_bound = Bound::Included(range.begin as u32);
    ranges.range_mut((lower_bound, upper_bound)).map(|(_, v)| v)
}

#[derive(Debug, Default)]
struct ReusableCaches {
    file_mapping: HashMap<u32, u32>,
}

impl ReusableCaches {
    fn clear(&mut self) {
        self.file_mapping.clear();
    }
}

#[derive(Debug)]
struct PerCuCache<'dwarf, R: gimli::Reader> {
    dwarf: &'dwarf Dwarf<R>,
    unit: &'dwarf Unit<R>,
    header: LineProgramHeader<R>,
    reusable_cache: &'dwarf mut ReusableCaches,
}

impl<'dwarf, R: gimli::Reader> PerCuCache<'dwarf, R> {
    fn new(
        reusable_cache: &'dwarf mut ReusableCaches,
        dwarf: &'dwarf Dwarf<R>,
        unit: &'dwarf Unit<R>,
        header: LineProgramHeader<R>,
    ) -> Self {
        reusable_cache.clear();
        reusable_cache
            .file_mapping
            .reserve(header.file_names().len());
        Self {
            dwarf,
            unit,
            header,
            reusable_cache,
        }
    }

    fn file(&mut self, converter: &mut Converter, file_index: u64) -> Result<u32> {
        let entry = match self.reusable_cache.file_mapping.entry(file_index as u32) {
            Entry::Occupied(e) => return Ok(*e.get()),
            Entry::Vacant(e) => e,
        };
        let file = match self.header.file(file_index) {
            Some(file) => file,
            None => return Ok(u32::MAX),
        };

        let directory_idx = if let Some(dir) = file.directory(&self.header) {
            let directory = self
                .dwarf
                .attr_string(self.unit, dir)?
                .to_string_lossy()?
                .into_owned();
            Some(converter.strings.insert_full(directory).0 as u32)
        } else {
            None
        };

        let path_name = self
            .dwarf
            .attr_string(self.unit, file.path_name())?
            .to_string_lossy()?
            .into_owned();
        let path_name_idx = converter.strings.insert_full(path_name).0 as u32;

        let file_idx = converter
            .files
            .insert_full(File {
                directory_idx,
                path_name_idx,
            })
            .0 as u32;

        entry.insert(file_idx);

        Ok(file_idx)
    }
}

fn find_caller_info<R: gimli::Reader>(
    entry: &DebuggingInformationEntry<R>,
) -> Result<(Option<u64>, Option<u64>)> {
    let mut call_file = None;
    let mut call_line = None;
    let mut attrs = entry.attrs();
    while let Some(attr) = attrs.next()? {
        match attr.name() {
            constants::DW_AT_call_file => {
                call_file = attr.udata_value();
            }
            constants::DW_AT_call_line => {
                call_line = attr.udata_value();
            }
            _ => {}
        }
    }
    Ok((call_file, call_line))
}

fn find_matching_lines(
    sequences: &mut [LineSequence],
    range: gimli::Range,
) -> Option<&mut [LineProgramRow]> {
    // find the sequence matching the riven range
    let seq_idx = sequences
        .binary_search_by_key(&range.end, |seq| seq.end)
        .unwrap_or_else(|i| i);
    let seq = sequences
        .get_mut(seq_idx)
        .filter(|seq| seq.start <= range.begin)?;

    // inside the sequence, find the rows that are matching the range
    let from = match seq.rows.binary_search_by_key(&range.begin, |x| x.address) {
        Ok(idx) => idx,
        Err(0) => return None,
        Err(next_idx) => next_idx - 1,
    };

    let len = seq.rows[from..]
        .binary_search_by_key(&range.end, |x| x.address)
        .unwrap_or_else(|e| e);
    seq.rows.get_mut(from..from + len)
}

#[derive(Debug)]
pub struct LineSequence {
    start: u64,
    end: u64,
    rows: Vec<LineProgramRow>,
}

#[derive(Debug)]
pub struct LineProgramRow {
    address: u64,
    file_index: u32,
    line: u32,
}

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

#[cfg(test)]
mod tests {
    use std::fmt::Write;
    use std::path::Path;
    use std::{borrow, fs};

    use object::{Object, ObjectSection};

    use crate::converter::lookup::SourceLocationIter;

    use super::*;

    type Dwarf<'a> = gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>;
    type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    fn with_loaded_dwarf<T, F: FnOnce(&Dwarf) -> Result<T>>(path: &Path, f: F) -> Result<T> {
        let file = fs::File::open(&path).unwrap();
        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
        let object = object::File::parse(mmap.as_ref())?;

        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        // Load a section and return as `Cow<[u8]>`.
        let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
            match object.section_by_name(id.name()) {
                Some(ref section) => Ok(section
                    .uncompressed_data()
                    .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
                None => Ok(borrow::Cow::Borrowed(&[][..])),
            }
        };

        // Load all of the sections.
        let dwarf_cow = gimli::Dwarf::load(&load_section)?;

        // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
        let borrow_section: &dyn for<'a> Fn(
            &'a borrow::Cow<[u8]>,
        )
            -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
            &|section| gimli::EndianSlice::new(&*section, endian);

        // Create `EndianSlice`s for all of the sections.
        let dwarf = dwarf_cow.borrow(&borrow_section);

        f(&dwarf)
    }

    fn print_frames(frames: SourceLocationIter) -> String {
        let mut s = String::new();

        for source_location in frames {
            let name = String::new();
            let file = symbolic_common::join_path(
                source_location.directory().unwrap_or(""),
                source_location.path_name(),
            );
            let line = source_location.line();

            writeln!(s, "{}:{}: {}", file, line, name).unwrap();
        }
        s
    }

    #[test]
    fn work_on_dwarf() -> Result<()> {
        with_loaded_dwarf("tests/fixtures/inlined.debug".as_ref(), |dwarf| {
            let mut converter = Converter::new();
            converter.process_dwarf(dwarf)?;

            dbg!(&converter);

            println!("0x{:x}:", 0x10f0);
            println!("{}", print_frames(converter.lookup(0x10f0)));
            println!("0x{:x}:", 0x10f2);
            println!("{}", print_frames(converter.lookup(0x10f2)));
            println!("0x{:x}:", 0x10f8);
            println!("{}", print_frames(converter.lookup(0x10f8)));
            println!("0x{:x}:", 0x10f9);
            println!("{}", print_frames(converter.lookup(0x10f9)));
            println!("0x{:x}:", 0x10ff);
            println!("{}", print_frames(converter.lookup(0x10ff)));

            Ok(())
        })
    }
}
