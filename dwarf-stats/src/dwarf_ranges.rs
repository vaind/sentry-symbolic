use std::mem;
use std::num::NonZeroU64;

use gimli::{constants, Dwarf, IncompleteLineProgram, Unit};

pub fn construct_ranges_for_cu<R: gimli::Reader>(
    dwarf: &Dwarf<R>,
    unit: &Unit<R>,
) -> Result<Vec<LineSequence>, gimli::Error> {
    // Construct LineRow Sequences.
    let sequences = unit.line_program.clone().and_then(parse_line_program);
    let mut sequences = match sequences {
        Some(seq) => seq,
        None => return Ok(vec![]),
    };

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

        // println!("{:indent$}{}", "", entry.tag(), indent = depth as usize);

        // Iterate over the attributes in the DIE.
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
            // println!("   {}: {:?}", attr.name(), attr.value());
        }

        let mut ranges = dwarf.die_ranges(unit, entry)?;
        while let Some(range) = ranges.next()? {
            if let Some(lines) = find_matching_lines(&mut sequences, range) {
                for line_record in lines {
                    if is_inlined_subroutine {
                        let caller_record = line_record.source_locations.last_mut().unwrap();
                        let mut own_record = caller_record.clone();
                        caller_record.file_index = call_file.unwrap() as u32;
                        caller_record.line = call_line.unwrap() as u32;

                        // TODO: write function name
                        line_record.source_locations.push(own_record);
                    } else {
                        let own_record = line_record.source_locations.last_mut().unwrap();
                        // TODO: write function name
                    }
                }
                // println!(
                //     "{:indent$} DIE range: {:?}",
                //     "",
                //     range,
                //     indent = depth as usize
                // );
                // println!(
                //     "{:indent$} LineRows: {:?}",
                //     "",
                //     lines,
                //     indent = depth as usize
                // );
            }
        }
    }
    Ok(sequences)
}

fn find_matching_lines(
    sequences: &mut [LineSequence],
    range: gimli::Range,
) -> Option<&mut [LineRow]> {
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
    pub rows: Vec<LineRow>,
}

#[derive(Debug)]
pub struct LineRow {
    address: u64,
    pub source_locations: Vec<SourceLocation>,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    fun: u32,
    file_index: u32,
    line: u32,
}

// Adapted from: https://github.com/gimli-rs/addr2line/blob/ce1aa2c056c0f0164feafa1ef4d886e50a72b2d7/src/lib.rs#L563-L622
fn parse_line_program<R: gimli::Reader>(
    ilnp: IncompleteLineProgram<R>,
) -> Option<Vec<LineSequence>> {
    let mut sequences = Vec::new();
    let mut sequence_rows = Vec::<LineRow>::new();
    let mut rows = ilnp.rows();
    while let Some((_, row)) = rows.next_row().ok()? {
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
        let column = match row.column() {
            gimli::ColumnType::LeftEdge => 0,
            gimli::ColumnType::Column(x) => x.get() as u32,
        };

        if let Some(last_row) = sequence_rows.last_mut() {
            if last_row.address == address {
                let source_loc = last_row.source_locations.last_mut().unwrap();
                source_loc.file_index = file_index;
                source_loc.line = line;
                continue;
            }
        }

        sequence_rows.push(LineRow {
            address,
            source_locations: vec![SourceLocation {
                fun: 0,
                file_index,
                line,
            }],
        });
    }
    sequences.sort_by_key(|x| x.start);

    Some(sequences)
}
