/// Trait for estimating the memory size of cached values.
///
/// This trait is used by memory-based cache limits to determine how much
/// memory each cached entry consumes. Implementations should return the
/// total memory footprint including heap allocations.
///
/// # Default Implementation
///
/// The default implementation uses `std::mem::size_of_val()` which only
/// accounts for stack-allocated data. For types with heap allocations
/// (like `String`, `Vec`, `HashMap`, etc.), you should provide a custom
/// implementation.
///
/// # Examples
///
/// ## Using Default Implementation
///
/// ```
/// use cachelito_core::MemoryEstimator;
///
/// #[derive(Clone)]
/// struct SimpleStruct {
///     value: i32,
///     flag: bool,
/// }
///
/// // Use default implementation (stack size only)
/// impl MemoryEstimator for SimpleStruct {}
///
/// let simple = SimpleStruct { value: 42, flag: true };
/// assert_eq!(simple.estimate_memory(), std::mem::size_of::<SimpleStruct>());
/// ```
///
/// ## Custom Implementation for Heap Data
///
/// ```
/// use cachelito_core::MemoryEstimator;
///
/// #[derive(Clone)]
/// struct ComplexStruct {
///     name: String,
///     data: Vec<u8>,
/// }
///
/// impl MemoryEstimator for ComplexStruct {
///     fn estimate_memory(&self) -> usize {
///         std::mem::size_of::<Self>()
///             + self.name.capacity()
///             + self.data.capacity()
///     }
/// }
///
/// let complex = ComplexStruct {
///     name: "test".to_string(),
///     data: vec![1, 2, 3, 4, 5],
/// };
/// // Returns struct size + string capacity + vector capacity
/// let size = complex.estimate_memory();
/// ```
pub trait MemoryEstimator {
    /// Estimates the total memory size of this value in bytes.
    ///
    /// This should include:
    /// - Stack-allocated size (`std::mem::size_of_val`)
    /// - Heap-allocated data (e.g., `String::capacity()`, `Vec::capacity()`)
    /// - Any other dynamically allocated memory
    ///
    /// # Returns
    ///
    /// The estimated memory size in bytes.
    ///
    /// # Note
    ///
    /// The default implementation only accounts for stack size.
    /// Override this method for types with heap allocations.
    fn estimate_memory(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

// Implement MemoryEstimator for common primitive types

impl MemoryEstimator for i8 {}
impl MemoryEstimator for i16 {}
impl MemoryEstimator for i32 {}
impl MemoryEstimator for i64 {}
impl MemoryEstimator for i128 {}
impl MemoryEstimator for isize {}

impl MemoryEstimator for u8 {}
impl MemoryEstimator for u16 {}
impl MemoryEstimator for u32 {}
impl MemoryEstimator for u64 {}
impl MemoryEstimator for u128 {}
impl MemoryEstimator for usize {}

impl MemoryEstimator for f32 {}
impl MemoryEstimator for f64 {}

impl MemoryEstimator for bool {}
impl MemoryEstimator for char {}

impl MemoryEstimator for () {}

// Implement for String (includes heap allocation)
impl MemoryEstimator for String {
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>() + self.capacity()
    }
}

// Implement for Vec<T> (includes heap allocation)
impl<T> MemoryEstimator for Vec<T>
where
    T: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        // Base struct size (stack-allocated Vec metadata)
        let base = size_of::<Self>();

        // Buffer capacity (heap-allocated array of T elements)
        let buffer = self.capacity() * size_of::<T>();

        // Additional heap memory for each element (beyond their stack size)
        // For primitives, this will be 0
        // For types like String/Vec, this counts their heap allocations
        let heap_extras: usize = self
            .iter()
            .map(|item| item.estimate_memory().saturating_sub(size_of_val(item)))
            .sum();

        base + buffer + heap_extras
    }
}

// Implement for Option<T>
impl<T> MemoryEstimator for Option<T>
where
    T: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .as_ref()
                .map_or(0, |val| val.estimate_memory() - std::mem::size_of_val(val))
    }
}

// Implement for Result<T, E>
impl<T, E> MemoryEstimator for Result<T, E>
where
    T: MemoryEstimator,
    E: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Ok(val) => val.estimate_memory() - std::mem::size_of_val(val),
                Err(err) => err.estimate_memory() - std::mem::size_of_val(err),
            }
    }
}

// Implement for tuples
impl<T1, T2> MemoryEstimator for (T1, T2)
where
    T1: MemoryEstimator,
    T2: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>()
            + (self.0.estimate_memory() - std::mem::size_of_val(&self.0))
            + (self.1.estimate_memory() - std::mem::size_of_val(&self.1))
    }
}

impl<T1, T2, T3> MemoryEstimator for (T1, T2, T3)
where
    T1: MemoryEstimator,
    T2: MemoryEstimator,
    T3: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>()
            + (self.0.estimate_memory() - std::mem::size_of_val(&self.0))
            + (self.1.estimate_memory() - std::mem::size_of_val(&self.1))
            + (self.2.estimate_memory() - std::mem::size_of_val(&self.2))
    }
}

