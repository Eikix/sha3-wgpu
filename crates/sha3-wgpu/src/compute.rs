//! GPU compute pipeline for SHA-3 batch hashing

use futures::channel::oneshot;
use sha3::digest::{Digest, ExtendableOutput, Update, XofReader};
use sha3_core::{BatchHashParams, Sha3Variant};
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::{context::GpuContext, error::GpuSha3Error};

/// Configuration for persistent buffer allocation
/// (max_batch_size, max_input_length, max_output_bytes)
type PersistentBufferConfig = (usize, usize, usize);

/// Parameters for persistent buffer hashing operation
#[derive(Debug)]
struct PersistentHashParams<'a> {
    inputs: &'a [&'a [u8]],
    params: &'a BatchHashParams,
    output_bytes: usize,
    total_output_bytes: usize,
}

// Include the WGSL shader at compile time
const SHADER_SOURCE: &str = include_str!("wgsl/sha3.wgsl");

/// Maximum input size per hash in bytes (must match MAX_INPUT_SIZE in WGSL shader)
const MAX_INPUT_SIZE: usize = 8192;

fn cpu_hash_batch(inputs: &[&[u8]], params: &BatchHashParams) -> Result<Vec<u8>, GpuSha3Error> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    if inputs.len() != params.num_hashes {
        return Err(GpuSha3Error::InvalidInputLength(params.num_hashes));
    }

    if !inputs.iter().all(|input| input.len() == params.input_length) {
        return Err(GpuSha3Error::InvalidInputLength(params.input_length));
    }

    let output_bytes = params.get_output_bytes().map_err(GpuSha3Error::Core)?;
    let mut output = Vec::with_capacity(inputs.len() * output_bytes);

    match params.variant {
        Sha3Variant::Sha3_224 => {
            for input in inputs {
                let mut hasher = sha3::Sha3_224::default();
                Update::update(&mut hasher, input);
                let digest = Digest::finalize(hasher);
                output.extend_from_slice(digest.as_ref());
            }
        }
        Sha3Variant::Sha3_256 => {
            for input in inputs {
                let mut hasher = sha3::Sha3_256::default();
                Update::update(&mut hasher, input);
                let digest = Digest::finalize(hasher);
                output.extend_from_slice(digest.as_ref());
            }
        }
        Sha3Variant::Sha3_384 => {
            for input in inputs {
                let mut hasher = sha3::Sha3_384::default();
                Update::update(&mut hasher, input);
                let digest = Digest::finalize(hasher);
                output.extend_from_slice(digest.as_ref());
            }
        }
        Sha3Variant::Sha3_512 => {
            for input in inputs {
                let mut hasher = sha3::Sha3_512::default();
                Update::update(&mut hasher, input);
                let digest = Digest::finalize(hasher);
                output.extend_from_slice(digest.as_ref());
            }
        }
        Sha3Variant::Shake128 => {
            for input in inputs {
                let mut hasher = sha3::Shake128::default();
                Update::update(&mut hasher, input);
                let mut reader = ExtendableOutput::finalize_xof(hasher);
                let mut buf = vec![0u8; output_bytes];
                reader.read(&mut buf);
                output.extend_from_slice(&buf);
            }
        }
        Sha3Variant::Shake256 => {
            for input in inputs {
                let mut hasher = sha3::Shake256::default();
                Update::update(&mut hasher, input);
                let mut reader = ExtendableOutput::finalize_xof(hasher);
                let mut buf = vec![0u8; output_bytes];
                reader.read(&mut buf);
                output.extend_from_slice(&buf);
            }
        }
    }

    Ok(output)
}

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

/// Persistent GPU buffers for optimized performance
/// Reuses buffers across multiple hash operations to eliminate allocation overhead
struct PersistentBuffers {
    /// Input buffer (storage, read-only) - packed u32 format for efficiency
    input_buffer: Buffer,
    /// Output buffer (storage, read-write)
    output_buffer: Buffer,
    /// Staging buffer for efficient CPU readback
    staging_buffer: Buffer,
    /// Uniform buffer for parameters
    uniform_buffer: Buffer,
    /// Bind group containing all buffers
    bind_group: BindGroup,
    /// Maximum batch size this buffer set can handle
    max_batch_size: usize,
    /// Maximum input length per hash this buffer set can handle
    max_input_length: usize,
    /// Maximum output bytes per hash this buffer set can handle
    max_output_bytes: usize,
}

