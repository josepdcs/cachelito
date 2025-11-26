use crate::CacheEntry;
use parking_lot::RwLockWriteGuard;
use std::collections::{HashMap, VecDeque};

/// Moves a key to the end of the order queue (marks as most recently used).
///
/// This utility function is used by LRU (Least Recently Used) and ARC (Adaptive Replacement Cache)
/// eviction policies to update the access order when a cache entry is accessed.
///
/// # Arguments
///
/// * `order` - A mutable reference to the order queue containing cache keys
/// * `key` - The key to move to the end of the queue
///
/// # Behavior
///
/// - If the key exists in the queue, it is removed from its current position and added to the end
/// - If the key doesn't exist, the queue remains unchanged
/// - If the key is already at the end, it's still removed and re-added (maintaining consistency)
///
/// # Performance
///
/// This operation has O(n) time complexity where n is the number of elements in the queue:
/// - Finding the position: O(n)
/// - Removing the element: O(n) in worst case
/// - Pushing to the back: O(1)
///
/// # Examples
///
/// ```
/// use std::collections::VecDeque;
/// use cachelito_core::utils::move_key_to_end;
///
/// let mut order = VecDeque::from(vec!["key1".to_string(), "key2".to_string(), "key3".to_string()]);
///
/// // Access key2, marking it as most recently used
/// move_key_to_end(&mut order, "key2");
///
/// // Order is now: ["key1", "key3", "key2"]
/// assert_eq!(order.back().unwrap(), "key2");
/// ```
///
/// ```
/// use std::collections::VecDeque;
/// use cachelito_core::utils::move_key_to_end;
///
/// let mut order = VecDeque::from(vec!["key1".to_string(), "key2".to_string()]);
///
/// // Trying to move a non-existent key has no effect
/// move_key_to_end(&mut order, "key3");
///
/// assert_eq!(order.len(), 2);
/// ```
pub fn move_key_to_end(order: &mut VecDeque<String>, key: &str) {
    if let Some(pos) = order.iter().position(|k| k == key) {
        order.remove(pos);
        order.push_back(key.to_string());
    }
}

/// Finds the key with the minimum access frequency in the order queue.
///
/// This utility function is used by LFU (Least Frequently Used) and ARC (Adaptive Replacement Cache)
/// eviction policies to identify the least frequently accessed entry for eviction.
///
/// # Arguments
///
/// * `map` - A reference to the cache map containing entries with their frequency counters
/// * `order` - A reference to the order queue containing cache keys to evaluate
///
/// # Returns
///
/// * `Some(String)` - The key with the minimum frequency count
/// * `None` - If the order queue is empty or no valid keys are found in the map
///
/// # Behavior
///
/// - Iterates through all keys in the order queue
/// - Looks up each key in the map to get its frequency
/// - Tracks the key with the lowest frequency counter
/// - If a key in the order queue doesn't exist in the map, it's skipped
/// - In case of frequency ties, returns the first key encountered with the minimum frequency
///
/// # Performance
///
/// This operation has O(n) time complexity where n is the number of elements in the order queue:
/// - Iterating through the queue: O(n)
/// - Looking up each key in the HashMap: O(1) average case
///
/// # Examples
///
/// ```
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::{CacheEntry, utils::find_min_frequency_key};
/// use std::time::Instant;
///
/// let mut map = HashMap::new();
/// map.insert("key1".to_string(), CacheEntry {
///     value: 100,
///     inserted_at: Instant::now(),
///     frequency: 5,
/// });
/// map.insert("key2".to_string(), CacheEntry {
///     value: 200,
///     inserted_at: Instant::now(),
///     frequency: 2,  // Lowest frequency
/// });
/// map.insert("key3".to_string(), CacheEntry {
///     value: 300,
///     inserted_at: Instant::now(),
///     frequency: 8,
/// });
///
/// let order = VecDeque::from(vec!["key1".to_string(), "key2".to_string(), "key3".to_string()]);
///
/// let min_key = find_min_frequency_key(&map, &order);
/// assert_eq!(min_key, Some("key2".to_string()));
/// ```
///
/// ```
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::utils::find_min_frequency_key;
///
/// let map: HashMap<String, cachelito_core::CacheEntry<i32>> = HashMap::new();
/// let order = VecDeque::new();
///
/// // Empty queue returns None
/// let min_key = find_min_frequency_key(&map, &order);
/// assert_eq!(min_key, None);
/// ```
pub fn find_min_frequency_key<R>(
    map: &HashMap<String, CacheEntry<R>>,
    order: &VecDeque<String>,
) -> Option<String> {
    let mut min_freq_key: Option<String> = None;
    let mut min_freq = u64::MAX;

    for evict_key in order.iter() {
        if let Some(entry) = map.get(evict_key) {
            if entry.frequency < min_freq {
                min_freq = entry.frequency;
                min_freq_key = Some(evict_key.clone());
            }
        }
    }

    min_freq_key
}

