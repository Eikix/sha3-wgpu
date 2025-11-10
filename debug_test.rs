use sha3_core::Sha3Variant;
use sha3_wgpu::{GpuContext, GpuSha3Hasher};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing GPU SHA-3 performance...");

    // Create GPU context
    let context = GpuContext::new().await?;
    println!("GPU context created");

    // Create hasher with persistent buffers
    let hasher = GpuSha3Hasher::with_persistent_buffers(
        context,
        Sha3Variant::Sha3_256,
        Some((1000, 4096, 32))
    )?;
    println!("GPU hasher created with persistent buffers: {}", hasher.buffers.is_some());

    // Test data
    let input = b"hello world";
    let inputs = vec![input.as_slice()];

    // Warm up
    println!("Warming up...");
    let _ = hasher.hash_batch(&inputs).await?;

    // Time a single batch
    println!("Timing single hash...");
    let start = Instant::now();
    let result = hasher.hash_batch(&inputs).await?;
    let elapsed = start.elapsed();

    println!("GPU hash took: {:.3}ms", elapsed.as_millis());
    println!("Result length: {}", result.len());
    println!("Result: {:x}", hex::encode(&result));

    // Compare with CPU
    use sha3::{Digest, Sha3_256};
    let start = Instant::now();
    let mut cpu_hasher = Sha3_256::new();
    cpu_hasher.update(input);
    let cpu_result = cpu_hasher.finalize();
    let cpu_elapsed = start.elapsed();

    println!("CPU hash took: {:.3}µs", cpu_elapsed.as_micros());
    println!("Results match: {}", result == cpu_result.as_slice());

    Ok(())
}
