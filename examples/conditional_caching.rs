// Example demonstrating conditional caching with cache_if attribute
//
// This example shows how to use the cache_if attribute to control when results
// should be cached based on custom predicates.
//
// Examples included:
// 1. Only cache non-empty vectors
// 2. Only cache successful HTTP-like responses (status 2xx)
// 3. Only cache Some values (skip None)
// 4. Only cache large results (> 1KB)
// 5. Cache based on result value (only positive numbers)
// 6. Only cache Ok results with custom predicate
// 7. Default Result behavior - only Ok values cached automatically
// 8. Only cache valid division results (finite numbers, no errors)

use cachelito::cache;

// Example 1: Only cache non-empty vectors
fn should_cache_non_empty(_key: &String, result: &Vec<String>) -> bool {
    !result.is_empty()
}

#[cache(scope = "global", limit = 100, cache_if = should_cache_non_empty)]
fn fetch_items(category: String) -> Vec<String> {
    println!("Fetching items for category: {}", category);

    // Simulate database query that sometimes returns empty results
    match category.as_str() {
        "electronics" => vec!["laptop".to_string(), "phone".to_string()],
        "books" => vec!["novel".to_string(), "textbook".to_string()],
        "empty" => vec![], // This won't be cached
        _ => vec![],
    }
}

// Example 2: Only cache successful HTTP-like responses
#[derive(Debug, Clone)]
struct Response {
    status: u16,
    body: String,
}

fn cache_success(_key: &String, response: &Response) -> bool {
    response.status >= 200 && response.status < 300
}

#[cache(scope = "global", limit = 50, cache_if = cache_success)]
fn api_call(url: String) -> Response {
    println!("Making API call to: {}", url);

    // Simulate API responses
    match url.as_str() {
        "https://api.example.com/users" => Response {
            status: 200,
            body: "User data".to_string(),
        },
        "https://api.example.com/error" => Response {
            status: 500,
            body: "Server error".to_string(), // This won't be cached
        },
        _ => Response {
            status: 404,
            body: "Not found".to_string(), // This won't be cached
        },
    }
}

// Example 3: Only cache Some values
fn cache_some(_key: &String, result: &Option<User>) -> bool {
    result.is_some()
}

#[derive(Debug, Clone)]
struct User {
    id: u32,
    name: String,
}

#[cache(scope = "global", limit = 100, cache_if = cache_some)]
fn find_user(id: u32) -> Option<User> {
    println!("Searching for user with id: {}", id);

    // Simulate database lookup
    if id > 0 && id <= 10 {
        Some(User {
            id,
            name: format!("User {}", id),
        })
    } else {
        None // This won't be cached
    }
}

// Example 4: Only cache large results
fn cache_if_large(_key: &String, data: &Vec<u8>) -> bool {
    data.len() > 1024 // Only cache results larger than 1KB
}

#[cache(scope = "global", limit = 10, cache_if = cache_if_large)]
fn process_data(size: usize) -> Vec<u8> {
    println!("Processing data of size: {}", size);
    vec![0u8; size]
}

// Example 5: Cache based on result value
fn cache_if_positive(_key: &String, value: &i32) -> bool {
    *value > 0
}

#[cache(scope = "thread", cache_if = cache_if_positive)]
fn compute(x: i32, y: i32) -> i32 {
    println!("Computing: {} + {}", x, y);
    x + y
}

// Example 6: Only cache Ok results (custom predicate)
fn cache_only_ok(_key: &String, result: &Result<String, String>) -> bool {
    result.is_ok()
}

#[cache(scope = "global", limit = 50, cache_if = cache_only_ok)]
fn fetch_user_data(user_id: u32) -> Result<String, String> {
    println!("Fetching user data for id: {}", user_id);

    if user_id == 0 {
        Err("Invalid user id".to_string()) // This won't be cached
    } else if user_id > 100 {
        Err("User not found".to_string()) // This won't be cached
    } else {
        Ok(format!("User data for id: {}", user_id)) // This will be cached
    }
}

// Example 7: Default behavior - Result types only cache Ok by default
#[cache(scope = "global", limit = 50)]
fn validate_email(email: String) -> Result<String, String> {
    println!("Validating email: {}", email);

    if email.contains('@') && email.contains('.') {
        Ok(format!("Valid email: {}", email)) // Cached by default
    } else {
        Err(format!("Invalid email format: {}", email)) // NOT cached by default
    }
}

// Example 8: Only cache specific error-free computations
fn cache_valid_division(_key: &String, result: &Result<f64, String>) -> bool {
    matches!(result, Ok(val) if val.is_finite())
}

#[cache(scope = "thread", cache_if = cache_valid_division)]
fn divide(a: f64, b: f64) -> Result<f64, String> {
    println!("Dividing {} / {}", a, b);

    if b == 0.0 {
        Err("Division by zero".to_string()) // This won't be cached
    } else {
        let result = a / b;
        if result.is_finite() {
            Ok(result) // Only cached if finite
        } else {
            Ok(result) // Infinity/NaN won't be cached
        }
    }
}

