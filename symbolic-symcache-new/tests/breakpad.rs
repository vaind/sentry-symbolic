use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs;

use symbolic_debuginfo::breakpad::BreakpadObject;
use symbolic_testutils::fixture;

use symbolic_symcache_new::lookups::{create_new_symcache_breakpad, resolve_lookup, ResolvedFrame};
use symbolic_symcache_new::*;

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
            line: LineNumber::try_from(312_u32).ok(),
        }
    );
}

#[test]
fn test_macos_all() {
    let file = fs::read(fixture("macos/crash.sym")).unwrap();
    let breakpad = BreakpadObject::parse(&file).unwrap();

    let mut converter = Converter::default();
    converter.process_breakpad(&breakpad, |_| {});
    let mut symcache_buf = Vec::new();
    converter.serialize(&mut symcache_buf, |_| {}).unwrap();
    let symcache = Format::parse(&symcache_buf).unwrap();

    let files: BTreeMap<_, _> = breakpad
        .file_records()
        .map(|fr| {
            let fr = fr.unwrap();
            (fr.id, fr.name)
        })
        .collect();

    for func in breakpad.func_records() {
        let func = func.unwrap();
        println!("{}", func.name);

        for line_rec in func.lines() {
            let line_rec = line_rec.unwrap();

            for addr in line_rec.range() {
                let lookup_result = resolve_lookup(&symcache, addr);
                assert_eq!(lookup_result.len(), 1);
                let ResolvedFrame {
                    function,
                    file,
                    line,
                } = &lookup_result[0];
                assert_eq!(function, func.name);
                assert_eq!(file, files[&line_rec.file_id]);
                assert_eq!(*line, LineNumber::try_from(line_rec.line as u32).ok());
            }
        }
    }
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
            line: LineNumber::try_from(846_u32).ok()
        }
    );
}
