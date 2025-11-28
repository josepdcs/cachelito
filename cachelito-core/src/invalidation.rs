//! # Cache Invalidation
//!
//! Smart cache invalidation mechanisms beyond simple TTL expiration.
//!
//! This module provides fine-grained control over cache invalidation through:
//! - **Tag-based invalidation**: Group related entries and invalidate them together
//! - **Event-driven invalidation**: Trigger invalidation based on events
//! - **Dependency-based invalidation**: Cascade invalidation to dependent caches
//! - **Conditional invalidation**: Custom predicates for invalidation logic _(planned for a future release)_
//!
//! # Examples
//!
//! ```rust
//! use cachelito_core::invalidation::{InvalidationRegistry, InvalidationMetadata};
//!
//! // Register a cache with tags
//! let registry = InvalidationRegistry::global();
//! let metadata = InvalidationMetadata::new(
//!     vec!["user_data".to_string(), "profile".to_string()],
//!     vec![],
//!     vec![]
//! );
//! registry.register("user_profile", metadata);
//!
//! // Invalidate all caches tagged with "user_data"
//! registry.invalidate_by_tag("user_data");
//! ```

use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Strategy for cache invalidation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Invalidate by tag
    Tag(String),
    /// Invalidate by event
    Event(String),
    /// Invalidate by dependency
    Dependency(String),
}

/// Metadata about cache invalidation configuration
#[derive(Debug, Clone)]
pub struct InvalidationMetadata {
    /// Tags associated with this cache
    pub tags: Vec<String>,
    /// Events that trigger invalidation
    pub events: Vec<String>,
    /// Dependencies that trigger cascade invalidation
    pub dependencies: Vec<String>,
}

impl InvalidationMetadata {
    /// Create new invalidation metadata
    pub fn new(tags: Vec<String>, events: Vec<String>, dependencies: Vec<String>) -> Self {
        Self {
            tags,
            events,
            dependencies,
        }
    }

    /// Check if metadata has any invalidation rules
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.events.is_empty() && self.dependencies.is_empty()
    }
}

/// Registry for managing cache invalidation
///
/// This struct maintains mappings between tags/events/dependencies and cache names,
/// allowing efficient invalidation of related caches.
pub struct InvalidationRegistry {
    /// Map from tag to set of cache names
    tag_to_caches: RwLock<HashMap<String, HashSet<String>>>,
    /// Map from event to set of cache names
    event_to_caches: RwLock<HashMap<String, HashSet<String>>>,
    /// Map from dependency to set of dependent cache names
    dependency_to_caches: RwLock<HashMap<String, HashSet<String>>>,
    /// Map from cache name to its metadata
    cache_metadata: RwLock<HashMap<String, InvalidationMetadata>>,
    /// Callbacks for invalidation actions (cache_name -> invalidation function)
    invalidation_callbacks: RwLock<HashMap<String, Arc<dyn Fn() + Send + Sync>>>,
}

