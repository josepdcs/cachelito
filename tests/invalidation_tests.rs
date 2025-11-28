use cachelito::cache;
use cachelito::{invalidate_by_dependency, invalidate_by_event, invalidate_by_tag};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_tag_based_invalidation() {
    static COUNTER1: AtomicUsize = AtomicUsize::new(0);
    static COUNTER2: AtomicUsize = AtomicUsize::new(0);

    #[cache(
        scope = "global",
        tags = ["test_tag"],
        name = "test_tag_fn1"
    )]
    fn tagged_fn1() -> usize {
        COUNTER1.fetch_add(1, Ordering::SeqCst)
    }

    #[cache(
        scope = "global",
        tags = ["test_tag"],
        name = "test_tag_fn2"
    )]
    fn tagged_fn2() -> usize {
        COUNTER2.fetch_add(1, Ordering::SeqCst)
    }

    // First calls
    let v1 = tagged_fn1();
    let v2 = tagged_fn2();
    assert_eq!(v1, 0);
    assert_eq!(v2, 0);

    // Cached calls
    assert_eq!(tagged_fn1(), 0);
    assert_eq!(tagged_fn2(), 0);

    // Invalidate by tag
    let count = invalidate_by_tag("test_tag");
    assert_eq!(count, 2);

    // Should recompute
    let v1_new = tagged_fn1();
    let v2_new = tagged_fn2();
    assert_eq!(v1_new, 1);
    assert_eq!(v2_new, 1);
}

#[test]
fn test_event_based_invalidation() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[cache(
        scope = "global",
        events = ["test_event"],
        name = "test_event_fn"
    )]
    fn event_fn() -> usize {
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    // First call
    let v1 = event_fn();
    assert_eq!(v1, 0);

    // Cached call
    assert_eq!(event_fn(), 0);

    // Trigger event
    let count = invalidate_by_event("test_event");
    assert_eq!(count, 1);

    // Should recompute
    let v2 = event_fn();
    assert_eq!(v2, 1);
}

#[test]
fn test_dependency_based_invalidation() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[cache(
        scope = "global",
        dependencies = ["base_function"],
        name = "test_dep_fn"
    )]
    fn dependent_fn() -> usize {
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    // First call
    let v1 = dependent_fn();
    assert_eq!(v1, 0);

    // Cached call
    assert_eq!(dependent_fn(), 0);

    // Invalidate dependency
    let count = invalidate_by_dependency("base_function");
    assert_eq!(count, 1);

    // Should recompute
    let v2 = dependent_fn();
    assert_eq!(v2, 1);
}

#[test]
fn test_multiple_invalidation_strategies() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[cache(
        scope = "global",
        tags = ["multi_tag"],
        events = ["multi_event"],
        dependencies = ["multi_dep"],
        name = "test_multi_fn"
    )]
    fn multi_fn() -> usize {
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    // First call
    assert_eq!(multi_fn(), 0);
    assert_eq!(multi_fn(), 0);

    // Invalidate by tag
    invalidate_by_tag("multi_tag");
    assert_eq!(multi_fn(), 1);
    assert_eq!(multi_fn(), 1);

    // Invalidate by event
    invalidate_by_event("multi_event");
    assert_eq!(multi_fn(), 2);
    assert_eq!(multi_fn(), 2);

    // Invalidate by dependency
    invalidate_by_dependency("multi_dep");
    assert_eq!(multi_fn(), 3);
    assert_eq!(multi_fn(), 3);
}

#[test]
fn test_selective_invalidation() {
    static COUNTER_A: AtomicUsize = AtomicUsize::new(0);
    static COUNTER_B: AtomicUsize = AtomicUsize::new(0);

    #[cache(
        scope = "global",
        tags = ["tag_a"],
        name = "test_select_fn_a"
    )]
    fn fn_a() -> usize {
        COUNTER_A.fetch_add(1, Ordering::SeqCst)
    }

    #[cache(
        scope = "global",
        tags = ["tag_b"],
        name = "test_select_fn_b"
    )]
    fn fn_b() -> usize {
        COUNTER_B.fetch_add(1, Ordering::SeqCst)
    }

    // Initial calls
    assert_eq!(fn_a(), 0);
    assert_eq!(fn_b(), 0);

    // Both cached
    assert_eq!(fn_a(), 0);
    assert_eq!(fn_b(), 0);

    // Invalidate only tag_a
    invalidate_by_tag("tag_a");

    // fn_a should recompute, fn_b should be cached
    assert_eq!(fn_a(), 1);
    assert_eq!(fn_b(), 0);
}

#[test]
fn test_cascade_invalidation() {
    static COUNTER_BASE: AtomicUsize = AtomicUsize::new(0);
    static COUNTER_DEP1: AtomicUsize = AtomicUsize::new(0);
    static COUNTER_DEP2: AtomicUsize = AtomicUsize::new(0);

    #[cache(scope = "global", name = "test_cascade_base")]
    fn base_fn() -> usize {
        COUNTER_BASE.fetch_add(1, Ordering::SeqCst)
    }

    #[cache(
        scope = "global",
        dependencies = ["test_cascade_base"],
        name = "test_cascade_dep1"
    )]
    fn dep1_fn() -> usize {
        COUNTER_DEP1.fetch_add(1, Ordering::SeqCst)
    }

    #[cache(
        scope = "global",
        dependencies = ["test_cascade_base"],
        name = "test_cascade_dep2"
    )]
    fn dep2_fn() -> usize {
        COUNTER_DEP2.fetch_add(1, Ordering::SeqCst)
    }

    // Initial calls
    assert_eq!(base_fn(), 0);
    assert_eq!(dep1_fn(), 0);
    assert_eq!(dep2_fn(), 0);

    // All cached
    assert_eq!(base_fn(), 0);
    assert_eq!(dep1_fn(), 0);
    assert_eq!(dep2_fn(), 0);

    // Invalidate base - should cascade to dependents
    let count = invalidate_by_dependency("test_cascade_base");
    assert_eq!(count, 2); // Both dep1 and dep2

    // Dependents should recompute
    assert_eq!(dep1_fn(), 1);
    assert_eq!(dep2_fn(), 1);

    // Base should still be cached
    assert_eq!(base_fn(), 0);
}
