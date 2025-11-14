use cachelito::cache;
use cachelito::MemoryEstimator;
#[derive(Clone, Debug)]
struct LargeData {
    id: u64,
    payload: Vec<u8>,
    metadata: String,
}
impl MemoryEstimator for LargeData {
    fn estimate_memory(&self) -> usize {
        std::mem::size_of::<Self>() + self.payload.capacity() * std::mem::size_of::<u8>() + self.metadata.capacity()
    }
}
#[cache(limit = 10, policy = "lru")]
fn process_data(id: u64) -> LargeData {
    LargeData {
        id,
        payload: vec![0u8; 1024 * 1024],
        metadata: format!("Data for ID {}", id),
    }
}
fn main() {
    println!("=== Memory Estimation Example ===\n");
    let data1 = process_data(1);
    let data2 = process_data(2);
    let data3 = process_data(3);
    let size1 = data1.estimate_memory();
    let size2 = data2.estimate_memory();
    let size3 = data3.estimate_memory();
    println!(
        "Entry 1 size: {} bytes ({:.2} MB)",
        size1,
        size1 as f64 / 1024.0 / 1024.0
    );
    println!(
        "Entry 2 size: {} bytes ({:.2} MB)",
        size2,
        size2 as f64 / 1024.0 / 1024.0
    );
    println!(
        "Entry 3 size: {} bytes ({:.2} MB)",
        size3,
        size3 as f64 / 1024.0 / 1024.0
    );
    let total = size1 + size2 + size3;
    println!(
        "\nTotal memory: {} bytes ({:.2} MB)",
        total,
        total as f64 / 1024.0 / 1024.0
    );
    println!("\n=== Built-in Implementations ===\n");
    let string = String::from("Hello, World!");
    println!("String size: {} bytes", string.estimate_memory());
    let vector = vec![1i32, 2, 3, 4, 5];
    println!("Vec<i32> size: {} bytes", vector.estimate_memory());
    println!("\nNote: Memory-based limits coming in v0.9.0!");
    println!("For now, use entry-based limits with 'limit = N' parameter.");
}
