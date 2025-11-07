use cachelito_core::{CacheEntry, EvictionPolicy, GlobalCache};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::thread;

// Global cache instances for benchmarking
static FIFO_MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static FIFO_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static LRU_MAP: Lazy<Mutex<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static LRU_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sequential");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("FIFO", size), size, |b, &size| {
            b.iter(|| {
                let cache = GlobalCache::new(
                    &FIFO_MAP,
                    &FIFO_ORDER,
                    Some(size),
                    EvictionPolicy::FIFO,
                    None,
                );
                for i in 0..size {
                    cache.insert(&format!("key{}", i), black_box(i as i32));
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("LRU", size), size, |b, &size| {
            b.iter(|| {
                let cache =
                    GlobalCache::new(&LRU_MAP, &LRU_ORDER, Some(size), EvictionPolicy::LRU, None);
                for i in 0..size {
                    cache.insert(&format!("key{}", i), black_box(i as i32));
                }
            });
        });
    }

    group.finish();
}

fn bench_get_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_sequential");

    for size in [10, 100, 1000].iter() {
        // Pre-populate cache
        let cache = GlobalCache::new(
            &FIFO_MAP,
            &FIFO_ORDER,
            Some(*size),
            EvictionPolicy::FIFO,
            None,
        );
        for i in 0..*size {
            cache.insert(&format!("key{}", i), i as i32);
        }

        group.bench_with_input(BenchmarkId::new("FIFO", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    black_box(cache.get(&format!("key{}", i)));
                }
            });
        });

        // LRU cache
        let lru_cache =
            GlobalCache::new(&LRU_MAP, &LRU_ORDER, Some(*size), EvictionPolicy::LRU, None);
        for i in 0..*size {
            lru_cache.insert(&format!("key{}", i), i as i32);
        }

        group.bench_with_input(BenchmarkId::new("LRU", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    black_box(lru_cache.get(&format!("key{}", i)));
                }
            });
        });
    }

    group.finish();
}

fn bench_concurrent_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_reads");

    for num_threads in [2, 4, 8].iter() {
        // Pre-populate cache
        let cache = GlobalCache::new(
            &FIFO_MAP,
            &FIFO_ORDER,
            Some(100),
            EvictionPolicy::FIFO,
            None,
        );
        for i in 0..100 {
            cache.insert(&format!("key{}", i), i as i32);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            thread::spawn(|| {
                                let cache = GlobalCache::new(
                                    &FIFO_MAP,
                                    &FIFO_ORDER,
                                    Some(100),
                                    EvictionPolicy::FIFO,
                                    None,
                                );
                                for i in 0..100 {
                                    black_box(cache.get(&format!("key{}", i % 100)));
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_mixed");

    for num_threads in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|thread_id| {
                            thread::spawn(move || {
                                let cache = GlobalCache::new(
                                    &FIFO_MAP,
                                    &FIFO_ORDER,
                                    Some(100),
                                    EvictionPolicy::FIFO,
                                    None,
                                );
                                for i in 0..50 {
                                    if i % 2 == 0 {
                                        cache.insert(
                                            &format!("key{}", thread_id * 50 + i),
                                            black_box(i as i32),
                                        );
                                    } else {
                                        black_box(cache.get(&format!("key{}", thread_id * 50 + i)));
                                    }
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("eviction");

    group.bench_function("FIFO_eviction", |b| {
        b.iter(|| {
            let cache =
                GlobalCache::new(&FIFO_MAP, &FIFO_ORDER, Some(50), EvictionPolicy::FIFO, None);
            // Insert 100 items in a cache with limit 50
            for i in 0..100 {
                cache.insert(&format!("key{}", i), black_box(i as i32));
            }
        });
    });

    group.bench_function("LRU_eviction", |b| {
        b.iter(|| {
            let cache = GlobalCache::new(&LRU_MAP, &LRU_ORDER, Some(50), EvictionPolicy::LRU, None);
            // Insert 100 items in a cache with limit 50
            for i in 0..100 {
                cache.insert(&format!("key{}", i), black_box(i as i32));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_sequential,
    bench_get_sequential,
    bench_concurrent_reads,
    bench_concurrent_mixed,
    bench_eviction
);
criterion_main!(benches);