// Implement for Box<T>
impl<T> MemoryEstimator for Box<T>
where
    T: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>() + (**self).estimate_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(42i32.estimate_memory(), std::mem::size_of::<i32>());
        assert_eq!(true.estimate_memory(), std::mem::size_of::<bool>());
        assert_eq!(3.14f64.estimate_memory(), std::mem::size_of::<f64>());
    }

    #[test]
    fn test_string_memory() {
        let s = String::from("hello");
        let expected = std::mem::size_of::<String>() + s.capacity();
        assert_eq!(s.estimate_memory(), expected);
    }

    #[test]
    fn test_vec_memory() {
        // Test Vec with primitives (no heap extras)
        let v_primitives = vec![1i32, 2, 3, 4, 5];
        let base = std::mem::size_of::<Vec<i32>>();
        let buffer = v_primitives.capacity() * std::mem::size_of::<i32>();
        assert_eq!(v_primitives.estimate_memory(), base + buffer);

        // Test Vec with heap-allocated types (Strings)
        let v_strings = vec![String::from("hello"), String::from("world")];
        let base_str = size_of::<Vec<String>>();
        let buffer_str = v_strings.capacity() * size_of::<String>();
        let heap_extras: usize = v_strings.iter().map(|s| s.capacity()).sum();
        assert_eq!(
            v_strings.estimate_memory(),
            base_str + buffer_str + heap_extras
        );

        // Test empty Vec
        let v_empty: Vec<i32> = Vec::new();
        assert_eq!(v_empty.estimate_memory(), std::mem::size_of::<Vec<i32>>());

        // Test Vec with nested Vecs
        let v_nested = vec![vec![1, 2], vec![3, 4, 5]];
        let base_nested = size_of::<Vec<Vec<i32>>>();
        let buffer_nested = v_nested.capacity() * size_of::<Vec<i32>>();
        let heap_nested: usize = v_nested
            .iter()
            .map(|inner| inner.capacity() * std::mem::size_of::<i32>())
            .sum();
        assert_eq!(
            v_nested.estimate_memory(),
            base_nested + buffer_nested + heap_nested
        );
    }

    #[test]
    fn test_vec_with_capacity() {
        // Test that we account for capacity, not just length
        let mut v = Vec::with_capacity(100);
        v.push(1i32);
        v.push(2i32);

        let base = std::mem::size_of::<Vec<i32>>();
        let buffer = 100 * std::mem::size_of::<i32>(); // capacity, not length
        assert_eq!(v.estimate_memory(), base + buffer);
        assert!(v.estimate_memory() > base + 2 * std::mem::size_of::<i32>());
    }

    #[test]
    fn test_vec_complex_types() {
        // Test with complex nested structures
        let v = vec![
            (String::from("key1"), vec![1, 2, 3]),
            (String::from("key2"), vec![4, 5, 6, 7, 8]),
        ];

        let base = std::mem::size_of::<Vec<(String, Vec<i32>)>>();
        let buffer = v.capacity() * std::mem::size_of::<(String, Vec<i32>)>();

        let heap_extras: usize = v
            .iter()
            .map(|(s, vec)| s.capacity() + vec.capacity() * std::mem::size_of::<i32>())
            .sum();

        assert_eq!(v.estimate_memory(), base + buffer + heap_extras);
    }

    #[test]
    fn test_option_memory() {
        // Option with primitive type (no heap allocation)
        let some_int = Some(42i32);
        let none: Option<i32> = None;

        assert_eq!(
            some_int.estimate_memory(),
            std::mem::size_of::<Option<i32>>()
        );
        assert_eq!(none.estimate_memory(), std::mem::size_of::<Option<i32>>());

        // Option with heap-allocated type
        let some_string = Some(String::from("hello"));
        let expected =
            std::mem::size_of::<Option<String>>() + some_string.as_ref().unwrap().capacity();
        assert_eq!(some_string.estimate_memory(), expected);
    }

    #[test]
    fn test_custom_struct() {
        #[derive(Clone)]
        struct MyStruct {
            name: String,
            data: Vec<u8>,
        }

        impl MemoryEstimator for MyStruct {
            fn estimate_memory(&self) -> usize {
                std::mem::size_of::<Self>() + self.name.capacity() + self.data.capacity()
            }
        }

        let s = MyStruct {
            name: "test".to_string(),
            data: vec![1, 2, 3],
        };

        let expected = std::mem::size_of::<MyStruct>() + s.name.capacity() + s.data.capacity();
        assert_eq!(s.estimate_memory(), expected);
    }

    #[test]
    fn test_tuple_memory() {
        // Test 2-tuple with primitives (no heap allocation)
        let tuple2 = (42i32, true);
        assert_eq!(tuple2.estimate_memory(), std::mem::size_of::<(i32, bool)>());

        // Test 2-tuple with heap-allocated data
        let tuple_heap = (42i32, String::from("hello"));
        let expected = std::mem::size_of::<(i32, String)>() + tuple_heap.1.capacity();
        assert_eq!(tuple_heap.estimate_memory(), expected);

        // Test 3-tuple with mixed types
        let tuple3 = (42i32, String::from("test"), vec![1u8, 2, 3]);
        let expected = std::mem::size_of::<(i32, String, Vec<u8>)>()
            + tuple3.1.capacity()
            + tuple3.2.capacity() * std::mem::size_of::<u8>();
        assert_eq!(tuple3.estimate_memory(), expected);
    }
}
