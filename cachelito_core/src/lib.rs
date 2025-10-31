use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::thread::LocalKey;

/// Trait for types that can produce a cache key string.
pub trait CacheableKey {
    fn to_cache_key(&self) -> String;
}

impl<T> CacheableKey for T
where
    T: Debug + ?Sized,
{
    fn to_cache_key(&self) -> String {
        format!("{:?}", self)
    }
}

/// Helper to cache results in a thread-local HashMap.
pub fn maybe_cache_result<R: Clone + 'static>(
    cache: &'static LocalKey<RefCell<HashMap<String, R>>>,
    key: &str,
    result: &R,
) {
    cache.with(|c| {
        c.borrow_mut().insert(key.to_string(), result.clone());
    });
}

/// Specialized helper for functions returning `Result<T, E>`.
pub fn maybe_cache_result_result<T, E>(
    cache: &'static LocalKey<RefCell<HashMap<String, Result<T, E>>>>,
    key: &str,
    result: &Result<T, E>,
) where
    T: Clone + Debug + 'static,
    E: Clone + Debug + 'static,
{
    if let Ok(val) = result {
        cache.with(|c| {
            c.borrow_mut().insert(key.to_string(), Ok(val.clone()));
        });
    }
}
