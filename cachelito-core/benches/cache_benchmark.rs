use cachelito_core::{CacheEntry, EvictionPolicy, GlobalCache};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, VecDeque};
use std::thread;

#[cfg(feature = "stats")]
use cachelito_core::CacheStats;

// Global cache instances for benchmarking
static FIFO_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static FIFO_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static LRU_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static LRU_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

#[cfg(feature = "stats")]
static FIFO_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static LRU_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

// Helper macro to create GlobalCache with or without stats
macro_rules! new_fifo_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &FIFO_MAP,
            &FIFO_ORDER,
            $limit,
            EvictionPolicy::FIFO,
            None,
            #[cfg(feature = "stats")]
            &FIFO_STATS,
        )
    };
}

macro_rules! new_lru_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &LRU_MAP,
            &LRU_ORDER,
            $limit,
            EvictionPolicy::LRU,
            None,
            #[cfg(feature = "stats")]
            &LRU_STATS,
        )
    };
}

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sequential");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("FIFO", size), size, |b, &size| {
            b.iter(|| {
                let cache = new_fifo_cache!(Some(size));
                for i in 0..size {
                    cache.insert(&format!("key{}", i), black_box(i as i32));
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("LRU", size), size, |b, &size| {
            b.iter(|| {
                let cache = new_lru_cache!(Some(size));
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
        let cache = new_fifo_cache!(Some(*size));
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
        let lru_cache = new_lru_cache!(Some(*size));
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
        let cache = new_fifo_cache!(Some(100));
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
                                let cache = new_fifo_cache!(Some(100));
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
                                let cache = new_fifo_cache!(Some(100));
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
            let cache = new_fifo_cache!(Some(50));
            // Insert 100 items in a cache with limit 50
            for i in 0..100 {
                cache.insert(&format!("key{}", i), black_box(i as i32));
            }
        });
    });

    group.bench_function("LRU_eviction", |b| {
        b.iter(|| {
            let cache = new_lru_cache!(Some(50));
            // Insert 100 items in a cache with limit 50
            for i in 0..100 {
                cache.insert(&format!("key{}", i), black_box(i as i32));
            }
        });
    });

    group.finish();
}

fn bench_rwlock_concurrent_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("rwlock_concurrent_reads");

    // Pre-populate cache
    let cache = new_fifo_cache!(Some(1000));
    for i in 0..1000 {
        cache.insert(&format!("key{}", i), i as i32);
    }

    for num_threads in [2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("pure_reads", num_threads),
            num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            thread::spawn(|| {
                                let cache = new_fifo_cache!(Some(1000));
                                // Pure reads - RwLock allows concurrent access
                                for i in 0..100 {
                                    black_box(cache.get(&format!("key{}", i)));
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

fn bench_read_heavy_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_heavy_workload");

    // 90% reads, 10% writes
    for num_threads in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("90_read_10_write", num_threads),
            num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|thread_id| {
                            thread::spawn(move || {
                                let cache = new_fifo_cache!(Some(100));
                                for i in 0..100 {
                                    if i % 10 == 0 {
                                        // 10% writes
                                        cache.insert(
                                            &format!("key{}", thread_id * 100 + i),
                                            black_box(i as i32),
                                        );
                                    } else {
                                        // 90% reads
                                        black_box(cache.get(&format!("key{}", i % 50)));
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

criterion_group!(
    benches,
    bench_insert_sequential,
    bench_get_sequential,
    bench_concurrent_reads,
    bench_concurrent_mixed,
    bench_eviction,
    bench_rwlock_concurrent_reads,
    bench_read_heavy_workload
);
criterion_main!(benches);
