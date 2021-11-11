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

mod converter;
pub mod format;
pub mod lookups;
