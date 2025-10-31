use cachelito::cache;
use cachelito_core::{CacheableKey, DefaultCacheableKey};

#[derive(Debug, Clone)]
struct Product {
    id: u32,
    name: String,
}

// Use default cache key implementation
impl DefaultCacheableKey for Product {}

#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
}

// Custom cache key implementation
impl CacheableKey for User {
    fn to_cache_key(&self) -> String {
        format!("user:{}:{}", self.id, self.name)
    }
}

#[cache]
fn compute_price(p: Product, tax: f64) -> f64 {
    println!("Calculating price for {:?}", p);
    (p.id as f64) * 10.0 * (1.0 + tax)
}

#[cache]
fn risky_operation(x: u32) -> Result<u32, String> {
    println!("Running risky operation for {}", x);
    if x % 2 == 0 {
        Ok(x * 2)
    } else {
        Err(format!("Odd number: {}", x))
    }
}

fn main() {
    let prod = Product {
        id: 1,
        name: "Book".to_string(),
    };

    println!("First call: {}", compute_price(prod.clone(), 0.2));
    println!("Second call (cached): {}", compute_price(prod.clone(), 0.2));

    println!("Result 1: {:?}", risky_operation(2));
    println!("Result 2 (cached): {:?}", risky_operation(2));
    println!("Result 3 (error, not cached): {:?}", risky_operation(3));
}
