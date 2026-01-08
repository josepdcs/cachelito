use cachelito_core::{CacheEntry, EvictionPolicy, GlobalCache};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
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

static LFU_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static LFU_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static ARC_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static ARC_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static RANDOM_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static RANDOM_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

static W_TINYLFU_MAP: Lazy<RwLock<HashMap<String, CacheEntry<i32>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static W_TINYLFU_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

// Memory-intensive cache (String values) to benchmark max_memory eviction
static MEM_MAP: Lazy<RwLock<HashMap<String, CacheEntry<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static MEM_ORDER: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

#[cfg(feature = "stats")]
static FIFO_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static LRU_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static LFU_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static ARC_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static RANDOM_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static W_TINYLFU_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());
#[cfg(feature = "stats")]
static MEM_STATS: Lazy<CacheStats> = Lazy::new(|| CacheStats::new());

// Helper macro to create GlobalCache with or without stats (updated signature: + max_memory + frequency_weight + W-TinyLFU params)
macro_rules! new_fifo_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &FIFO_MAP,
            &FIFO_ORDER,
            $limit,
            None, // max_memory
            EvictionPolicy::FIFO,
            None, // ttl
            None, // frequency_weight
            None, // window_ratio
            None, // sketch_width
            None, // sketch_depth
            None, // decay_interval
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
            None, // max_memory
            EvictionPolicy::LRU,
            None, // ttl
            None, // frequency_weight
            None, // window_ratio
            None, // sketch_width
            None, // sketch_depth
            None, // decay_interval
            #[cfg(feature = "stats")]
            &LRU_STATS,
        )
    };
}

macro_rules! new_lfu_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &LFU_MAP,
            &LFU_ORDER,
            $limit,
            None, // max_memory
            EvictionPolicy::LFU,
            None, // ttl
            None, // frequency_weight
            None, // window_ratio
            None, // sketch_width
            None, // sketch_depth
            None, // decay_interval
            #[cfg(feature = "stats")]
            &LFU_STATS,
        )
    };
}

macro_rules! new_arc_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &ARC_MAP,
            &ARC_ORDER,
            $limit,
            None, // max_memory
            EvictionPolicy::ARC,
            None, // ttl
            None, // frequency_weight
            None, // window_ratio
            None, // sketch_width
            None, // sketch_depth
            None, // decay_interval
            #[cfg(feature = "stats")]
            &ARC_STATS,
        )
    };
}

macro_rules! new_random_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &RANDOM_MAP,
            &RANDOM_ORDER,
            $limit,
            None, // max_memory
            EvictionPolicy::Random,
            None, // ttl
            None, // frequency_weight
            None, // window_ratio
            None, // sketch_width
            None, // sketch_depth
            None, // decay_interval
            #[cfg(feature = "stats")]
            &RANDOM_STATS,
        )
    };
}

macro_rules! new_w_tinylfu_cache {
    ($limit:expr) => {
        GlobalCache::new(
            &W_TINYLFU_MAP,
            &W_TINYLFU_ORDER,
            $limit,
            None, // max_memory
            EvictionPolicy::WTinyLFU,
            None,       // ttl
            None,       // frequency_weight
            Some(0.20), // window_ratio (default 20%)
            None,       // sketch_width
            None,       // sketch_depth
            None,       // decay_interval
            #[cfg(feature = "stats")]
            &W_TINYLFU_STATS,
        )
    };
}

macro_rules! new_mem_cache {
    ($limit:expr, $max_mem:expr) => {
        GlobalCache::new(
            &MEM_MAP,
            &MEM_ORDER,
            $limit,
            $max_mem,
            EvictionPolicy::LRU,
            None, // ttl
            None, // frequency_weight
            None, // window_ratio
            None, // sketch_width
            None, // sketch_depth
            None, // decay_interval
            #[cfg(feature = "stats")]
            &MEM_STATS,
        )
    };
}

