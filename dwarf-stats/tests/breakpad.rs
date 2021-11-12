use std::path::Path;
use std::string::String;
use std::{borrow, fs};

use symbolic_debuginfo::breakpad::BreakpadObject;

// TODO: do this properly
mod dwarf;
use dwarf::resolve_lookup;

use dwarf_stats::*;

#[test]
fn test_macos() {
    let file = fs::File::open("../symbolic-testutils/fixtures/macos/crash.sym").unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
    let breakpad = BreakpadObject::parse(&mmap).unwrap();

    let mut converter = Converter::default();
    converter.process_breakpad(&breakpad, |_| {});
    let mut symcache_buf = Vec::new();
    converter.serialize(&mut symcache_buf, |_| {}).unwrap();
    let symcache = Format::parse(&symcache_buf).unwrap();

    let lookup_result = resolve_lookup(&symcache, 0x1a2a);
    println!("{:?}", lookup_result);
}
