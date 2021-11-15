use std::fs;

use symbolic_symcache_new::lookups::{create_new_symcache_breakpad, resolve_lookup, ResolvedFrame};
use symbolic_symcache_new::*;
use symbolic_testutils::fixture;

#[test]
fn test_macos() {
    let file = fs::read(fixture("macos/crash.sym")).unwrap();
    let symcache_buf = create_new_symcache_breakpad(&file).unwrap();
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

#[test]
fn test_windows() {
    let file = fs::read(fixture("windows/crash.sym")).unwrap();
    let symcache_buf = create_new_symcache_breakpad(&file).unwrap();
    let symcache = Format::parse(&symcache_buf).unwrap();

    let lookup_result = resolve_lookup(&symcache, 0x2112);
    assert_eq!(
        lookup_result[0],
        ResolvedFrame {
            function: "google_breakpad::ExceptionHandler::WriteMinidumpWithException(unsigned long,_EXCEPTION_POINTERS *,MDRawAssertionInfo *)".to_string(),
            file: "c:\\projects\\breakpad-tools\\deps\\breakpad\\src\\client\\windows\\handler\\exception_handler.cc".to_string(),
            line: 846
        }
    );
}
