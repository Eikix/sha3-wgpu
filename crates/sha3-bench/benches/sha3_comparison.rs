//! Criterion benchmarks comparing GPU vs CPU SHA-3 implementations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sha3::{Digest, Sha3_256};
use sha3_core::Sha3Variant;
use sha3_wgpu::{GpuContext, GpuSha3Hasher};

/// Benchmark CPU SHA-3 (reference implementation)
fn bench_cpu_sha3(data: &[Vec<u8>]) -> Vec<Vec<u8>> {
    data.iter()
        .map(|input| {
            let mut hasher = Sha3_256::new();
            hasher.update(input);
            hasher.finalize().to_vec()
        })
        .collect()
}

/// Benchmark GPU SHA-3 (our implementation)
async fn bench_gpu_sha3(hasher: &GpuSha3Hasher, data: &[&[u8]]) -> Vec<u8> {
    hasher.hash_batch(data).await.unwrap()
}

fn setup_gpu_hasher() -> GpuSha3Hasher {
    pollster::block_on(async {
        let context = GpuContext::new().await.unwrap();
        GpuSha3Hasher::new(context, Sha3Variant::Sha3_256).unwrap()
    })
}

fn benchmark_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha3_batch_comparison");

    // Test different batch sizes
    let batch_sizes = vec![1, 10, 50, 100, 500, 1000];
    let input_size = 64; // 64 bytes per input

    let gpu_hasher = setup_gpu_hasher();

    for batch_size in batch_sizes {
        let data: Vec<Vec<u8>> =
            (0..batch_size).map(|i| format!("test input number {i}").into_bytes()).collect();

        // Ensure all inputs are same length for GPU batching
        let padded_data: Vec<Vec<u8>> = data
            .iter()
            .map(|v| {
                let mut padded = v.clone();
                padded.resize(input_size, 0);
                padded
            })
            .collect();

        let total_bytes = (batch_size * input_size) as u64;
        group.throughput(Throughput::Bytes(total_bytes));

        // Benchmark CPU
        group.bench_with_input(BenchmarkId::new("CPU", batch_size), &padded_data, |b, data| {
            b.iter(|| {
                let results = bench_cpu_sha3(black_box(data));
                black_box(results);
            });
        });

        // Benchmark GPU
        let input_refs: Vec<&[u8]> = padded_data.iter().map(|v| v.as_slice()).collect();
        group.bench_with_input(BenchmarkId::new("GPU", batch_size), &input_refs, |b, data| {
            b.iter(|| {
                let result = pollster::block_on(bench_gpu_sha3(&gpu_hasher, black_box(data)));
                black_box(result);
            });
        });
    }

    group.finish();
}

fn benchmark_input_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha3_input_size_comparison");

    // Test different input sizes with fixed batch size
    let input_sizes = vec![32, 64, 128, 256, 512, 1024, 4096];
    let batch_size = 100;

    let gpu_hasher = setup_gpu_hasher();

    for input_size in input_sizes {
        let data: Vec<Vec<u8>> = (0..batch_size).map(|_| vec![0xAB; input_size]).collect();

        let total_bytes = (batch_size * input_size) as u64;
        group.throughput(Throughput::Bytes(total_bytes));

        // Benchmark CPU
        group.bench_with_input(BenchmarkId::new("CPU", input_size), &data, |b, data| {
            b.iter(|| {
                let results = bench_cpu_sha3(black_box(data));
                black_box(results);
            });
        });

        // Benchmark GPU
        let input_refs: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();
        group.bench_with_input(BenchmarkId::new("GPU", input_size), &input_refs, |b, data| {
            b.iter(|| {
                let result = pollster::block_on(bench_gpu_sha3(&gpu_hasher, black_box(data)));
                black_box(result);
            });
        });
    }

    group.finish();
}

fn benchmark_single_vs_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha3_single_vs_batch");

    let batch_size = 100;
    let input_size = 64;

    let data: Vec<Vec<u8>> = (0..batch_size)
        .map(|i| {
            let mut v = format!("test input {i}").into_bytes();
            v.resize(input_size, 0);
            v
        })
        .collect();

    let gpu_hasher = setup_gpu_hasher();

    // Single hash repeated
    group.bench_function("CPU_single_x100", |b| {
        b.iter(|| {
            for input in &data {
                let mut hasher = Sha3_256::new();
                hasher.update(black_box(input));
                let result = hasher.finalize();
                black_box(result);
            }
        });
    });

    // Batch processing
    group.bench_function("CPU_batch_x100", |b| {
        b.iter(|| {
            let results = bench_cpu_sha3(black_box(&data));
            black_box(results);
        });
    });

    // GPU single (actually batched but small)
    let input_refs: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();
    group.bench_function("GPU_batch_x100", |b| {
        b.iter(|| {
            let result = pollster::block_on(bench_gpu_sha3(&gpu_hasher, black_box(&input_refs)));
            black_box(result);
        });
    });

    group.finish();
}

fn benchmark_large_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha3_large_batch");
    group.sample_size(10); // Reduce sample size for large batches

    let batch_sizes = vec![1000, 5000, 10000];
    let input_size = 64;

    let gpu_hasher = setup_gpu_hasher();

    for batch_size in batch_sizes {
        let data: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| {
                let mut v = format!("input {i}").into_bytes();
                v.resize(input_size, 0);
                v
            })
            .collect();

        let total_bytes = (batch_size * input_size) as u64;
        group.throughput(Throughput::Bytes(total_bytes));

        // CPU benchmark
        group.bench_with_input(BenchmarkId::new("CPU", batch_size), &data, |b, data| {
            b.iter(|| {
                let results = bench_cpu_sha3(black_box(data));
                black_box(results);
            });
        });

        // GPU benchmark
        let input_refs: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();
        group.bench_with_input(BenchmarkId::new("GPU", batch_size), &input_refs, |b, data| {
            b.iter(|| {
                let result = pollster::block_on(bench_gpu_sha3(&gpu_hasher, black_box(data)));
                black_box(result);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_batch_sizes,
    benchmark_input_sizes,
    benchmark_single_vs_batch,
    benchmark_large_batch
);
criterion_main!(benches);
