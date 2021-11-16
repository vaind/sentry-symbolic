use std::fs;

use symbolic_symcache_new::lookups::{
    create_addr2line, create_new_symcache_dwarf, get_executable_range, lookup_addr2line,
    lookup_new_symcache, resolve_lookup, ResolvedFrame,
};
use symbolic_symcache_new::*;
use symbolic_testutils::fixture;

#[test]
fn works_on_simple() {
    let buf = fs::read(fixture("inlining/simple.debug")).unwrap();
    let symcache_buf = create_new_symcache_dwarf(&buf).unwrap();
    let symcache = Format::parse(&symcache_buf).unwrap();

    assert_eq!(&resolve_lookup(&symcache, 0x10ef), &[]);

    assert_eq!(
        &resolve_lookup(&symcache, 0x10f2),
        &[ResolvedFrame {
            function: "simple_fn".into(),
            file: "/root-comp-dir/simple.rs".into(),
            line: 5
        }]
    );

    // TODO: assert that we can resolve non-DWARF symbols
}

#[test]
fn works_on_inlined() {
    let buf = fs::read(fixture("inlining/inlined.debug")).unwrap();
    let symcache_buf = create_new_symcache_dwarf(&buf).unwrap();
    let symcache = Format::parse(&symcache_buf).unwrap();

    assert_eq!(
        &resolve_lookup(&symcache, 0x10f2),
        &[
            ResolvedFrame {
                function: "_ZN7inlined10inlined_fn17haa7a5b60e644bff9E".into(),
                file: "/root-comp-dir/inlined.rs".into(),
                line: 10
            },
            ResolvedFrame {
                function: "caller_fn".into(),
                file: "/root-comp-dir/inlined.rs".into(),
                line: 3
            }
        ]    );

    // TODO: assert that we can resolve non-DWARF symbols
}

//#[test]
//fn simple_all() {
//    let file = fs::File::open(fixture("inlining/simple.debug")).unwrap();
//    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
//    let object = object::File::parse(&*mmap).unwrap();
//    let executable_range = get_executable_range(&object);
//
//    let addr2line_ctx = create_addr2line(&mmap).unwrap();
//    let symcache_buf = create_new_symcache_dwarf(&mmap).unwrap();
//    let symcache = format::Format::parse(&symcache_buf).unwrap();
//
//    for addr in executable_range {
//        let addr2line_result = lookup_addr2line(&addr2line_ctx, addr).unwrap();
//        let symcache_result = lookup_new_symcache(&symcache, addr).unwrap();
//
//        assert_eq!(symcache_result, addr2line_result, "addr: {}", addr);
//    }
//}
//
//#[test]
//fn inlined_all() {
//    let file = fs::File::open(fixture("inlining/inlined.debug")).unwrap();
//    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
//    let object = object::File::parse(&*mmap).unwrap();
//    let executable_range = get_executable_range(&object);
//
//    let addr2line_ctx = create_addr2line(&mmap).unwrap();
//    let symcache_buf = create_new_symcache_dwarf(&mmap).unwrap();
//    let symcache = format::Format::parse(&symcache_buf).unwrap();
//
//    for addr in executable_range {
//        let addr2line_result = lookup_addr2line(&addr2line_ctx, addr).unwrap();
//        let symcache_result = lookup_new_symcache(&symcache, addr).unwrap();
//
//        assert_eq!(symcache_result, addr2line_result, "addr: {}", addr);
//    }
//}
//#[test]
//fn two_inlined_all() {
//    let file = fs::File::open(fixture("inlining/two_inlined.debug")).unwrap();
//    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
//    let object = object::File::parse(&*mmap).unwrap();
//    let executable_range = get_executable_range(&object);
//
//    let addr2line_ctx = create_addr2line(&mmap).unwrap();
//    let symcache_buf = create_new_symcache_dwarf(&mmap).unwrap();
//    let symcache = format::Format::parse(&symcache_buf).unwrap();
//
//    for addr in executable_range {
//        let addr2line_result = lookup_addr2line(&addr2line_ctx, addr).unwrap();
//        let symcache_result = lookup_new_symcache(&symcache, addr).unwrap();
//
//        assert_eq!(symcache_result, addr2line_result, "addr: {}", addr);
//    }
//}