// Utility to clear underlying static structures to avoid cross-iteration interference
fn reset_fifo() {
    FIFO_MAP.write().clear();
    FIFO_ORDER.lock().clear();
}
fn reset_lru() {
    LRU_MAP.write().clear();
    LRU_ORDER.lock().clear();
}
fn reset_lfu() {
    LFU_MAP.write().clear();
    LFU_ORDER.lock().clear();
}
fn reset_arc() {
    ARC_MAP.write().clear();
    ARC_ORDER.lock().clear();
}
fn reset_random() {
    RANDOM_MAP.write().clear();
    RANDOM_ORDER.lock().clear();
}
fn reset_w_tinylfu() {
    W_TINYLFU_MAP.write().clear();
    W_TINYLFU_ORDER.lock().clear();
}
fn reset_mem() {
    MEM_MAP.write().clear();
    MEM_ORDER.lock().clear();
}

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sequential");

    for size in [10, 100, 1000].iter() {
        // FIFO
        group.bench_with_input(BenchmarkId::new("FIFO", size), size, |b, &size| {
            b.iter(|| {
                reset_fifo();
                let cache = new_fifo_cache!(Some(size));
                for i in 0..size {
                    cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
                }
            });
        });
        // LRU
        group.bench_with_input(BenchmarkId::new("LRU", size), size, |b, &size| {
            b.iter(|| {
                reset_lru();
                let cache = new_lru_cache!(Some(size));
                for i in 0..size {
                    cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
                }
            });
        });
        // LFU
        group.bench_with_input(BenchmarkId::new("LFU", size), size, |b, &size| {
            b.iter(|| {
                reset_lfu();
                let cache = new_lfu_cache!(Some(size));
                for i in 0..size {
                    cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
                }
            });
        });
        // ARC
        group.bench_with_input(BenchmarkId::new("ARC", size), size, |b, &size| {
            b.iter(|| {
                reset_arc();
                let cache = new_arc_cache!(Some(size));
                for i in 0..size {
                    cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
                }
            });
        });
        // Random
        group.bench_with_input(BenchmarkId::new("Random", size), size, |b, &size| {
            b.iter(|| {
                reset_random();
                let cache = new_random_cache!(Some(size));
                for i in 0..size {
                    cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
                }
            });
        });
    }
    group.finish();
}