impl PersistentBuffers {
    /// Create persistent buffers for the given maximum batch parameters
    fn new(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        max_batch_size: usize,
        max_input_length: usize,
        max_output_bytes: usize,
    ) -> Result<Self, GpuSha3Error> {
        let total_input_bytes = max_batch_size * max_input_length;
        let total_output_bytes = max_batch_size * max_output_bytes;

        // Align buffer sizes to 16 bytes (WGSL struct alignment requirement)
        let input_buffer_size = ((total_input_bytes + 15) / 16) * 16;
        let output_buffer_size = ((total_output_bytes + 15) / 16) * 16;

        // Create input buffer (storage, read-only)
        let input_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Persistent Input Buffer"),
            size: input_buffer_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create output buffer (storage, read-write)
        let output_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Persistent Output Buffer"),
            size: output_buffer_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create staging buffer for CPU readback (persistent for performance)
        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Persistent Staging Buffer"),
            size: output_buffer_size as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SHA-3 Persistent Uniform Buffer"),
            size: std::mem::size_of::<GpuHashParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("SHA-3 Persistent Bind Group"),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });

        Ok(Self {
            input_buffer,
            output_buffer,
            staging_buffer,
            uniform_buffer,
            bind_group,
            max_batch_size,
            max_input_length,
            max_output_bytes,
        })
    }

    /// Check if this buffer set can handle the given batch parameters
    fn can_handle_batch(
        &self,
        num_hashes: usize,
        input_length: usize,
        output_bytes: usize,
    ) -> bool {
        num_hashes <= self.max_batch_size
            && input_length <= self.max_input_length
            && output_bytes <= self.max_output_bytes
    }
}

/// GPU-accelerated SHA-3 batch hasher
pub struct GpuSha3Hasher {
    context: GpuContext,
    variant: Sha3Variant,
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
    /// Persistent buffers for performance optimization (optional)
    buffers: Option<PersistentBuffers>,
    /// Maximum batch size for persistent buffers
    max_batch_size: usize,
}

impl GpuSha3Hasher {
    /// Create a new GPU SHA-3 hasher for a specific variant
    /// Uses persistent buffers by default for optimal performance
    pub fn new(context: GpuContext, variant: Sha3Variant) -> Result<Self, GpuSha3Error> {
        // Enable persistent buffers by default for performance
        // Conservative defaults: 1000 hashes, 8KB input, 64 bytes output
        let max_batch_size = 1000;
        let max_input_length = 8192; // 8KB per input (matches shader limit)
        let max_output_bytes = 64; // Maximum output size (covers SHA3-512 and reasonable SHAKE outputs)
        Self::with_persistent_buffers(
            context,
            variant,
            Some((max_batch_size, max_input_length, max_output_bytes)),
        )
    }

    /// Create a new GPU SHA-3 hasher with optional persistent buffers for performance
    pub fn with_persistent_buffers(
        context: GpuContext,
        variant: Sha3Variant,
        max_batch_config: Option<PersistentBufferConfig>,
    ) -> Result<Self, GpuSha3Error> {
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

        // Initialize persistent buffers if requested
        let buffers =
            if let Some((max_batch_size, max_input_length, max_output_bytes)) = max_batch_config {
                Some(PersistentBuffers::new(
                    device,
                    &bind_group_layout,
                    max_batch_size,
                    max_input_length,
                    max_output_bytes,
                )?)
            } else {
                None
            };

        // Set default max_batch_size based on persistent buffers or fallback
        let max_batch_size = buffers.as_ref().map(|b| b.max_batch_size).unwrap_or(1000);

        Ok(Self { context, variant, pipeline, bind_group_layout, buffers, max_batch_size })
    }