fn main() {
    println!("=== Example 1: Only cache non-empty vectors ===");
    let items1 = fetch_items("electronics".to_string());
    println!("First call (electronics): {:?}", items1);
    let items2 = fetch_items("electronics".to_string());
    println!("Second call (electronics - cached): {:?}", items2);

    let items3 = fetch_items("empty".to_string());
    println!("First call (empty): {:?}", items3);
    let items4 = fetch_items("empty".to_string());
    println!(
        "Second call (empty - NOT cached, will execute again): {:?}",
        items4
    );

    println!("\n=== Example 2: Only cache successful responses ===");
    let resp1 = api_call("https://api.example.com/users".to_string());
    println!("First call (200): status={}", resp1.status);
    let resp2 = api_call("https://api.example.com/users".to_string());
    println!("Second call (200 - cached): status={}", resp2.status);

    let resp3 = api_call("https://api.example.com/error".to_string());
    println!("First call (500): status={}", resp3.status);
    let resp4 = api_call("https://api.example.com/error".to_string());
    println!(
        "Second call (500 - NOT cached, will execute again): status={}",
        resp4.status
    );

    println!("\n=== Example 3: Only cache Some values ===");
    let user1 = find_user(5);
    println!("First call (id=5): {:?}", user1);
    let user2 = find_user(5);
    println!("Second call (id=5 - cached): {:?}", user2);

    let user3 = find_user(999);
    println!("First call (id=999, None): {:?}", user3);
    let user4 = find_user(999);
    println!(
        "Second call (id=999 - NOT cached, will execute again): {:?}",
        user4
    );

    println!("\n=== Example 4: Only cache large results ===");
    let data1 = process_data(2048);
    println!("First call (2048 bytes): len={}", data1.len());
    let data2 = process_data(2048);
    println!("Second call (2048 - cached): len={}", data2.len());

    let data3 = process_data(512);
    println!("First call (512 bytes): len={}", data3.len());
    let data4 = process_data(512);
    println!(
        "Second call (512 - NOT cached, will execute again): len={}",
        data4.len()
    );

    println!("\n=== Example 5: Cache based on value ===");
    let result1 = compute(5, 3);
    println!("First call (5 + 3 = 8): {}", result1);
    let result2 = compute(5, 3);
    println!("Second call (5 + 3 - cached): {}", result2);

    let result3 = compute(-5, 3);
    println!("First call (-5 + 3 = -2): {}", result3);
    let result4 = compute(-5, 3);
    println!(
        "Second call (-5 + 3 - NOT cached, will execute again): {}",
        result4
    );

    println!("\n=== Example 6: Only cache Ok results (custom predicate) ===");
    let user_data1 = fetch_user_data(42);
    println!("First call (id=42): {:?}", user_data1);
    let user_data2 = fetch_user_data(42);
    println!("Second call (id=42 - cached): {:?}", user_data2);

    let user_data3 = fetch_user_data(0);
    println!("First call (id=0, error): {:?}", user_data3);
    let user_data4 = fetch_user_data(0);
    println!(
        "Second call (id=0 - NOT cached, will execute again): {:?}",
        user_data4
    );

    let user_data5 = fetch_user_data(999);
    println!("First call (id=999, error): {:?}", user_data5);
    let user_data6 = fetch_user_data(999);
    println!(
        "Second call (id=999 - NOT cached, will execute again): {:?}",
        user_data6
    );

    println!("\n=== Example 7: Default Result behavior - only Ok cached ===");
    let email1 = validate_email("user@example.com".to_string());
    println!("First call (valid email): {:?}", email1);
    let email2 = validate_email("user@example.com".to_string());
    println!("Second call (valid email - cached): {:?}", email2);

    let email3 = validate_email("invalid-email".to_string());
    println!("First call (invalid email): {:?}", email3);
    let email4 = validate_email("invalid-email".to_string());
    println!(
        "Second call (invalid email - NOT cached by default): {:?}",
        email4
    );

    println!("\n=== Example 8: Only cache valid division results ===");
    let div1 = divide(10.0, 2.0);
    println!("First call (10.0 / 2.0): {:?}", div1);
    let div2 = divide(10.0, 2.0);
    println!("Second call (10.0 / 2.0 - cached): {:?}", div2);

    let div3 = divide(10.0, 0.0);
    println!("First call (10.0 / 0.0, error): {:?}", div3);
    let div4 = divide(10.0, 0.0);
    println!(
        "Second call (10.0 / 0.0 - NOT cached, will execute again): {:?}",
        div4
    );

    let div5 = divide(f64::MAX, 2.0);
    println!("First call (MAX / 2.0): {:?}", div5);
    let div6 = divide(f64::MAX, 2.0);
    println!("Second call (MAX / 2.0 - cached): {:?}", div6);
}