fn bench_get_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_sequential");

    for size in [10, 100, 1000].iter() {
        // Pre-populate caches
        let fifo_cache = new_fifo_cache!(Some(*size));
        for i in 0..*size {
            fifo_cache.insert(&format!("key{}", i), i as i32);
        }
        let lru_cache = new_lru_cache!(Some(*size));
        for i in 0..*size {
            lru_cache.insert(&format!("key{}", i), i as i32);
        }
        let lfu_cache = new_lfu_cache!(Some(*size));
        for i in 0..*size {
            lfu_cache.insert(&format!("key{}", i), i as i32);
        }
        let arc_cache = new_arc_cache!(Some(*size));
        for i in 0..*size {
            arc_cache.insert(&format!("key{}", i), i as i32);
        }
        let random_cache = new_random_cache!(Some(*size));
        for i in 0..*size {
            random_cache.insert(&format!("key{}", i), i as i32);
        }

        group.bench_with_input(BenchmarkId::new("FIFO", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    std::hint::black_box(fifo_cache.get(&format!("key{}", i)));
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("LRU", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    std::hint::black_box(lru_cache.get(&format!("key{}", i)));
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("LFU", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    std::hint::black_box(lfu_cache.get(&format!("key{}", i)));
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("ARC", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    std::hint::black_box(arc_cache.get(&format!("key{}", i)));
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("Random", size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    std::hint::black_box(random_cache.get(&format!("key{}", i)));
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
                                    std::hint::black_box(cache.get(&format!("key{}", i % 100)));
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
                                            std::hint::black_box(i as i32),
                                        );
                                    } else {
                                        std::hint::black_box(
                                            cache.get(&format!("key{}", thread_id * 50 + i)),
                                        );
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
            for i in 0..100 {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });
    group.bench_function("LRU_eviction", |b| {
        b.iter(|| {
            let cache = new_lru_cache!(Some(50));
            for i in 0..100 {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });
    group.bench_function("LFU_eviction", |b| {
        b.iter(|| {
            let cache = new_lfu_cache!(Some(50));
            for i in 0..100 {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });
    group.bench_function("ARC_eviction", |b| {
        b.iter(|| {
            let cache = new_arc_cache!(Some(50));
            for i in 0..100 {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });
    group.finish();
}

fn bench_memory_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_eviction");
    // Limit entries high, rely on max_memory (approx 1MB) with ~100KB strings
    group.bench_function("LRU_memory_eviction", |b| {
        b.iter(|| {
            reset_mem();
            let cache = new_mem_cache!(None, Some(1 * 1024 * 1024)); // 1MB
            for i in 0..15 {
                // ~ exceed memory
                let val = "X".repeat(100_000); // ~100KB
                cache.insert(&format!("k{}", i), val);
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
                                    std::hint::black_box(cache.get(&format!("key{}", i)));
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
                                            std::hint::black_box(i as i32),
                                        );
                                    } else {
                                        // 90% reads
                                        std::hint::black_box(cache.get(&format!("key{}", i % 50)));
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

fn bench_random_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_vs_other_policies");

    // Benchmark eviction-heavy workload: small cache, many inserts
    let cache_limit = 50;
    let num_insertions = 500; // 10x the cache size

    group.bench_function("Random_eviction_heavy", |b| {
        b.iter(|| {
            reset_random();
            let cache = new_random_cache!(Some(cache_limit));
            for i in 0..num_insertions {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });

    group.bench_function("FIFO_eviction_heavy", |b| {
        b.iter(|| {
            reset_fifo();
            let cache = new_fifo_cache!(Some(cache_limit));
            for i in 0..num_insertions {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });

    group.bench_function("LRU_eviction_heavy", |b| {
        b.iter(|| {
            reset_lru();
            let cache = new_lru_cache!(Some(cache_limit));
            for i in 0..num_insertions {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });

    group.bench_function("LFU_eviction_heavy", |b| {
        b.iter(|| {
            reset_lfu();
            let cache = new_lfu_cache!(Some(cache_limit));
            for i in 0..num_insertions {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });

    group.bench_function("ARC_eviction_heavy", |b| {
        b.iter(|| {
            reset_arc();
            let cache = new_arc_cache!(Some(cache_limit));
            for i in 0..num_insertions {
                cache.insert(&format!("key{}", i), std::hint::black_box(i as i32));
            }
        });
    });

    group.finish();
}

/// Benchmark W-TinyLFU vs other policies with hot/cold data pattern
fn bench_w_tinylfu_hot_cold(c: &mut Criterion) {
    let mut group = c.benchmark_group("w_tinylfu_hot_cold_pattern");
    let cache_limit = 100;
    let hot_keys: Vec<String> = (0..10).map(|i| format!("hot_{}", i)).collect();
    let cold_keys: Vec<String> = (0..200).map(|i| format!("cold_{}", i)).collect();

    // FIFO baseline
    group.bench_function("FIFO", |b| {
        b.iter(|| {
            reset_fifo();
            let cache = new_fifo_cache!(Some(cache_limit));

            // Access hot keys multiple times
            for _ in 0..10 {
                for key in &hot_keys {
                    cache.insert(key, 1);
                }
            }

            // Access cold keys once
            for key in &cold_keys {
                cache.insert(key, 1);
            }

            // Re-access hot keys - measure cache hits
            for key in &hot_keys {
                let _ = cache.get(key);
            }
        });
    });

    // LRU
    group.bench_function("LRU", |b| {
        b.iter(|| {
            reset_lru();
            let cache = new_lru_cache!(Some(cache_limit));

            for _ in 0..10 {
                for key in &hot_keys {
                    cache.insert(key, 1);
                }
            }

            for key in &cold_keys {
                cache.insert(key, 1);
            }

            for key in &hot_keys {
                let _ = cache.get(key);
            }
        });
    });

    // LFU
    group.bench_function("LFU", |b| {
        b.iter(|| {
            reset_lfu();
            let cache = new_lfu_cache!(Some(cache_limit));

            for _ in 0..10 {
                for key in &hot_keys {
                    cache.insert(key, 1);
                }
            }

            for key in &cold_keys {
                cache.insert(key, 1);
            }

            for key in &hot_keys {
                let _ = cache.get(key);
            }
        });
    });

    // W-TinyLFU
    group.bench_function("W_TinyLFU", |b| {
        b.iter(|| {
            reset_w_tinylfu();
            let cache = new_w_tinylfu_cache!(Some(cache_limit));

            for _ in 0..10 {
                for key in &hot_keys {
                    cache.insert(key, 1);
                }
            }

            for key in &cold_keys {
                cache.insert(key, 1);
            }

            for key in &hot_keys {
                let _ = cache.get(key);
            }
        });
    });

    group.finish();
}

/// Benchmark W-TinyLFU with different window ratios
fn bench_w_tinylfu_window_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("w_tinylfu_window_ratios");
    let iterations = 1000;

    for window_ratio in [0.1, 0.2, 0.3, 0.4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("ratio_{:.1}", window_ratio)),
            window_ratio,
            |b, &ratio| {
                b.iter(|| {
                    reset_w_tinylfu();
                    let cache = GlobalCache::new(
                        &W_TINYLFU_MAP,
                        &W_TINYLFU_ORDER,
                        Some(100),
                        None,
                        EvictionPolicy::WTinyLFU,
                        None,
                        None,
                        Some(ratio),
                        None,
                        None,
                        None,
                        #[cfg(feature = "stats")]
                        &W_TINYLFU_STATS,
                    );

                    for i in 0..iterations {
                        cache.insert(&format!("key{}", i % 150), i);
                        if i % 10 == 0 {
                            cache.get(&format!("key{}", i % 50));
                        }
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
    bench_read_heavy_workload,
    bench_memory_eviction,
    bench_random_eviction,
    bench_w_tinylfu_hot_cold,
    bench_w_tinylfu_window_ratios
);
criterion_main!(benches);