/// Removes a key from both the cache map and order queue (global cache version).
///
/// This utility function is used by eviction policies in the global cache to maintain
/// consistency between the cache map (protected by `RwLock`) and the order queue.
/// It ensures that when an entry is evicted, it's removed from both data structures.
///
/// # Arguments
///
/// * `map` - A mutable write guard to the global cache map (protected by `parking_lot::RwLock`)
/// * `order` - A mutable reference to the order queue containing cache keys
/// * `key` - The key to remove from both structures
///
/// # Returns
///
/// * `true` - If the key was removed from either the map or the order queue (or both)
/// * `false` - If the key was not found in either structure
///
/// # Behavior
///
/// - Attempts to remove the key from the cache map
/// - Searches for the key in the order queue and removes it if found
/// - Returns `true` if removed from at least one structure
/// - Safe to call even if the key doesn't exist in one or both structures
///
/// # Performance
///
/// - Map removal: O(1) average case
/// - Order queue removal: O(n) where n is the number of elements in the queue
///
/// # Examples
///
/// ```
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::{CacheEntry, utils::remove_key_from_global_cache};
/// use parking_lot::RwLock;
/// use std::time::Instant;
///
/// let cache = RwLock::new(HashMap::new());
/// let mut order = VecDeque::new();
///
/// // Insert an entry
/// {
///     let mut map = cache.write();
///     map.insert("key1".to_string(), CacheEntry {
///         value: 42,
///         inserted_at: Instant::now(),
///         frequency: 1,
///     });
///     order.push_back("key1".to_string());
/// }
///
/// // Remove the entry
/// let mut map = cache.write();
/// let removed = remove_key_from_global_cache(&mut map, &mut order, "key1");
/// assert!(removed);
/// assert!(!map.contains_key("key1"));
/// assert!(order.is_empty());
/// ```
pub fn remove_key_from_global_cache<R>(
    map: &mut RwLockWriteGuard<HashMap<String, CacheEntry<R>>>,
    order: &mut VecDeque<String>,
    key: &str,
) -> bool {
    let (removed_from_map, removed_from_order) = remove_from_maps(map, order, key);

    removed_from_map || removed_from_order
}

/// Removes a key from both the cache map and order queue (thread-local cache version).
///
/// This utility function is used by eviction policies in the thread-local cache to maintain
/// consistency between the cache map (protected by `RefCell`) and the order queue.
/// It ensures that when an entry is evicted, it's removed from both data structures.
///
/// # Arguments
///
/// * `map` - A mutable reference to the thread-local cache map
/// * `order` - A mutable reference to the order queue containing cache keys
/// * `key` - The key to remove from both structures
///
/// # Returns
///
/// * `true` - If the key was removed from either the map or the order queue (or both)
/// * `false` - If the key was not found in either structure
///
/// # Behavior
///
/// - Attempts to remove the key from the cache map
/// - Searches for the key in the order queue and removes it if found
/// - Returns `true` if removed from at least one structure
/// - Safe to call even if the key doesn't exist in one or both structures
///
/// # Performance
///
/// - Map removal: O(1) average case
/// - Order queue removal: O(n) where n is the number of elements in the queue
///
/// # Examples
///
/// ```
/// use std::collections::{HashMap, VecDeque};
/// use cachelito_core::{CacheEntry, utils::remove_key_from_cache_local};
/// use std::time::Instant;
///
/// let mut map = HashMap::new();
/// let mut order = VecDeque::new();
///
/// // Insert an entry
/// map.insert("key1".to_string(), CacheEntry {
///     value: 42,
///     inserted_at: Instant::now(),
///     frequency: 1,
/// });
/// order.push_back("key1".to_string());
///
/// // Remove the entry
/// let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");
/// assert!(removed);
/// assert!(!map.contains_key("key1"));
/// assert!(order.is_empty());
/// ```
pub fn remove_key_from_cache_local<R>(
    map: &mut HashMap<String, CacheEntry<R>>,
    order: &mut VecDeque<String>,
    key: &str,
) -> bool {
    let (removed_from_map, removed_from_order) = remove_from_maps(map, order, key);

    removed_from_map || removed_from_order
}

