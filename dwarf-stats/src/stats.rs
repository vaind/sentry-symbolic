use std::collections::HashSet;
use std::{borrow, path};

use object::{Object, ObjectSection};

use crate::hist::Histogram;

const SHF_EXECINSTR: u64 = 0x4;

pub fn dump_file(object: &object::File) -> Result<(), gimli::Error> {
    let mut executable_range = 0;
    for s in object.sections() {
        if let object::SectionFlags::Elf { sh_flags } = s.flags() {
            if sh_flags & SHF_EXECINSTR == SHF_EXECINSTR {
                executable_range += s.size();
            }
        }
    }

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
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_cow.borrow(&borrow_section);

    let mut covered_range = 0;
    let mut addr_ranges = Histogram::new();
    let mut lines = Histogram::new();
    let mut file_paths = HashSet::new();

    // Iterate over the compilation units.
    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;

        //dwarf_ranges::construct_ranges_for_cu(&dwarf, &unit)?;

        // Get the line program for the compilation unit.
        if let Some(program) = unit.line_program.clone() {
            let mut seen_files = HashSet::new();

            let comp_dir = if let Some(ref dir) = unit.comp_dir {
                path::PathBuf::from(dir.to_string_lossy().into_owned())
            } else {
                path::PathBuf::new()
            };

            // Iterate over the line program rows.
            let mut prev_row: Option<gimli::LineRow> = None;
            let mut rows = program.rows();
            while let Some((header, row)) = rows.next_row()? {
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
                        if seen_files.insert(row.file_index()) {
                            // Determine the path.
                            if let Some(file) = row.file(header) {
                                let mut path = comp_dir.clone();
                                if let Some(dir) = file.directory(header) {
                                    path.push(
                                        dwarf.attr_string(&unit, dir)?.to_string_lossy().as_ref(),
                                    );
                                }
                                path.push(
                                    dwarf
                                        .attr_string(&unit, file.path_name())?
                                        .to_string_lossy()
                                        .as_ref(),
                                );
                                file_paths.insert(path);
                            }
                        }
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

                    // Determine line/column. DWARF line/column is never 0, so we use that
                    // but other applications may want to display this differently.
                    // let line = match row.line() {
                    //     Some(line) => line.get(),
                    //     None => 0,
                    // };
                }
            }
        }
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
    println!("Number of files: {}", file_paths.len());
    println!("Median #lines: {}", line_stats.median);
    println!("p90 #lines: {}", line_stats.p90);
    println!("p99 #lines: {}", line_stats.p99);
    println!("p999 #lines: {}", line_stats.p999);

    Ok(())
}
