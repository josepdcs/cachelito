use cachelito::cache;

#[cache]
fn my_cached_fn() -> i32 {
    println!("⏳ Executing");
    33
}

fn main() {
    println!("{}", my_cached_fn()); // Execute
    println!("{}", my_cached_fn()); // Use cache
}
