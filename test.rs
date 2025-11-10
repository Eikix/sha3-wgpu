use sha3_core::Sha3Variant;
use sha3_wgpu::{GpuContext, GpuSha3Hasher};

#[tokio::main]
async fn main() {
    println!("Testing GPU SHA-3...");

    let context = GpuContext::new().await.unwrap();
    let hasher = GpuSha3Hasher::with_persistent_buffers(
        context,
        Sha3Variant::Sha3_256,
        Some((1000, 4096, 32)),
    )
    .unwrap();

    let input = b"test";
    let inputs = vec![input.as_slice()];

    println!("Hashing...");
    let result = hasher.hash_batch(&inputs).await.unwrap();
    println!("Result: {:x}", hex::encode(&result));
}
