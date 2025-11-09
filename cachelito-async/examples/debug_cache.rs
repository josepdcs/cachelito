use cachelito_async::cache_async;
use std::time::Duration;

#[cache_async(limit = 100)]
async fn add(a: u32, b: u32) -> u32 {
    println!("Computing {} + {}", a, b);
    tokio::time::sleep(Duration::from_millis(10)).await;
    a + b
}

#[tokio::main]
async fn main() {
    println!("First call:");
    let start = std::time::Instant::now();
    let r1 = add(1, 2).await;
    println!("Result: {}, time: {:?}\n", r1, start.elapsed());

    println!("Second call (should be instant):");
    let start = std::time::Instant::now();
    let r2 = add(1, 2).await;
    println!("Result: {}, time: {:?}\n", r2, start.elapsed());

    if start.elapsed().as_millis() > 5 {
        println!("❌ CACHE NOT WORKING - took {:?}", start.elapsed());
    } else {
        println!("✅ Cache working correctly");
    }
}
