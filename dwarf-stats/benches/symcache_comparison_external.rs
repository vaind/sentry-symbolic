use std::fs;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::prelude::*;
use rand::rngs::SmallRng;

use dwarf_stats::{format, lookups};

const DEBUG_PATH: &'static str = "testcases/libc6.debug";

pub fn creation(c: &mut Criterion) {
    let file = fs::File::open(&DEBUG_PATH).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

    // create the two contexts
    let mut group = c.benchmark_group("Cache creation");
    group.bench_with_input(
        BenchmarkId::new("addr2line", DEBUG_PATH),
        &mmap,
        |b, mmap| b.iter(|| lookups::create_addr2line(mmap).unwrap()),
    );
    group.bench_with_input(
        BenchmarkId::new("old symcache", DEBUG_PATH),
        &mmap,
        |b, mmap| {
            b.iter(|| {
                let symcache_buf = lookups::create_symcache(mmap).unwrap();
                symbolic_symcache::SymCache::parse(&symcache_buf).unwrap();
            })
        },
    );
    group.bench_with_input(
        BenchmarkId::new("new symcache", DEBUG_PATH),
        &mmap,
        |b, mmap| {
            b.iter(|| {
                let symcache_buf = lookups::create_new_symcache(mmap).unwrap();
                format::Format::parse(&symcache_buf).unwrap();
            })
        },
    );
    //let mut rng = SmallRng::seed_from_u64(0);
    //let start = std::time::Instant::now();
    //for _ in 0..1000 {
    //    let addr = rng.gen_range(executable_range.clone());
    //    lookups::lookup_addr2line(&ctx, addr)?;
    //}
    //println!("{:?} (1000x)", start.elapsed());

    //print!("Looking up in SymCache ");
    //let mut rng = SmallRng::seed_from_u64(0);
    //let start = std::time::Instant::now();
    //for _ in 0..1000 {
    //    let addr = rng.gen_range(executable_range.clone());
    //    lookups::lookup_symcache(&symcache, addr)?;
    //}
    //println!("{:?} (1000x)", start.elapsed());

    //print!("Looking up in new SymCache ");
    //let mut rng = SmallRng::seed_from_u64(0);
    //let start = std::time::Instant::now();
    //for _ in 0..1000 {
    //    let addr = rng.gen_range(executable_range.clone());
    //    lookups::lookup_new_symcache(&symcache2, addr)?;
    //}
    //println!("{:?} (1000x)", start.elapsed());

    //// check correctness
    //let mut rng = rand::thread_rng();
    //// when testing with `tests/fixtures/inlined.debug:
    //// for addr in 0x10ef..0x10fa {
    //for _ in 0..10 {
    //    let addr = rng.gen_range(executable_range.clone());
    //    let a = lookups::lookup_addr2line(&ctx, addr)?;
    //    let s = lookups::lookup_symcache(&symcache, addr)?;
    //    let n = lookups::lookup_new_symcache(&symcache2, addr)?;
    //    if a != s || a != n {
    //        println!();
    //        println!("disagreement for 0x{:x}", addr);
    //        println!("addr2line: {:#?}", a);
    //        println!("symcache: {:#?}", s);
    //        println!("new symcache: {:#?}", n);
    //    }
    //}
    //}
    group.finish();
}

pub fn lookup(c: &mut Criterion) {
    let file = fs::File::open(&DEBUG_PATH).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

    let object = object::File::parse(mmap.as_ref()).unwrap();
    let executable_range = lookups::get_executable_range(&object);

    let addr2line_ctx = lookups::create_addr2line(&mmap).unwrap();
    let symcache_buf_old = lookups::create_symcache(&mmap).unwrap();
    let symcache_old = symbolic_symcache::SymCache::parse(&symcache_buf_old).unwrap();
    let symcache_buf_new = lookups::create_new_symcache(&mmap).unwrap();
    let symcache_new = format::Format::parse(&symcache_buf_new).unwrap();

    let mut group = c.benchmark_group("Address lookup");

    let mut rng = SmallRng::seed_from_u64(0);
    group.bench_with_input(
        BenchmarkId::new("addr2line", DEBUG_PATH),
        &addr2line_ctx,
        |b, ctx| {
            b.iter(|| {
                let addr = rng.gen_range(executable_range.clone());
                lookups::lookup_addr2line(&ctx, addr).unwrap();
            })
        },
    );

    let mut rng = SmallRng::seed_from_u64(0);
    group.bench_with_input(
        BenchmarkId::new("old symcache", DEBUG_PATH),
        &symcache_old,
        |b, symcache| {
            b.iter(|| {
                let addr = rng.gen_range(executable_range.clone());
                lookups::lookup_symcache(&symcache, addr).unwrap();
            })
        },
    );

    let mut rng = SmallRng::seed_from_u64(0);
    group.bench_with_input(
        BenchmarkId::new("new symcache", DEBUG_PATH),
        &symcache_new,
        |b, symcache| {
            b.iter(|| {
                let addr = rng.gen_range(executable_range.clone());
                lookups::lookup_new_symcache(&symcache, addr).unwrap();
            })
        },
    );
    group.finish();
}

criterion_group!(benches, creation, lookup);
criterion_main!(benches);
