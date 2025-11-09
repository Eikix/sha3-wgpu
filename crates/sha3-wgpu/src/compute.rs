//! GPU compute pipeline for SHA-3 batch hashing

use futures::channel::oneshot;
use sha3_core::{BatchHashParams, Sha3Variant};
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::{context::GpuContext, error::GpuSha3Error};

// Include the WGSL shader at compile time
const SHADER_SOURCE: &str = include_str!("wgsl/sha3.wgsl");

/// Maximum input size per hash in bytes (must match MAX_INPUT_SIZE in WGSL shader)
const MAX_INPUT_SIZE: usize = 8192;

/// GPU parameters structure matching WGSL uniform
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct GpuHashParams {
    num_hashes: u32,
    input_length: u32,
    rate_bytes: u32,
    output_bytes: u32,
}

// SAFETY: GpuHashParams is repr(C) with only u32 fields, which are Pod and Zeroable.
// The struct has no padding, references, or other unsafe fields.
unsafe impl bytemuck::Pod for GpuHashParams {}
unsafe impl bytemuck::Zeroable for GpuHashParams {}

/// GPU-accelerated SHA-3 batch hasher
pub struct GpuSha3Hasher {
    context: GpuContext,
    variant: Sha3Variant,
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl GpuSha3Hasher {
    /// Create a new GPU SHA-3 hasher for a specific variant
    pub fn new(context: GpuContext, variant: Sha3Variant) -> Result<Self, GpuSha3Error> {
        let device = context.device();

        // Create shader module
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("SHA-3 Compute Shader"),
            source: ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SHA-3 Bind Group Layout"),
            entries: &[
                // Input buffer (storage, read-only)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output buffer (storage, read-write)
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters (uniform)
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("SHA-3 Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("SHA-3 Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self { context, variant, pipeline, bind_group_layout })
    }

    /// Hash a batch of inputs (all must be the same length)
    /// Returns a flattened vector of all output hashes
    pub async fn hash_batch(&self, inputs: &[&[u8]]) -> Result<Vec<u8>, GpuSha3Error> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all inputs are the same length
        let input_length = inputs[0].len();
        if !inputs.iter().all(|input| input.len() == input_length) {
            return Err(GpuSha3Error::InvalidInputLength(input_length));
        }

        let params = BatchHashParams::new(self.variant, inputs.len(), input_length);
        self.hash_batch_with_params(inputs, &params).await
    }

    /// Hash a batch with custom parameters (for SHAKE variants with custom output length)
    pub async fn hash_batch_with_params(
        &self,
        inputs: &[&[u8]],
        params: &BatchHashParams,
    ) -> Result<Vec<u8>, GpuSha3Error> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        // Validate input size doesn't exceed GPU shader limits
        if params.input_length > MAX_INPUT_SIZE {
            return Err(GpuSha3Error::InvalidInputLength(params.input_length));
        }

        let device = self.context.device();
        let queue = self.context.queue();

        // Prepare GPU parameters
        let output_bytes = params.get_output_bytes().map_err(GpuSha3Error::Core)?;
        let gpu_params = GpuHashParams {
            num_hashes: params.num_hashes as u32,
            input_length: params.input_length as u32,
            rate_bytes: params.variant.rate_bytes() as u32,
            output_bytes: output_bytes as u32,
        };

        // Calculate buffer sizes (pad to 16-byte alignment to match WGSL struct alignment)
        let total_input_bytes = params.num_hashes * params.input_length;
        let input_buffer_size = if total_input_bytes == 0 {
            16 // Minimum size for empty input (16-byte alignment)
        } else {
            ((total_input_bytes + 15) / 16) * 16 // Align to 16 bytes
        };

        let total_output_bytes = params.num_hashes * output_bytes;
        let output_buffer_size = if total_output_bytes == 0 {
            16 // Minimum size for empty output (16-byte alignment)
        } else {
            ((total_output_bytes + 15) / 16) * 16 // Align to 16 bytes
        };

        // Create input buffer and copy data
        let input_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Input Buffer"),
            size: input_buffer_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Flatten and copy input data (optimized allocation)
        let mut input_data = Vec::with_capacity(input_buffer_size);
        for input in inputs.iter() {
            input_data.extend_from_slice(input);
        }
        // Pad to required buffer size
        input_data.resize(input_buffer_size, 0);
        queue.write_buffer(&input_buffer, 0, &input_data);

        // Create output buffer
        let output_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Output Buffer"),
            size: output_buffer_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create staging buffer for reading results
        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Staging Buffer"),
            size: output_buffer_size as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform buffer for parameters
        let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("SHA-3 Uniform Buffer"),
            contents: bytemuck::cast_slice(&[gpu_params]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("SHA-3 Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        // Create command encoder and dispatch compute shader
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("SHA-3 Command Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("SHA-3 Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (one thread per hash, 64 threads per workgroup)
            let workgroup_size = 64;
            let num_workgroups = (params.num_hashes + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        // Copy output to staging buffer
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            output_buffer_size as u64,
        );

        // Submit commands
        // The copy_buffer_to_buffer operation ensures compute finishes before copying
        queue.submit(Some(encoder.finish()));

        // Read results from staging buffer
        // Buffer mapping will wait for the copy operation to complete
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = oneshot::channel();

        buffer_slice.map_async(MapMode::Read, move |result| {
            // If send fails, the receiver was dropped, which will be caught when we await it
            let _ = sender.send(result);
        });

        // Ensure the mapping callback is processed on native targets.
        // On native, wgpu requires explicit polling for asynchronous operations.
        #[allow(unused_must_use)]
        {
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        }

        // In WASM, wasm-bindgen-futures will poll the device automatically
        // Wait for the mapping callback to fire
        receiver
            .await
            .map_err(|_| GpuSha3Error::BufferMapping("Failed to receive buffer mapping result".into()))?
            .map_err(|e| GpuSha3Error::BufferMapping(format!("Buffer mapping failed: {e:?}")))?;

        // Extract output data
        let data = buffer_slice.get_mapped_range();
        let mut result = vec![0u8; total_output_bytes];
        result.copy_from_slice(&data[..total_output_bytes]);

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    /// Get the SHA-3 variant this hasher uses
    pub fn variant(&self) -> Sha3Variant {
        self.variant
    }

    /// Get reference to the GPU context
    pub fn context(&self) -> &GpuContext {
        &self.context
    }
}

impl std::fmt::Debug for GpuSha3Hasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuSha3Hasher")
            .field("variant", &self.variant)
            .field("context", &self.context)
            .finish()
    }
}
