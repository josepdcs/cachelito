use std::time::Duration;

async fn test() {
    tokio::time::sleep(Duration::from_millis(10)).await;
}

#[tokio::main]
async fn main() {
    println!("=== Testing std::time::Instant ===");
    let start = std::time::Instant::now();
    test().await;
    let elapsed = start.elapsed();

    println!("elapsed = {:?}", elapsed);
    println!("as_millis() = {}", elapsed.as_millis());

    println!("\n=== Testing tokio::time::Instant ===");
    let start = tokio::time::Instant::now();
    test().await;
    let elapsed = start.elapsed();

    println!("elapsed = {:?}", elapsed);
    println!("as_millis() = {}", elapsed.as_millis());
}
