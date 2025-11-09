use cachelito_async::cache_async;

#[cache_async]
async fn test_fn(x: u32) -> u32 {
    x + 1
}

fn main() {
    println!("This is for cargo expand only");
}
