use cachelito::cache;

#[cache]
fn my_cached_fn() -> i32 {
    println!("⏳ Executing");
    33
}

#[cache]
fn sum(a: i32, b: i32) -> i32 {
    println!("⏳ Executing (a + b)");
    a + b
}

#[cache]
fn fibonacci(n: u32) -> u64 {
    println!("⏳ Executing fibonacci({})", n);
    if n <= 1 {
        return n as u64;
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

#[cache]
fn might_fail(n: i32) -> Result<i32, String> {
    println!("executing might_fail({})", n);
    if n < 0 {
        Err("negative numbers not allowed".into())
    } else {
        Ok(n * 2)
    }
}

fn main() {
    println!("{}", my_cached_fn()); // Execute
    println!("{}", my_cached_fn()); // Use cache

    println!("{}", sum(10, 20)); // Execute
    println!("{}", sum(10, 20)); // Use cache

    println!("{}", fibonacci(10)); // Execute
    println!("{}", fibonacci(10)); // Use cache
}
