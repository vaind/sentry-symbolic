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

use std::{env, fs};

use rand::prelude::*;
use rand::rngs::SmallRng;

mod dwarf_converter;
mod dwarf_ranges;
mod hist;
mod lookups;
mod stats;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in env::args().skip(1) {
        let file = fs::File::open(&path).unwrap();
        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

        println!("Using `{}`", path);
        println!("============================================================");

        let object = object::File::parse(mmap.as_ref())?;
        let executable_range = lookups::get_executable_range(&object);

        // stats::dump_file(&object)?;

        // println!();

        // // create the two contexts
        // print!("Creating addr2line::Context ");
        // let start = std::time::Instant::now();
        // let ctx = lookups::create_addr2line(&mmap)?;
        // println!("{:?}", start.elapsed());

        // print!("Creating SymCache ");
        // let start = std::time::Instant::now();
        // let symcache_buf = lookups::create_symcache(&mmap)?;
        // let symcache = symbolic_symcache::SymCache::parse(&symcache_buf)?;
        // println!("{:?}", start.elapsed());

        // println!();

        // // run lookups
        // print!("Looking up in addr2line ");
        // let mut rng = SmallRng::seed_from_u64(0);
        // let start = std::time::Instant::now();
        // for _ in 0..1000 {
        //     let addr = rng.gen_range(executable_range.clone());
        //     lookups::lookup_addr2line(&ctx, addr)?;
        // }
        // println!("{:?} (1000x)", start.elapsed());

        // print!("Looking up in SymCache ");
        // let mut rng = SmallRng::seed_from_u64(0);
        // let start = std::time::Instant::now();
        // for _ in 0..1000 {
        //     let addr = rng.gen_range(executable_range.clone());
        //     lookups::lookup_symcache(&symcache, addr)?;
        // }
        // println!("{:?} (1000x)", start.elapsed());

        // // check correctness
        // println!();
        // let mut rng = rand::thread_rng();
        // for _ in 0..1_000 {
        //     let addr = rng.gen_range(executable_range.clone());
        //     let a = lookups::lookup_addr2line(&ctx, addr)?;
        //     let s = lookups::lookup_symcache(&symcache, addr)?;
        //     if a != s {
        //         println!("addr2line and symcache disagree for 0x{:x}", addr);
        //         println!("addr2line: {:#?}", a);
        //         println!("symcache: {:#?}", s);
        //     }
        // }
    }
    Ok(())
}
