use cachelito::cache;

#[cache] // Global by default
fn simple_fn(x: i32) -> i32 {
    println!("Computing {}", x);
    x * 2
}

fn main() {
    println!("Calling simple_fn(5)...");
    let r1 = simple_fn(5);
    println!("Result: {}", r1);

    println!("\nCalling simple_fn(5) again...");
    let r2 = simple_fn(5);
    println!("Result: {}", r2);

    println!("\nCalling simple_fn(10)...");
    let r3 = simple_fn(10);
    println!("Result: {}", r3);

    #[cfg(feature = "stats")]
    {
        println!("\nGetting stats using stats_registry...");
        if let Some(stats) = cachelito::stats_registry::get("simple_fn") {
            println!("Hits: {}", stats.hits());
            println!("Misses: {}", stats.misses());
            println!("Total: {}", stats.total_accesses());
            println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        } else {
            println!("Stats not found!");
        }
    }
}
