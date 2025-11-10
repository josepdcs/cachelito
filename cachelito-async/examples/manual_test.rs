use std::sync::atomic::{AtomicUsize, Ordering};

static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);

async fn simple(x: u32) -> u32 {
    static ASYNC_CACHE_SIMPLE: once_cell::sync::Lazy<dashmap::DashMap<String, (u32, u64)>> =
        once_cell::sync::Lazy::new(|| dashmap::DashMap::new());

    let __key = {
        let mut __key_parts = Vec::new();
        __key_parts.push(format!("{:?}", x));
        __key_parts.join("|")
    };

    if let Some(__entry_ref) = ASYNC_CACHE_SIMPLE.get(&__key) {
        let __now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let __is_expired = if let Some(__ttl) = None::<u64> {
            __now - __entry_ref.1 > __ttl
        } else {
            false
        };
        if !__is_expired {
            let __cached_value = __entry_ref.0.clone();
            drop(__entry_ref);
            return __cached_value;
        }
        drop(__entry_ref);
        ASYNC_CACHE_SIMPLE.remove(&__key);
    }

    let __result = {
        EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
        x + 1
    };

    let __timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    ASYNC_CACHE_SIMPLE.insert(__key, (__result.clone(), __timestamp));
    __result
}

#[tokio::main]
async fn main() {
    let r1 = simple(42).await;
    assert_eq!(r1, 43);

    let r2 = simple(42).await;
    assert_eq!(r2, 43);

    let count = EXEC_COUNT.load(Ordering::SeqCst);
    println!("Count: {}", count);
}
