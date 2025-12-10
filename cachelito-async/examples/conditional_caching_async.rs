use cachelito_async::cache_async;
use std::time::Duration;
use tokio::time::sleep;
fn should_cache_non_empty(_key: &String, result: &Vec<String>) -> bool {
    !result.is_empty()
}
#[cache_async(limit = 100, cache_if = should_cache_non_empty)]
async fn fetch_items_async(category: String) -> Vec<String> {
    println!("Fetching items for category: {}", category);
    sleep(Duration::from_millis(100)).await;
    match category.as_str() {
        "electronics" => vec!["laptop".to_string(), "phone".to_string()],
        "empty" => vec![],
        _ => vec![],
    }
}
#[tokio::main]
async fn main() {
    println!("=== Conditional Caching with cache_if ===");
    let items1 = fetch_items_async("electronics".to_string()).await;
    println!("First call (electronics): {:?}", items1);
    let items2 = fetch_items_async("electronics".to_string()).await;
    println!("Second call (cached): {:?}", items2);
    let items3 = fetch_items_async("empty".to_string()).await;
    println!("First call (empty): {:?}", items3);
    let items4 = fetch_items_async("empty".to_string()).await;
    println!("Second call (NOT cached): {:?}", items4);
}
