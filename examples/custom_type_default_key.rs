//! # Custom Type with Default Cache Key Example
//!
//! This example demonstrates using custom types with the default cache key
//! implementation, which generates keys based on the `Debug` trait representation.
use cachelito::cache;
use cachelito_core::DefaultCacheableKey;
/// Represents a product in an inventory system.
///
/// This type demonstrates using the default cache key implementation,
/// which generates keys based on the `Debug` trait representation.
#[derive(Debug, Clone)]
struct Product {
    /// Unique product identifier
    id: u32,
    /// Human-readable product name
    name: String,
}
// Enable default cache key implementation for Product
// The cache key will be: format!("{:?}", product)
impl DefaultCacheableKey for Product {}
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
#[cache]
fn compute_price(p: Product, tax: f64) -> f64 {
    println!("Calculating price for {:?}", p);
    (p.id as f64) * 10.0 * (1.0 + tax)
}
fn main() {
    println!("=== Custom Type with Default Cache Key Example ===\n");
    // Create product instances
    let prod1 = Product {
        id: 1,
        name: "Book".to_string(),
    };
    let prod2 = Product {
        id: 2,
        name: "Laptop".to_string(),
    };
    println!("--- Testing Product Price Caching ---");
    // First call with prod1: computes and caches the result
    println!("Computing price for prod1 (first time):");
    let price1 = compute_price(prod1.clone(), 0.2);
    println!("Result: ${:.2}\n", price1);
    // Second call with prod1: returns cached result (no "Calculating" message)
    println!("Getting price for prod1 (should be cached):");
    let cached_price1 = compute_price(prod1.clone(), 0.2);
    println!("Result: ${:.2}\n", cached_price1);
    // First call with prod2: computes and caches the result
    println!("Computing price for prod2 (first time):");
    let price2 = compute_price(prod2.clone(), 0.2);
    println!("Result: ${:.2}\n", price2);
    // Verify cache works correctly
    assert_eq!(price1, cached_price1);
    assert_eq!(price1, 12.0); // (1 * 10.0) * 1.2
    assert_eq!(price2, 24.0); // (2 * 10.0) * 1.2
    println!("✅ Custom Type with Default Cache Key Test PASSED");
}