impl InvalidationRegistry {
    /// Create a new empty invalidation registry
    fn new() -> Self {
        Self {
            tag_to_caches: RwLock::new(HashMap::new()),
            event_to_caches: RwLock::new(HashMap::new()),
            dependency_to_caches: RwLock::new(HashMap::new()),
            cache_metadata: RwLock::new(HashMap::new()),
            invalidation_callbacks: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global invalidation registry
    pub fn global() -> &'static InvalidationRegistry {
        static INSTANCE: std::sync::OnceLock<InvalidationRegistry> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(InvalidationRegistry::new)
    }

    /// Register a cache with its invalidation metadata
    ///
    /// # Arguments
    ///
    /// * `cache_name` - Unique name of the cache
    /// * `metadata` - Invalidation metadata (tags, events, dependencies)
    pub fn register(&self, cache_name: &str, metadata: InvalidationMetadata) {
        // Register tags
        {
            let mut tag_map = self.tag_to_caches.write();
            for tag in &metadata.tags {
                tag_map
                    .entry(tag.clone())
                    .or_insert_with(HashSet::new)
                    .insert(cache_name.to_string());
            }
        }

        // Register events
        {
            let mut event_map = self.event_to_caches.write();
            for event in &metadata.events {
                event_map
                    .entry(event.clone())
                    .or_insert_with(HashSet::new)
                    .insert(cache_name.to_string());
            }
        }

        // Register dependencies
        {
            let mut dep_map = self.dependency_to_caches.write();
            for dep in &metadata.dependencies {
                dep_map
                    .entry(dep.clone())
                    .or_insert_with(HashSet::new)
                    .insert(cache_name.to_string());
            }
        }

        // Store metadata
        self.cache_metadata
            .write()
            .insert(cache_name.to_string(), metadata);
    }

    /// Register an invalidation callback for a cache
    ///
    /// This callback will be invoked when the cache needs to be invalidated.
    ///
    /// # Arguments
    ///
    /// * `cache_name` - Name of the cache
    /// * `callback` - Function to call when invalidating
    pub fn register_callback<F>(&self, cache_name: &str, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.invalidation_callbacks
            .write()
            .insert(cache_name.to_string(), Arc::new(callback));
    }

    /// Invalidate all caches associated with a tag
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to invalidate
    ///
    /// # Returns
    ///
    /// Number of caches invalidated
    pub fn invalidate_by_tag(&self, tag: &str) -> usize {
        let cache_names = self
            .tag_to_caches
            .read()
            .get(tag)
            .cloned()
            .unwrap_or_default();

        self.invalidate_caches(&cache_names)
    }

    /// Invalidate all caches associated with an event
    ///
    /// # Arguments
    ///
    /// * `event` - The event that occurred
    ///
    /// # Returns
    ///
    /// Number of caches invalidated
    pub fn invalidate_by_event(&self, event: &str) -> usize {
        let cache_names = self
            .event_to_caches
            .read()
            .get(event)
            .cloned()
            .unwrap_or_default();

        self.invalidate_caches(&cache_names)
    }

    /// Invalidate all dependent caches when a dependency changes
    ///
    /// # Arguments
    ///
    /// * `dependency` - The dependency that changed
    ///
    /// # Returns
    ///
    /// Number of caches invalidated
    pub fn invalidate_by_dependency(&self, dependency: &str) -> usize {
        let cache_names = self
            .dependency_to_caches
            .read()
            .get(dependency)
            .cloned()
            .unwrap_or_default();

        self.invalidate_caches(&cache_names)
    }

    /// Invalidate a specific cache by name
    ///
    /// # Arguments
    ///
    /// * `cache_name` - Name of the cache to invalidate
    ///
    /// # Returns
    ///
    /// `true` if the cache was found and invalidated
    pub fn invalidate_cache(&self, cache_name: &str) -> bool {
        if let Some(callback) = self.invalidation_callbacks.read().get(cache_name) {
            callback();
            true
        } else {
            false
        }
    }

    /// Invalidate multiple caches
    ///
    /// # Arguments
    ///
    /// * `cache_names` - Set of cache names to invalidate
    ///
    /// # Returns
    ///
    /// Number of caches successfully invalidated
    fn invalidate_caches(&self, cache_names: &HashSet<String>) -> usize {
        let callbacks = self.invalidation_callbacks.read();
        let mut count = 0;

        for name in cache_names {
            if let Some(callback) = callbacks.get(name) {
                callback();
                count += 1;
            }
        }

        count
    }

    /// Get all caches associated with a tag
    pub fn get_caches_by_tag(&self, tag: &str) -> Vec<String> {
        self.tag_to_caches
            .read()
            .get(tag)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all caches associated with an event
    pub fn get_caches_by_event(&self, event: &str) -> Vec<String> {
        self.event_to_caches
            .read()
            .get(event)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all dependent caches
    pub fn get_dependent_caches(&self, dependency: &str) -> Vec<String> {
        self.dependency_to_caches
            .read()
            .get(dependency)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Clear all registrations
    pub fn clear(&self) {
        self.tag_to_caches.write().clear();
        self.event_to_caches.write().clear();
        self.dependency_to_caches.write().clear();
        self.cache_metadata.write().clear();
        self.invalidation_callbacks.write().clear();
    }
}

impl Default for InvalidationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global convenience function to invalidate all caches with a given tag
///
/// # Arguments
///
/// * `tag` - The tag to match
///
/// # Returns
///
/// Number of caches invalidated
///
/// # Example
///
/// ```ignore
/// use cachelito_core::invalidate_by_tag;
///
/// let count = invalidate_by_tag("user_data");
/// println!("Invalidated {} caches", count);
/// ```
pub fn invalidate_by_tag(tag: &str) -> usize {
    InvalidationRegistry::global().invalidate_by_tag(tag)
}

/// Global convenience function to invalidate all caches listening to an event
///
/// # Arguments
///
/// * `event` - The event that occurred
///
/// # Returns
///
/// Number of caches invalidated
///
/// # Example
///
/// ```ignore
/// use cachelito_core::invalidate_by_event;
///
/// let count = invalidate_by_event("user_updated");
/// println!("Invalidated {} caches", count);
/// ```
pub fn invalidate_by_event(event: &str) -> usize {
    InvalidationRegistry::global().invalidate_by_event(event)
}

/// Global convenience function to invalidate all dependent caches
///
/// # Arguments
///
/// * `dependency` - The dependency that changed
///
/// # Returns
///
/// Number of caches invalidated
///
/// # Example
///
/// ```ignore
/// use cachelito_core::invalidate_by_dependency;
///
/// let count = invalidate_by_dependency("get_user");
/// println!("Invalidated {} caches", count);
/// ```
pub fn invalidate_by_dependency(dependency: &str) -> usize {
    InvalidationRegistry::global().invalidate_by_dependency(dependency)
}

/// Invalidate a specific cache by its name
///
/// This function invalidates a single cache identified by its name.
///
/// # Arguments
///
/// * `cache_name` - The name of the cache to invalidate
///
/// # Returns
///
/// `true` if the cache was found and invalidated, `false` otherwise
///
/// # Examples
///
/// ```ignore
/// use cachelito_core::invalidate_cache;
///
/// // Invalidate a specific cache:
/// invalidate_cache("get_user_profile");
/// ```
pub fn invalidate_cache(cache_name: &str) -> bool {
    InvalidationRegistry::global().invalidate_cache(cache_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_tag_based_invalidation() {
        let registry = InvalidationRegistry::new();
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = counter1.clone();
        let c2 = counter2.clone();

        // Register two caches with same tag
        registry.register(
            "cache1",
            InvalidationMetadata::new(vec!["user_data".to_string()], vec![], vec![]),
        );
        registry.register(
            "cache2",
            InvalidationMetadata::new(vec!["user_data".to_string()], vec![], vec![]),
        );

        registry.register_callback("cache1", move || {
            c1.fetch_add(1, Ordering::SeqCst);
        });
        registry.register_callback("cache2", move || {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        // Invalidate by tag
        let count = registry.invalidate_by_tag("user_data");
        assert_eq!(count, 2);
        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_based_invalidation() {
        let registry = InvalidationRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        registry.register(
            "cache1",
            InvalidationMetadata::new(vec![], vec!["user_updated".to_string()], vec![]),
        );
        registry.register_callback("cache1", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        let count = registry.invalidate_by_event("user_updated");
        assert_eq!(count, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dependency_based_invalidation() {
        let registry = InvalidationRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        registry.register(
            "cache1",
            InvalidationMetadata::new(vec![], vec![], vec!["get_user".to_string()]),
        );
        registry.register_callback("cache1", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        let count = registry.invalidate_by_dependency("get_user");
        assert_eq!(count, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_get_caches_by_tag() {
        let registry = InvalidationRegistry::new();

        registry.register(
            "cache1",
            InvalidationMetadata::new(vec!["tag1".to_string()], vec![], vec![]),
        );
        registry.register(
            "cache2",
            InvalidationMetadata::new(vec!["tag1".to_string()], vec![], vec![]),
        );

        let caches = registry.get_caches_by_tag("tag1");
        assert_eq!(caches.len(), 2);
        assert!(caches.contains(&"cache1".to_string()));
        assert!(caches.contains(&"cache2".to_string()));
    }

    #[test]
    fn test_invalidate_specific_cache() {
        let registry = InvalidationRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        registry.register_callback("cache1", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        assert!(registry.invalidate_cache("cache1"));
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Non-existent cache
        assert!(!registry.invalidate_cache("cache2"));
    }

    #[test]
    fn test_clear_registry() {
        let registry = InvalidationRegistry::new();
        registry.register("cache1", InvalidationMetadata::new(vec![], vec![], vec![]));
        registry.clear();
        assert!(registry.cache_metadata.read().is_empty());
    }
}
