use std::fs;

use symbolic_symcache_new::{format, lookups};

#[test]
fn test_simple() {
    let file = fs::File::open("tests/fixtures/simple.debug").unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

    let object = object::File::parse(mmap.as_ref()).unwrap();
    let executable_range = lookups::get_executable_range(&object);

    let addr2line_ctx = lookups::create_addr2line(&mmap).unwrap();
    let symcache_buf = lookups::create_new_symcache(&mmap).unwrap();
    let symcache = format::Format::parse(&symcache_buf).unwrap();

    for addr in executable_range {
        let addr2line_result = lookups::lookup_addr2line(&addr2line_ctx, addr).unwrap();
        let symcache_result = lookups::lookup_new_symcache(&symcache, addr).unwrap();

        assert_eq!(symcache_result, addr2line_result, "addr: {}", addr);
    }
}

#[test]
fn test_inlined() {
    let file = fs::File::open("tests/fixtures/inlined.debug").unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

    let object = object::File::parse(mmap.as_ref()).unwrap();
    let executable_range = lookups::get_executable_range(&object);

    let addr2line_ctx = lookups::create_addr2line(&mmap).unwrap();
    let symcache_buf = lookups::create_new_symcache(&mmap).unwrap();
    let symcache = format::Format::parse(&symcache_buf).unwrap();

    for addr in executable_range {
        let addr2line_result = lookups::lookup_addr2line(&addr2line_ctx, addr).unwrap();
        let symcache_result = lookups::lookup_new_symcache(&symcache, addr).unwrap();

        assert_eq!(symcache_result, addr2line_result, "addr: {}", addr);
    }
}
#[test]
fn test_two_inlined() {
    let file = fs::File::open("tests/fixtures/two_inlined.debug").unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

    let object = object::File::parse(mmap.as_ref()).unwrap();
    let executable_range = lookups::get_executable_range(&object);

    let addr2line_ctx = lookups::create_addr2line(&mmap).unwrap();
    let symcache_buf = lookups::create_new_symcache(&mmap).unwrap();
    let symcache = format::Format::parse(&symcache_buf).unwrap();

    for addr in executable_range {
        let addr2line_result = lookups::lookup_addr2line(&addr2line_ctx, addr).unwrap();
        let symcache_result = lookups::lookup_new_symcache(&symcache, addr).unwrap();

        assert_eq!(symcache_result, addr2line_result, "addr: {}", addr);
    }
}
