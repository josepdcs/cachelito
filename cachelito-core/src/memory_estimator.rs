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
        let base = std::mem::size_of::<Self>();
        let elements: usize = self.iter().map(|item| item.estimate_memory()).sum();
        base + elements
    }
}

// Implement for Option<T>
impl<T> MemoryEstimator for Option<T>
where
    T: MemoryEstimator,
{
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Some(val) => val.estimate_memory(),
                None => 0,
            }
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
                Ok(val) => val.estimate_memory(),
                Err(err) => err.estimate_memory(),
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
        std::mem::size_of::<Self>() + self.0.estimate_memory() + self.1.estimate_memory()
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
            + self.0.estimate_memory()
            + self.1.estimate_memory()
            + self.2.estimate_memory()
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
        let v = vec![1i32, 2, 3, 4, 5];
        let base = std::mem::size_of::<Vec<i32>>();
        let elements = 5 * std::mem::size_of::<i32>();
        assert_eq!(v.estimate_memory(), base + elements);
    }

    #[test]
    fn test_option_memory() {
        let some = Some(42i32);
        let none: Option<i32> = None;

        assert!(some.estimate_memory() > std::mem::size_of::<Option<i32>>());
        assert_eq!(none.estimate_memory(), std::mem::size_of::<Option<i32>>());
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
}