    /// Hash a batch of inputs (all must be the same length)
    /// Returns a flattened vector of all output hashes
    pub async fn hash_batch(&self, inputs: &[&[u8]]) -> Result<Vec<u8>, GpuSha3Error> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all inputs are the same length
        let input_length = inputs[0].len();
        if input_length > MAX_INPUT_SIZE {
            return Err(GpuSha3Error::InvalidInputLength(input_length));
        }
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
            return cpu_hash_batch(inputs, params);
        }

        let output_bytes = params.get_output_bytes().map_err(GpuSha3Error::Core)?;
        let total_output_bytes = params.num_hashes * output_bytes;

        // Try persistent buffers first, fall back to dynamic allocation
        if self.can_use_persistent_buffers(params.num_hashes, params.input_length, output_bytes) {
            let buffers = self.buffers.as_ref().unwrap();
            let hash_params =
                PersistentHashParams { inputs, params, output_bytes, total_output_bytes };
            self.hash_batch_with_persistent_buffers(buffers, hash_params).await
        } else {
            // Fallback to dynamic buffer allocation
            self.hash_batch_with_dynamic_buffers(inputs, params, output_bytes, total_output_bytes)
                .await
        }
    }

    /// Check if persistent buffers can handle a batch
    fn can_use_persistent_buffers(
        &self,
        num_hashes: usize,
        input_length: usize,
        output_bytes: usize,
    ) -> bool {
        self.buffers
            .as_ref()
            .map(|buffers| buffers.can_handle_batch(num_hashes, input_length, output_bytes))
            .unwrap_or(false)
    }

    /// Optimized path using persistent buffers
    async fn hash_batch_with_persistent_buffers(
        &self,
        buffers: &PersistentBuffers,
        hash_params: PersistentHashParams<'_>,
    ) -> Result<Vec<u8>, GpuSha3Error> {
        let device = self.context.device();
        let queue = self.context.queue();

        // Prepare GPU parameters
        let gpu_params = GpuHashParams {
            num_hashes: hash_params.params.num_hashes as u32,
            input_length: hash_params.params.input_length as u32,
            rate_bytes: hash_params.params.variant.rate_bytes() as u32,
            output_bytes: hash_params.output_bytes as u32,
        };

        // Calculate actual buffer sizes needed for this batch
        let total_input_bytes = hash_params.params.num_hashes * hash_params.params.input_length;
        let input_buffer_size = ((total_input_bytes + 15) / 16) * 16; // Align to 16 bytes
        let output_buffer_size = ((hash_params.total_output_bytes + 15) / 16) * 16; // Align to 16 bytes

        // Flatten and copy input data (reuse persistent buffers)
        let mut input_data = Vec::with_capacity(input_buffer_size);
        for input in hash_params.inputs.iter() {
            input_data.extend_from_slice(input);
        }
        // Pad to required buffer size
        input_data.resize(input_buffer_size, 0);
        queue.write_buffer(&buffers.input_buffer, 0, &input_data);

        // Update uniform buffer with parameters
        queue.write_buffer(&buffers.uniform_buffer, 0, bytemuck::cast_slice(&[gpu_params]));

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
            compute_pass.set_bind_group(0, &buffers.bind_group, &[]);

            // Dispatch workgroups (one thread per hash, 256 threads per workgroup)
            // Optimized: Increased from 128 to 256 for maximum GPU occupancy
            let workgroup_size = 256;
            let num_workgroups =
                (hash_params.params.num_hashes + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        // Copy output to staging buffer
        let current_staging = &buffers.staging_buffer;
        encoder.copy_buffer_to_buffer(
            &buffers.output_buffer,
            0,
            current_staging,
            0,
            output_buffer_size as u64,
        );

        // Submit commands
        queue.submit(Some(encoder.finish()));

        // Read results from current staging buffer
        let buffer_slice = current_staging.slice(..);
        let (sender, receiver) = oneshot::channel();

        buffer_slice.map_async(MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // Ensure the mapping callback is processed on native targets
        #[allow(unused_must_use)]
        {
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        }

        // Wait for the mapping callback to fire
        receiver
            .await
            .map_err(|_| {
                GpuSha3Error::BufferMapping("Failed to receive buffer mapping result".into())
            })?
            .map_err(|e| GpuSha3Error::BufferMapping(format!("Buffer mapping failed: {e:?}")))?;

        // Extract output data
        let data = buffer_slice.get_mapped_range();
        let mut result = vec![0u8; hash_params.total_output_bytes];
        result.copy_from_slice(&data[..hash_params.total_output_bytes]);

        drop(data);
        current_staging.unmap();

        Ok(result)
    }

    /// Fallback path for very large batches that exceed persistent buffer capacity
    async fn hash_batch_with_dynamic_buffers(
        &self,
        inputs: &[&[u8]],
        params: &BatchHashParams,
        output_bytes: usize,
        total_output_bytes: usize,
    ) -> Result<Vec<u8>, GpuSha3Error> {
        let device = self.context.device();
        let queue = self.context.queue();

        // Prepare GPU parameters
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

            // Dispatch workgroups (one thread per hash, 256 threads per workgroup)
            let workgroup_size = 256;
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
        queue.submit(Some(encoder.finish()));

        // Read results from staging buffer
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = oneshot::channel();

        buffer_slice.map_async(MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // Ensure the mapping callback is processed on native targets
        #[allow(unused_must_use)]
        {
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        }

        // Wait for the mapping callback to fire
        receiver
            .await
            .map_err(|_| {
                GpuSha3Error::BufferMapping("Failed to receive buffer mapping result".into())
            })?
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
            .field("max_batch_size", &self.max_batch_size)
            .field("has_persistent_buffers", &self.buffers.is_some())
            .finish()
    }
}
