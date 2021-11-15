use std::fs;

use symbolic_debuginfo::breakpad::BreakpadObject;

// TODO: do this properly
mod dwarf;
use dwarf::{resolve_lookup, ResolvedFrame};

use symbolic_symcache_new::*;

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
    assert_eq!(
        lookup_result[0],
        ResolvedFrame {
            function: "google_breakpad::MinidumpFileWriter::Copy(unsigned int, void const*, long)".to_string(),
            file: "/Users/travis/build/getsentry/breakpad-tools/macos/../deps/breakpad/src/client/minidump_file_writer.cc".to_string(),
            line: 312
        }
    );
}
