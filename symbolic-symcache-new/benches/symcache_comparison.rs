use std::fs;
use std::ops::Range;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::prelude::*;
use rand::rngs::SmallRng;

use symbolic_symcache_new::{format, lookups};
use symbolic_testutils::fixture;

fn random_addresses(range: &Range<u64>, rng: &mut SmallRng) -> [u64; 1000] {
    let mut addresses = [0; 1000];
    for i in 0..1000 {
        addresses[i] = rng.gen_range(range.clone());
    }
    addresses
}

pub fn creation(c: &mut Criterion) {
    // create the two contexts
    let mut group = c.benchmark_group("Cache creation");
    for path in ["simple.debug", "inlined.debug", "two_inlined.debug"] {
        let file = fs::File::open(fixture(format!("inlining/{}", path))).unwrap();
        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
        group.bench_function(BenchmarkId::new("addr2line", path), |b| {
            b.iter(|| lookups::create_addr2line(&mmap).unwrap())
        });
        group.bench_function(BenchmarkId::new("old symcache", path), |b| {
            b.iter(|| {
                let symcache_buf = lookups::create_symcache(&mmap).unwrap();
                symbolic_symcache::SymCache::parse(&symcache_buf).unwrap();
            })
        });
        group.bench_function(BenchmarkId::new("new symcache", path), |b| {
            b.iter(|| {
                let symcache_buf = lookups::create_new_symcache_dwarf(&mmap).unwrap();
                format::Format::parse(&symcache_buf).unwrap();
            })
        });
    }
    group.finish();
}

pub fn lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("Address lookup");

    for path in ["simple.debug", "inlined.debug", "two_inlined.debug"] {
        let file = fs::File::open(fixture(format!("inlining/{}", path))).unwrap();
        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };

        let object = object::File::parse(mmap.as_ref()).unwrap();
        let executable_range = lookups::get_executable_range(&object);
        let mut rng = SmallRng::seed_from_u64(0);
        let addresses = random_addresses(&executable_range, &mut rng);

        let addr2line_ctx = lookups::create_addr2line(&mmap).unwrap();
        let symcache_buf_old = lookups::create_symcache(&mmap).unwrap();
        let symcache_old = symbolic_symcache::SymCache::parse(&symcache_buf_old).unwrap();
        let symcache_buf_new = lookups::create_new_symcache_dwarf(&mmap).unwrap();
        let symcache_new = format::Format::parse(&symcache_buf_new).unwrap();

        group.bench_function(BenchmarkId::new("addr2line", path), |b| {
            b.iter(|| {
                for addr in addresses {
                    lookups::lookup_addr2line(&addr2line_ctx, addr).unwrap();
                }
            })
        });

        group.bench_function(BenchmarkId::new("old symcache", path), |b| {
            b.iter(|| {
                for addr in addresses {
                    lookups::lookup_symcache(&symcache_old, addr).unwrap();
                }
            })
        });

        group.bench_function(BenchmarkId::new("new symcache", path), |b| {
            b.iter(|| {
                for addr in addresses {
                    lookups::lookup_new_symcache(&symcache_new, addr).unwrap();
                }
            })
        });
    }
    group.finish();
}

criterion_group!(benches, creation, lookup);
criterion_main!(benches);
