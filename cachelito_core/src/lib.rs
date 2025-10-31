use std::{cell::RefCell, collections::HashMap, fmt::Debug, thread::LocalKey};

/// Trait defining how to generate a cache key for a given type
pub trait CacheableKey {
    fn to_cache_key(&self) -> String;
}

/// Marker trait for types that want to use the *default* cache key behavior
///
/// Implement this for any type that should automatically get a cache key
/// derived from its `Debug` representation.
pub trait DefaultCacheableKey: Debug {}

/// Blanket implementation for any type that explicitly opts in via `DefaultCacheableKey`
impl<T> CacheableKey for T
where
    T: DefaultCacheableKey + ?Sized,
{
    fn to_cache_key(&self) -> String {
        format!("{:?}", self)
    }
}

// Numeric types
impl DefaultCacheableKey for u8 {}
impl DefaultCacheableKey for u16 {}
impl DefaultCacheableKey for u32 {}
impl DefaultCacheableKey for u64 {}
impl DefaultCacheableKey for u128 {}
impl DefaultCacheableKey for usize {}

impl DefaultCacheableKey for i8 {}
impl DefaultCacheableKey for i16 {}
impl DefaultCacheableKey for i32 {}
impl DefaultCacheableKey for i64 {}
impl DefaultCacheableKey for i128 {}
impl DefaultCacheableKey for isize {}

impl DefaultCacheableKey for f32 {}
impl DefaultCacheableKey for f64 {}

// Boolean
impl DefaultCacheableKey for bool {}

// Character
impl DefaultCacheableKey for char {}

// String types
impl DefaultCacheableKey for String {}
impl<'a> DefaultCacheableKey for &'a str {}

// Tuples (until 5-tuple)
impl<T1: DefaultCacheableKey> DefaultCacheableKey for (T1,) {}
impl<T1: DefaultCacheableKey, T2: DefaultCacheableKey> DefaultCacheableKey for (T1, T2) {}
impl<T1: DefaultCacheableKey, T2: DefaultCacheableKey, T3: DefaultCacheableKey> DefaultCacheableKey
    for (T1, T2, T3)
{
}
impl<
        T1: DefaultCacheableKey,
        T2: DefaultCacheableKey,
        T3: DefaultCacheableKey,
        T4: DefaultCacheableKey,
    > DefaultCacheableKey for (T1, T2, T3, T4)
{
}
impl<
        T1: DefaultCacheableKey,
        T2: DefaultCacheableKey,
        T3: DefaultCacheableKey,
        T4: DefaultCacheableKey,
        T5: DefaultCacheableKey,
    > DefaultCacheableKey for (T1, T2, T3, T4, T5)
{
}

// Option and Result
impl<T: DefaultCacheableKey> DefaultCacheableKey for Option<T> {}
impl<T: DefaultCacheableKey, E: DefaultCacheableKey> DefaultCacheableKey for Result<T, E> {}

// Vec and slice
impl<T: DefaultCacheableKey> DefaultCacheableKey for Vec<T> {}
impl<T: DefaultCacheableKey> DefaultCacheableKey for &[T] {}

/// Core cache abstraction that stores values in a thread-local HashMap.
///
/// This cache is designed for static thread-local maps declared with `thread_local!`.
pub struct ThreadLocalCache<R: 'static> {
    pub cache: &'static LocalKey<RefCell<HashMap<String, R>>>,
}

impl<R: Clone + 'static> ThreadLocalCache<R> {
    pub fn new(cache: &'static LocalKey<RefCell<HashMap<String, R>>>) -> Self {
        ThreadLocalCache { cache }
    }

    pub fn get(&self, key: &str) -> Option<R> {
        self.cache.with(|c| c.borrow().get(key).cloned())
    }

    pub fn insert(&self, key: &str, value: R) {
        self.cache.with(|c| {
            c.borrow_mut().insert(key.to_string(), value);
        });
    }
}

/// Specialized helper for `Result<T, E>` return types.
/// Only caches successful (`Ok`) results.
impl<T: Clone + Debug + 'static, E: Clone + Debug + 'static> ThreadLocalCache<Result<T, E>> {
    pub fn insert_result(&self, key: &str, value: &Result<T, E>) {
        if let Ok(val) = value {
            self.cache.with(|c| {
                c.borrow_mut().insert(key.to_string(), Ok(val.clone()));
            });
        }
    }
}
