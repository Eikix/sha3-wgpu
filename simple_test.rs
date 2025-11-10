use sha3_core::Sha3Variant;
use sha3_wgpu::{GpuContext, GpuSha3Hasher};

#[tokio::main]
async fn main() {
    println!("Simple GPU SHA-3 test");

    let context = GpuContext::new().await.unwrap();
    println!("Context created");

    let hasher = GpuSha3Hasher::with_persistent_buffers(
        context,
        Sha3Variant::Sha3_256,
        Some((1000, 4096, 32))
    ).unwrap();

    println!("Hasher created, has persistent buffers: {}", hasher.buffers.is_some());

    let input = b"test";
    let inputs = vec![input.as_slice()];

    println!("Starting hash...");
    let start = std::time::Instant::now();
    let result = hasher.hash_batch(&inputs).await.unwrap();
    let elapsed = start.elapsed();

    println!("Hash completed in {:?}", elapsed);
    println!("Result length: {}", result.len());
}
