//! Basic example of using sha3-wgpu for GPU-accelerated batch SHA-3 hashing

use sha3_core::Sha3Variant;
use sha3_wgpu::{GpuContext, GpuSha3Hasher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== GPU-Accelerated SHA-3 Batch Hashing Example ===\n");

    // Initialize GPU context
    println!("Initializing GPU...");
    let context = GpuContext::new().await?;
    println!("GPU initialized: {:?}", context.adapter_info().name);
    println!("Backend: {:?}\n", context.adapter_info().backend);

    // Create SHA-3 256 hasher
    println!("Creating SHA3-256 hasher...");
    let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_256)?;
    println!("Hasher created!\n");

    // Prepare batch inputs
    let inputs = vec![
        b"Hello, GPU-accelerated SHA-3!".as_slice(),
        b"Batch processing is efficient".as_slice(),
        b"GPU hashing rocks!".as_slice(),
        b"WGSL + wgpu-rs = fast".as_slice(),
        b"Node.js and Bun support".as_slice(),
    ];

    println!("Hashing {} inputs in one GPU batch...\n", inputs.len());

    // Perform batch hashing
    let results = hasher.hash_batch(&inputs).await?;

    // Display results
    let output_size = hasher.variant().output_bytes();
    for (i, (input, hash)) in inputs.iter().zip(results.chunks(output_size)).enumerate() {
        let input_str = String::from_utf8_lossy(input);
        let hash_hex = hex::encode(hash);
        println!("Hash {}: {}", i + 1, input_str);
        println!("       {}\n", hash_hex);
    }

    println!("=== Batch hashing completed successfully! ===");

    Ok(())
}

// Add hex dependency to Cargo.toml for this example
#[allow(dead_code)]
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    }
}
