//! # Cachelito Example
//!
//! This example demonstrates various use cases of the Cachelito caching library:
//!
//! 1. Custom types with default cache key implementation
//! 2. Custom types with custom cache key implementation
//! 3. Function caching with complex types
//! 4. Result-based caching (only caches Ok values)

use cachelito::cache;
use cachelito_core::{CacheableKey, DefaultCacheableKey};

/// Represents a product in an inventory system.
///
/// This type demonstrates using the default cache key implementation,
/// which generates keys based on the `Debug` trait representation.
#[derive(Debug, Clone)]
struct Product {
    /// Unique product identifier
    id: u32,
    /// Human-readable product name
    #[allow(dead_code)]
    name: String,
}

// Enable default cache key implementation for Product
// The cache key will be: format!("{:?}", product)
impl DefaultCacheableKey for Product {}

/// Represents a user in the system.
///
/// This type demonstrates implementing a custom cache key for better
/// performance and more control over the cache key format.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct User {
    /// Unique user identifier
    id: u64,
    /// User's display name
    name: String,
}

// Custom cache key implementation for User
// This is more efficient than the default Debug-based approach
// as it only uses the ID and name without Debug formatting overhead
impl CacheableKey for User {
    fn to_cache_key(&self) -> String {
        format!("user:{}:{}", self.id, self.name)
    }
}

/// Computes the final price of a product including tax.
///
/// This function is cached, so repeated calls with the same product
/// and tax rate will return the cached result without recomputation.
///
/// # Arguments
///
/// * `p` - The product to price
/// * `tax` - The tax rate (e.g., 0.2 for 20% tax)
///
/// # Returns
///
/// The final price including tax
///
/// # Examples
///
/// ```no_run
/// # use cachelito::cache;
/// # use cachelito_core::DefaultCacheableKey;
/// # #[derive(Debug, Clone)]
/// # struct Product { id: u32, name: String }
/// # impl DefaultCacheableKey for Product {}
/// # #[cache]
/// # fn compute_price(p: Product, tax: f64) -> f64 {
/// #     (p.id as f64) * 10.0 * (1.0 + tax)
/// # }
/// let product = Product { id: 1, name: "Book".to_string() };
/// let price = compute_price(product.clone(), 0.2);
/// // Second call uses cached result
/// let cached_price = compute_price(product, 0.2);
/// assert_eq!(price, cached_price);
/// ```
#[cache]
fn compute_price(p: Product, tax: f64) -> f64 {
    println!("Calculating price for {:?}", p);
    (p.id as f64) * 10.0 * (1.0 + tax)
}

/// Performs a risky operation that may fail for odd numbers.
///
/// This function demonstrates caching with `Result` types. Only successful
/// (`Ok`) results are cached - errors are not cached and will be recomputed
/// on each call.
///
/// # Arguments
///
/// * `x` - The input number
///
/// # Returns
///
/// * `Ok(x * 2)` if x is even (this result is cached)
/// * `Err(message)` if x is odd (this error is NOT cached)
///
/// # Examples
///
/// ```no_run
/// # use cachelito::cache;
/// # #[cache]
/// # fn risky_operation(x: u32) -> Result<u32, String> {
/// #     if x % 2 == 0 { Ok(x * 2) } else { Err(format!("Odd: {}", x)) }
/// # }
/// // Success - will be cached
/// assert_eq!(risky_operation(2), Ok(4));
/// assert_eq!(risky_operation(2), Ok(4)); // Uses cache
///
/// // Error - will NOT be cached
/// assert!(risky_operation(3).is_err());
/// assert!(risky_operation(3).is_err()); // Recomputes
/// ```
#[cache]
fn risky_operation(x: u32) -> Result<u32, String> {
    println!("Running risky operation for {}", x);
    if x.is_multiple_of(2) {
        Ok(x * 2)
    } else {
        Err(format!("Odd number: {}", x))
    }
}

/// Main entry point demonstrating Cachelito's caching capabilities.
///
/// This function shows:
/// 1. How repeated calls with the same arguments use the cache
/// 2. How Result types only cache Ok values, not Err values
fn main() {
    println!("=== Cachelito Example ===\n");

    // Create a product instance
    let prod = Product {
        id: 1,
        name: "Book".to_string(),
    };

    println!("--- Testing Product Price Caching ---");
    // First call: computes and caches the result
    println!("First call: {}", compute_price(prod.clone(), 0.2));
    // Second call: returns cached result (notice no "Calculating" message)
    println!("Second call (cached): {}", compute_price(prod.clone(), 0.2));

    println!("\n--- Testing Result Caching ---");
    // First call with even number: computes, caches, returns Ok
    println!("Result 1: {:?}", risky_operation(2));
    // Second call with same even number: returns cached Ok (no "Running" message)
    println!("Result 2 (cached): {:?}", risky_operation(2));
    // First call with odd number: computes and returns Err (NOT cached)
    println!("Result 3 (error, not cached): {:?}", risky_operation(3));
    // Second call with same odd number: computes again (notice "Running" message)
    println!("Result 4 (error again): {:?}", risky_operation(3));
}
