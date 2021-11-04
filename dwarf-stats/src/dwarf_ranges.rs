use std::mem;
use std::num::NonZeroU64;

use gimli::{constants, Dwarf, IncompleteLineProgram, Unit};

pub fn construct_ranges_for_cu<R: gimli::Reader>(
    dwarf: &Dwarf<R>,
    unit: &Unit<R>,
) -> Result<(), gimli::Error> {
    // Construct LineRow Sequences.
    let sequences = unit.line_program.clone().and_then(parse_line_program);
    let sequences = match sequences {
        Some(seq) => seq,
        None => return Ok(()),
    };

    // Iterate over the Debugging Information Entries (DIEs) in the unit.
    let mut depth = 0;
    let mut entries = unit.entries();
    while let Some((delta_depth, entry)) = entries.next_dfs()? {
        depth += delta_depth;

        match entry.tag() {
            constants::DW_TAG_subprogram => {}
            constants::DW_TAG_inlined_subroutine => {}
            _ => continue,
        }

        println!("{:indent$}{}", "", entry.tag(), indent = depth as usize);

        // // Iterate over the attributes in the DIE.
        // let mut attrs = entry.attrs();
        // while let Some(attr) = attrs.next()? {
        //     println!("   {}: {:?}", attr.name(), attr.value());
        // }

        let mut ranges = dwarf.die_ranges(unit, entry)?;
        while let Some(range) = ranges.next()? {
            if let Some(lines) = find_matching_lines(&sequences, range) {
                println!(
                    "{:indent$} DIE range: {:?}",
                    "",
                    range,
                    indent = depth as usize
                );
                println!(
                    "{:indent$} LineRows: {:?}",
                    "",
                    lines,
                    indent = depth as usize
                );
            }
        }
    }
    Ok(())
}

fn find_matching_lines(sequences: &[LineSequence], range: gimli::Range) -> Option<&[LineRow]> {
    // find the sequence matching the riven range
    let seq_idx = sequences
        .binary_search_by_key(&range.end, |seq| seq.end)
        .unwrap_or_else(|i| i);
    let seq = sequences
        .get(seq_idx)
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
    seq.rows.get(from..from + len)
}

#[derive(Debug)]
struct LineSequence {
    start: u64,
    end: u64,
    rows: Box<[LineRow]>,
}

#[derive(Debug)]
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
    let mut rows = ilnp.rows();
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
