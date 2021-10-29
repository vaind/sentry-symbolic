//! A simple program to gather some stats about DWARF info, to answer the
//! following questions:
//!
//! - What is the distribution / histogram / number of smallest
//!   address ranges / line-mappings (looking at the line programs)
//! - Get a histogram of the function ranges (how big are functions)
//! - Histogram of line programs per function ?
//! - Number of unique files/dirs and functions.
//!
//! Started out as a copy of:
//! - https://github.com/gimli-rs/gimli/blob/master/examples/simple.rs
//! - https://github.com/gimli-rs/gimli/blob/master/examples/simple_line.rs

use gimli::{constants, IncompleteLineProgram};
use object::{Object, ObjectSection};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::NonZeroU64;
use std::{borrow, env, fs, mem, path};

use crate::hist::Histogram;

const SHF_EXECINSTR: u64 = 0x4;

mod hist;

fn main() {
    for path in env::args().skip(1) {
        let file = fs::File::open(&path).unwrap();
        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
        let object = object::File::parse(&*mmap).unwrap();
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };
        dump_file(&object, endian).unwrap();
    }
}

fn dump_file(object: &object::File, endian: gimli::RunTimeEndian) -> Result<(), gimli::Error> {
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
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_cow.borrow(&borrow_section);

    let mut executable_range = 0;
    for s in object.sections() {
        if let object::SectionFlags::Elf { sh_flags } = s.flags() {
            if sh_flags & SHF_EXECINSTR == SHF_EXECINSTR {
                executable_range += s.size();
            }
        }
    }

    let mut covered_range = 0;
    let mut addr_ranges = Histogram::new();
    let mut lines = Histogram::new();
    let mut num_files = 0;

    // Iterate over the compilation units.
    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;

        // Construct LineRow Sequences.
        let sequences = unit.line_program.clone().and_then(parse_line_program);

        let mut files = HashSet::new();

        // Iterate over the Debugging Information Entries (DIEs) in the unit.
        // let mut depth = 0;
        // let mut entries = unit.entries();
        // while let Some((delta_depth, entry)) = entries.next_dfs()? {
        //     depth += delta_depth;

        //     match entry.tag() {
        //         constants::DW_TAG_subprogram => {}
        //         constants::DW_TAG_inlined_subroutine => {}
        //         _ => continue,
        //     }

        //     println!("{:indent$}{}", "", entry.tag(), indent = depth as usize);

        //     // // Iterate over the attributes in the DIE.
        //     // let mut attrs = entry.attrs();
        //     // while let Some(attr) = attrs.next()? {
        //     //     println!("   {}: {:?}", attr.name(), attr.value());
        //     // }

        //     let mut ranges = dwarf.die_ranges(&unit, entry)?;
        //     while let Some(range) = ranges.next()? {
        //         println!("{:indent$}{:?}", "", range, indent = depth as usize);
        //     }
        // }

        // Get the line program for the compilation unit.
        if let Some(program) = unit.line_program.clone() {
            // let comp_dir = if let Some(ref dir) = unit.comp_dir {
            //     path::PathBuf::from(dir.to_string_lossy().into_owned())
            // } else {
            //     path::PathBuf::new()
            // };

            // Iterate over the line program rows.
            let mut prev_row: Option<gimli::LineRow> = None;
            let mut rows = program.rows();
            while let Some((_header, row)) = rows.next_row()? {
                let addr = row.address();

                if let Some(prev_row) = prev_row {
                    if !row.end_sequence()
                        && (prev_row.file_index(), prev_row.line())
                            == (row.file_index(), row.line())
                    {
                        continue;
                    }
                    let range = addr - prev_row.address();
                    if range > 0 {
                        addr_ranges.record(range);
                        files.insert(row.file_index());
                        let line = match row.line() {
                            Some(line) => line.get(),
                            None => 0,
                        };
                        lines.record(line);

                        covered_range += range;
                    }
                }

                if row.end_sequence() {
                    prev_row = None;
                } else {
                    prev_row = Some(*row);

                    // Determine the path. Real applications should cache this for performance.
                    // let mut path = path::PathBuf::new();
                    // if let Some(file) = row.file(header) {
                    //     path = comp_dir.clone();
                    //     if let Some(dir) = file.directory(header) {
                    //         path.push(dwarf.attr_string(&unit, dir)?.to_string_lossy().as_ref());
                    //     }
                    //     path.push(
                    //         dwarf
                    //             .attr_string(&unit, file.path_name())?
                    //             .to_string_lossy()
                    //             .as_ref(),
                    //     );
                    // }

                    // Determine line/column. DWARF line/column is never 0, so we use that
                    // but other applications may want to display this differently.
                    // let line = match row.line() {
                    //     Some(line) => line.get(),
                    //     None => 0,
                    // };
                }
            }
        }
        num_files += files.len();
    }

    // println!("Histogram of address ranges:");
    // println!();

    let addr_stats = addr_ranges.stats();
    let line_stats = lines.stats();
    let coverage = (covered_range * 100) / executable_range;
    println!("Total executable bytes in sections: {}", executable_range);
    println!(
        "Total address range covered: {} (coverage: {}%)",
        covered_range, coverage
    );
    println!("Number of ranges: {}", addr_stats.total);
    println!("Median range: {}", addr_stats.median);
    println!("p90 range: {}", addr_stats.p90);
    println!("p99 range: {}", addr_stats.p99);
    println!("p999 range: {}", addr_stats.p999);
    println!();
    println!("Estimated number of files: {}", num_files);
    println!("Median #lines: {}", line_stats.median);
    println!("p90 #lines: {}", line_stats.p90);
    println!("p99 #lines: {}", line_stats.p99);
    println!("p999 #lines: {}", line_stats.p999);

    Ok(())
}

struct LineSequence {
    start: u64,
    end: u64,
    rows: Box<[LineRow]>,
}

struct LineRow {
    address: u64,
    file_index: u64,
    line: u32,
    column: u32,
}

// Adapted from: https://github.com/gimli-rs/addr2line/blob/ce1aa2c056c0f0164feafa1ef4d886e50a72b2d7/src/lib.rs#L563-L622
fn parse_line_program<R: gimli::Reader>(
    ilnp: IncompleteLineProgram<R>,
) -> Option<Vec<LineSequence>> {
    let mut sequences = Vec::new();
    let mut sequence_rows = Vec::<LineRow>::new();
    let mut rows = ilnp.clone().rows();
    while let Some((_, row)) = rows.next_row().ok()? {
        if row.end_sequence() {
            if let Some(start) = sequence_rows.first().map(|x| x.address) {
                let end = row.address();
                let mut rows = Vec::new();
                mem::swap(&mut rows, &mut sequence_rows);
                sequences.push(LineSequence {
                    start,
                    end,
                    rows: rows.into_boxed_slice(),
                });
            }
            continue;
        }

        let address = row.address();
        let file_index = row.file_index();
        let line = row.line().map(NonZeroU64::get).unwrap_or(0) as u32;
        let column = match row.column() {
            gimli::ColumnType::LeftEdge => 0,
            gimli::ColumnType::Column(x) => x.get() as u32,
        };

        if let Some(last_row) = sequence_rows.last_mut() {
            if last_row.address == address {
                last_row.file_index = file_index;
                last_row.line = line;
                last_row.column = column;
                continue;
            }
        }

        sequence_rows.push(LineRow {
            address,
            file_index,
            line,
            column,
        });
    }
    sequences.sort_by_key(|x| x.start);

    Some(sequences)
}