/// Internal helper function to remove a key from both the cache map and order queue.
///
/// This private function encapsulates the common logic shared by both `remove_key_from_cache`
/// (for global caches) and `remove_key_from_cache_local` (for thread-local caches).
///
/// # Arguments
///
/// * `map` - A mutable reference to the cache map
/// * `order` - A mutable reference to the order queue
/// * `key` - The key to remove from both structures
///
/// # Returns
///
/// A tuple `(bool, bool)` where:
/// - First element: `true` if the key was removed from the map, `false` otherwise
/// - Second element: `true` if the key was removed from the order queue, `false` otherwise
///
/// # Performance
///
/// - Map removal: O(1) average case
/// - Order queue search and removal: O(n) where n is the number of elements
fn remove_from_maps<R>(
    map: &mut HashMap<String, CacheEntry<R>>,
    order: &mut VecDeque<String>,
    key: &str,
) -> (bool, bool) {
    let removed_from_map = map.remove(key).is_some();
    let removed_from_order = if let Some(pos) = order.iter().position(|k| k == key) {
        order.remove(pos);
        true
    } else {
        false
    };
    (removed_from_map, removed_from_order)
}

/// Finds the key with the lowest ARC (Adaptive Replacement Cache) score for eviction.
///
/// The ARC policy combines recency and frequency by calculating a score for each key:
/// - **Frequency**: How many times the entry has been accessed
/// - **Recency**: Position in the access order (more recent = higher weight)
/// - **Score**: `frequency × position_weight`, where position_weight is higher for recent entries
///
/// The key with the **lowest score** is chosen for eviction, meaning entries that are both
/// infrequently accessed and old are prioritized for removal.
///
/// # Arguments
///
/// * `map` - Reference to the HashMap containing cache entries with frequency counters
/// * `keys_iter` - Iterator over (index, key) tuples representing the access order
///
/// # Returns
///
/// * `Some(K)` - The key with the lowest ARC score (candidate for eviction)
/// * `None` - If the iterator is empty or no valid keys exist in the map
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use std::time::Instant;
/// use cachelito_core::{CacheEntry, utils::find_arc_eviction_key};
///
/// let mut map = HashMap::new();
/// map.insert("recent_freq".to_string(), CacheEntry {
///     value: 2,
///     inserted_at: Instant::now(),
///     frequency: 10, // High frequency
/// });
/// map.insert("old_rare".to_string(), CacheEntry {
///     value: 1,
///     inserted_at: Instant::now(),
///     frequency: 1, // Low frequency
/// });
///
/// // Order: most recent first (recent_freq), oldest last (old_rare)
/// let order = vec!["recent_freq".to_string(), "old_rare".to_string()];
/// let evict_key = find_arc_eviction_key(&map, order.iter().enumerate());
///
/// assert_eq!(evict_key, Some("old_rare".to_string())); // Low frequency + old position
/// ```
pub fn find_arc_eviction_key<'a, K, V, I>(
    map: &HashMap<K, CacheEntry<V>>,
    keys_iter: I,
) -> Option<K>
where
    K: std::hash::Hash + Eq + Clone + 'a,
    V: Clone,
    I: Iterator<Item = (usize, &'a K)>,
{
    let mut best_evict_key: Option<K> = None;
    let mut best_score = f64::MAX;
    let keys_vec: Vec<_> = keys_iter.collect();
    let total_len = keys_vec.len();

    for (idx, evict_key) in keys_vec {
        if let Some(entry) = map.get(evict_key) {
            let frequency = entry.frequency as f64;
            let position_weight = (total_len - idx) as f64;
            let score = frequency * position_weight;

            if score < best_score {
                best_score = score;
                best_evict_key = Some(evict_key.clone());
            }
        }
    }

    best_evict_key
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn create_cache_entry<R>(value: R, frequency: u64) -> CacheEntry<R> {
        CacheEntry {
            value,
            inserted_at: Instant::now(),
            frequency,
        }
    }

    #[test]
    fn test_move_key_to_end_existing_key() {
        let mut order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);
        move_key_to_end(&mut order, "key2");

        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "key1");
        assert_eq!(order[1], "key3");
        assert_eq!(order[2], "key2");
    }

    #[test]
    fn test_move_key_to_end_first_key() {
        let mut order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);
        move_key_to_end(&mut order, "key1");

        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "key2");
        assert_eq!(order[1], "key3");
        assert_eq!(order[2], "key1");
    }

    #[test]
    fn test_move_key_to_end_last_key() {
        let mut order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);
        move_key_to_end(&mut order, "key3");

        // Should remain unchanged since key3 is already at the end
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "key1");
        assert_eq!(order[1], "key2");
        assert_eq!(order[2], "key3");
    }

    #[test]
    fn test_move_key_to_end_nonexistent_key() {
        let mut order = VecDeque::from(vec!["key1".to_string(), "key2".to_string()]);
        move_key_to_end(&mut order, "key3");

        // Should remain unchanged since key3 doesn't exist
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], "key1");
        assert_eq!(order[1], "key2");
    }

    #[test]
    fn test_move_key_to_end_empty_queue() {
        let mut order = VecDeque::new();
        move_key_to_end(&mut order, "key1");

        // Should remain empty
        assert_eq!(order.len(), 0);
    }

    #[test]
    fn test_move_key_to_end_single_key() {
        let mut order = VecDeque::from(vec!["key1".to_string()]);
        move_key_to_end(&mut order, "key1");

        // Should remain unchanged
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], "key1");
    }

    #[test]
    fn test_find_min_frequency_key_basic() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, 5));
        map.insert("key2".to_string(), create_cache_entry(200, 2)); // Lowest
        map.insert("key3".to_string(), create_cache_entry(300, 8));

        let order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, Some("key2".to_string()));
    }

    #[test]
    fn test_find_min_frequency_key_empty_queue() {
        let map: HashMap<String, CacheEntry<i32>> = HashMap::new();
        let order = VecDeque::new();

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, None);
    }

    #[test]
    fn test_find_min_frequency_key_empty_map() {
        let map: HashMap<String, CacheEntry<i32>> = HashMap::new();
        let order = VecDeque::from(vec!["key1".to_string(), "key2".to_string()]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, None);
    }

    #[test]
    fn test_find_min_frequency_key_single_entry() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, 10));

        let order = VecDeque::from(vec!["key1".to_string()]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, Some("key1".to_string()));
    }

    #[test]
    fn test_find_min_frequency_key_tie_returns_first() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, 5));
        map.insert("key2".to_string(), create_cache_entry(200, 3)); // Tied for lowest
        map.insert("key3".to_string(), create_cache_entry(300, 3)); // Tied for lowest

        let order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        let min_key = find_min_frequency_key(&map, &order);
        // Should return the first one encountered (key2)
        assert_eq!(min_key, Some("key2".to_string()));
    }

    #[test]
    fn test_find_min_frequency_key_orphaned_keys() {
        let mut map = HashMap::new();
        map.insert("key2".to_string(), create_cache_entry(200, 5));
        map.insert("key3".to_string(), create_cache_entry(300, 2)); // Lowest

        // Order has key1 which doesn't exist in map
        let order = VecDeque::from(vec![
            "key1".to_string(), // Orphaned key
            "key2".to_string(),
            "key3".to_string(),
        ]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, Some("key3".to_string()));
    }

    #[test]
    fn test_find_min_frequency_key_all_orphaned() {
        let mut map = HashMap::new();
        map.insert("key4".to_string(), create_cache_entry(400, 1));

        // None of the keys in order exist in map
        let order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, None);
    }

    #[test]
    fn test_find_min_frequency_key_zero_frequency() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, 10));
        map.insert("key2".to_string(), create_cache_entry(200, 0)); // Zero frequency
        map.insert("key3".to_string(), create_cache_entry(300, 5));

        let order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, Some("key2".to_string()));
    }

    #[test]
    fn test_find_min_frequency_key_large_frequencies() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, u64::MAX - 1));
        map.insert("key2".to_string(), create_cache_entry(200, u64::MAX)); // Maximum
        map.insert("key3".to_string(), create_cache_entry(300, 1000)); // Lowest

        let order = VecDeque::from(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, Some("key3".to_string()));
    }

    #[test]
    fn test_find_min_frequency_key_different_types() {
        let mut map = HashMap::new();
        map.insert(
            "key1".to_string(),
            create_cache_entry("value1".to_string(), 5),
        );
        map.insert(
            "key2".to_string(),
            create_cache_entry("value2".to_string(), 2),
        );

        let order = VecDeque::from(vec!["key1".to_string(), "key2".to_string()]);

        let min_key = find_min_frequency_key(&map, &order);
        assert_eq!(min_key, Some("key2".to_string()));
    }

    // Tests for remove_key_from_cache_local

    #[test]
    fn test_remove_key_from_cache_local_existing_key() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        // Insert an entry
        map.insert("key1".to_string(), create_cache_entry(100, 1));
        order.push_back("key1".to_string());

        // Remove the entry
        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(removed);
        assert!(!map.contains_key("key1"));
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_local_nonexistent_key() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        map.insert("key1".to_string(), create_cache_entry(100, 1));
        order.push_back("key1".to_string());

        // Try to remove a non-existent key
        let removed = remove_key_from_cache_local(&mut map, &mut order, "key2");

        assert!(!removed);
        assert_eq!(map.len(), 1);
        assert_eq!(order.len(), 1);
    }

    #[test]
    fn test_remove_key_from_cache_local_multiple_entries() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        // Insert multiple entries
        map.insert("key1".to_string(), create_cache_entry(100, 1));
        map.insert("key2".to_string(), create_cache_entry(200, 2));
        map.insert("key3".to_string(), create_cache_entry(300, 3));
        order.push_back("key1".to_string());
        order.push_back("key2".to_string());
        order.push_back("key3".to_string());

        // Remove the middle entry
        let removed = remove_key_from_cache_local(&mut map, &mut order, "key2");

        assert!(removed);
        assert!(!map.contains_key("key2"));
        assert_eq!(map.len(), 2);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], "key1");
        assert_eq!(order[1], "key3");
    }

    #[test]
    fn test_remove_key_from_cache_local_only_in_map() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        // Key exists in map but not in order
        map.insert("key1".to_string(), create_cache_entry(100, 1));

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(removed); // Should return true because it was in the map
        assert!(!map.contains_key("key1"));
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_local_only_in_order() {
        let mut map: HashMap<String, CacheEntry<i32>> = HashMap::new();
        let mut order = VecDeque::new();

        // Key exists in order but not in map (orphaned key scenario)
        order.push_back("key1".to_string());

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(removed); // Should return true because it was in the order queue
        assert!(map.is_empty());
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_local_empty_structures() {
        let mut map: HashMap<String, CacheEntry<i32>> = HashMap::new();
        let mut order: VecDeque<String> = VecDeque::new();

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(!removed);
        assert!(map.is_empty());
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_local_first_in_order() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        map.insert("key1".to_string(), create_cache_entry(100, 1));
        map.insert("key2".to_string(), create_cache_entry(200, 2));
        order.push_back("key1".to_string());
        order.push_back("key2".to_string());

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(removed);
        assert_eq!(map.len(), 1);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], "key2");
    }

    #[test]
    fn test_remove_key_from_cache_local_last_in_order() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        map.insert("key1".to_string(), create_cache_entry(100, 1));
        map.insert("key2".to_string(), create_cache_entry(200, 2));
        order.push_back("key1".to_string());
        order.push_back("key2".to_string());

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key2");

        assert!(removed);
        assert_eq!(map.len(), 1);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], "key1");
    }

    #[test]
    fn test_remove_key_from_cache_local_single_entry() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        map.insert("key1".to_string(), create_cache_entry(100, 1));
        order.push_back("key1".to_string());

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(removed);
        assert!(map.is_empty());
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_local_different_value_types() {
        let mut map = HashMap::new();
        let mut order = VecDeque::new();

        map.insert(
            "key1".to_string(),
            create_cache_entry("string_value".to_string(), 1),
        );
        order.push_back("key1".to_string());

        let removed = remove_key_from_cache_local(&mut map, &mut order, "key1");

        assert!(removed);
        assert!(map.is_empty());
        assert!(order.is_empty());
    }

    // Tests for remove_key_from_cache (global version with RwLock)

    #[test]
    fn test_remove_key_from_cache_existing_key() {
        use parking_lot::RwLock;

        let cache = RwLock::new(HashMap::new());
        let mut order = VecDeque::new();

        // Insert an entry
        {
            let mut map = cache.write();
            map.insert("key1".to_string(), create_cache_entry(100, 1));
            order.push_back("key1".to_string());
        }

        // Remove the entry
        let mut map = cache.write();
        let removed = remove_key_from_global_cache(&mut map, &mut order, "key1");

        assert!(removed);
        assert!(!map.contains_key("key1"));
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_nonexistent_key() {
        use parking_lot::RwLock;

        let cache = RwLock::new(HashMap::new());
        let mut order = VecDeque::new();

        {
            let mut map = cache.write();
            map.insert("key1".to_string(), create_cache_entry(100, 1));
            order.push_back("key1".to_string());
        }

        let mut map = cache.write();
        let removed = remove_key_from_global_cache(&mut map, &mut order, "key2");

        assert!(!removed);
        assert_eq!(map.len(), 1);
        assert_eq!(order.len(), 1);
    }

    #[test]
    fn test_remove_key_from_cache_multiple_entries() {
        use parking_lot::RwLock;

        let cache = RwLock::new(HashMap::new());
        let mut order = VecDeque::new();

        {
            let mut map = cache.write();
            map.insert("key1".to_string(), create_cache_entry(100, 1));
            map.insert("key2".to_string(), create_cache_entry(200, 2));
            map.insert("key3".to_string(), create_cache_entry(300, 3));
            order.push_back("key1".to_string());
            order.push_back("key2".to_string());
            order.push_back("key3".to_string());
        }

        let mut map = cache.write();
        let removed = remove_key_from_global_cache(&mut map, &mut order, "key2");

        assert!(removed);
        assert!(!map.contains_key("key2"));
        assert_eq!(map.len(), 2);
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], "key1");
        assert_eq!(order[1], "key3");
    }

    #[test]
    fn test_remove_key_from_cache_only_in_map() {
        use parking_lot::RwLock;

        let cache = RwLock::new(HashMap::new());
        let mut order = VecDeque::new();

        {
            let mut map = cache.write();
            map.insert("key1".to_string(), create_cache_entry(100, 1));
        }

        let mut map = cache.write();
        let removed = remove_key_from_global_cache(&mut map, &mut order, "key1");

        assert!(removed);
        assert!(!map.contains_key("key1"));
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_only_in_order() {
        use parking_lot::RwLock;

        let cache: RwLock<HashMap<String, CacheEntry<i32>>> = RwLock::new(HashMap::new());
        let mut order = VecDeque::new();

        order.push_back("key1".to_string());

        let mut map = cache.write();
        let removed = remove_key_from_global_cache(&mut map, &mut order, "key1");

        assert!(removed);
        assert!(map.is_empty());
        assert!(order.is_empty());
    }

    #[test]
    fn test_remove_key_from_cache_empty_structures() {
        use parking_lot::RwLock;

        let cache: RwLock<HashMap<String, CacheEntry<i32>>> = RwLock::new(HashMap::new());
        let mut order = VecDeque::new();

        let mut map = cache.write();
        let removed = remove_key_from_global_cache(&mut map, &mut order, "key1");

        assert!(!removed);
        assert!(map.is_empty());
        assert!(order.is_empty());
    }

    #[test]
    fn test_find_arc_eviction_key_empty_order() {
        let map: HashMap<String, CacheEntry<i32>> = HashMap::new();
        let order: Vec<String> = vec![];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        assert_eq!(result, None);
    }

    #[test]
    fn test_find_arc_eviction_key_single_entry() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, 5));

        let order = vec!["key1".to_string()];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        assert_eq!(result, Some("key1".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_low_frequency_wins() {
        let mut map = HashMap::new();
        // Recent entry with high frequency (score = 10 * 2 = 20)
        map.insert("recent_freq".to_string(), create_cache_entry(200, 10));
        // Old entry with low frequency (score = 1 * 1 = 1)
        map.insert("old_rare".to_string(), create_cache_entry(100, 1));

        // Order: recent items first, old items last
        let order = vec!["recent_freq".to_string(), "old_rare".to_string()];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // The old, rarely accessed entry should be evicted
        assert_eq!(result, Some("old_rare".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_recency_matters() {
        let mut map = HashMap::new();
        // Recent entry with same frequency (score = 5 * 2 = 10)
        map.insert("recent".to_string(), create_cache_entry(200, 5));
        // Old entry with same frequency (score = 5 * 1 = 5)
        map.insert("old".to_string(), create_cache_entry(100, 5));

        // Order: recent items first (index 0), old items last
        let order = vec!["recent".to_string(), "old".to_string()];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // The older entry should be evicted (lower score)
        assert_eq!(result, Some("old".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_multiple_entries() {
        let mut map = HashMap::new();
        // Scores: freq * position_weight (position_weight = total_len - idx)
        map.insert("key1".to_string(), create_cache_entry(100, 10)); // 10 * 3 = 30
        map.insert("key2".to_string(), create_cache_entry(200, 5)); // 5 * 2 = 10
        map.insert("key3".to_string(), create_cache_entry(300, 3)); // 3 * 1 = 3

        // Order: most recent first (index 0)
        let order = vec![
            "key1".to_string(), // position 0, weight = 3
            "key2".to_string(), // position 1, weight = 2
            "key3".to_string(), // position 2, weight = 1
        ];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // key3 has the lowest score (3 * 1 = 3)
        assert_eq!(result, Some("key3".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_missing_entries() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), create_cache_entry(100, 5));
        map.insert("key3".to_string(), create_cache_entry(300, 10));

        // key2 is in order but not in map
        // Order: most recent first
        let order = vec!["key3".to_string(), "key2".to_string(), "key1".to_string()];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // Should only consider entries that exist in the map
        // key3: 10 * 3 = 30, key1: 5 * 1 = 5
        assert_eq!(result, Some("key1".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_all_missing() {
        let map: HashMap<String, CacheEntry<i32>> = HashMap::new();
        let order = vec!["key1".to_string(), "key2".to_string()];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // No valid entries
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_arc_eviction_key_zero_frequency() {
        let mut map = HashMap::new();
        map.insert("high_freq".to_string(), create_cache_entry(200, 100));
        map.insert("zero_freq".to_string(), create_cache_entry(100, 0));

        // Order: most recent first
        let order = vec!["high_freq".to_string(), "zero_freq".to_string()];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // Zero frequency entry should have the lowest score (0 * 1 = 0)
        assert_eq!(result, Some("zero_freq".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_complex_scenario() {
        let mut map = HashMap::new();
        // Simulate a realistic cache scenario
        map.insert("user:1".to_string(), create_cache_entry(1, 50)); // Very frequent, old
        map.insert("user:2".to_string(), create_cache_entry(2, 2)); // Rare, middle
        map.insert("user:3".to_string(), create_cache_entry(3, 100)); // Very frequent, recent

        let order = vec![
            "user:1".to_string(), // position 0, weight = 3, score = 50 * 3 = 150
            "user:2".to_string(), // position 1, weight = 2, score = 2 * 2 = 4
            "user:3".to_string(), // position 2, weight = 1, score = 100 * 1 = 100
        ];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // user:2 has the lowest score (rare and not most recent)
        assert_eq!(result, Some("user:2".to_string()));
    }

    #[test]
    fn test_find_arc_eviction_key_with_integer_keys() {
        let mut map = HashMap::new();
        map.insert(1, create_cache_entry("a", 10));
        map.insert(2, create_cache_entry("b", 5));
        map.insert(3, create_cache_entry("c", 20));

        let order = vec![1, 2, 3];

        let result = find_arc_eviction_key(&map, order.iter().enumerate());

        // Key 3: 20 * 1 = 20
        // Key 2: 5 * 2 = 10
        // Key 1: 10 * 3 = 30
        assert_eq!(result, Some(2));
    }
}
