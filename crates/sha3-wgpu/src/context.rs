//! WebGPU context management

use crate::error::GpuSha3Error;
use wgpu::*;

/// WebGPU context for SHA-3 computation
pub struct GpuContext {
    device: Device,
    queue: Queue,
    adapter_info: AdapterInfo,
}

impl GpuContext {
    /// Create a new GPU context with default settings
    pub async fn new() -> Result<Self, GpuSha3Error> {
        Self::new_with_features(None).await
    }

    /// Create a new GPU context with specific feature requirements
    pub async fn new_with_features(
        required_features: Option<Features>,
    ) -> Result<Self, GpuSha3Error> {
        // Create wgpu instance
        let instance_descriptor =
            InstanceDescriptor { backends: Backends::all(), ..Default::default() };
        let instance = Instance::new(&instance_descriptor);

        // Allow fallback adapter in CI environments (e.g., GitHub Actions without GPU)
        let force_fallback = std::env::var("WGPU_FORCE_FALLBACK_ADAPTER")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        // Request adapter (GPU or fallback)
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: force_fallback,
                compatible_surface: None,
            })
            .await
            .map_err(|e| {
                GpuSha3Error::AdapterNotFound(format!("Failed to find GPU adapter: {e}"))
            })?;

        let adapter_info = adapter.get_info();

        // Start with downlevel defaults which should be browser-compatible
        // Then override only the specific limits we need from the adapter
        // This avoids including max_inter_stage_shader_components which browsers don't recognize
        let adapter_limits = adapter.limits();
        let mut limits = Limits::downlevel_defaults();

        // Override with adapter limits for fields we actually use, clamped to reasonable maximums
        limits.max_buffer_size = adapter_limits.max_buffer_size.min(1 << 30); // Up to 1GB
        limits.max_storage_buffer_binding_size =
            adapter_limits.max_storage_buffer_binding_size.min(1 << 30);
        limits.max_compute_workgroup_storage_size =
            adapter_limits.max_compute_workgroup_storage_size.min(16384);
        limits.max_compute_invocations_per_workgroup =
            adapter_limits.max_compute_invocations_per_workgroup.min(256);
        limits.max_compute_workgroup_size_x = adapter_limits.max_compute_workgroup_size_x.min(256);
        limits.max_compute_workgroup_size_y = adapter_limits.max_compute_workgroup_size_y;
        limits.max_compute_workgroup_size_z = adapter_limits.max_compute_workgroup_size_z;
        limits.max_compute_workgroups_per_dimension =
            adapter_limits.max_compute_workgroups_per_dimension;
        limits.max_bind_groups = adapter_limits.max_bind_groups;
        limits.max_storage_buffers_per_shader_stage =
            adapter_limits.max_storage_buffers_per_shader_stage;
        limits.max_uniform_buffers_per_shader_stage =
            adapter_limits.max_uniform_buffers_per_shader_stage;
        limits.max_uniform_buffer_binding_size = adapter_limits.max_uniform_buffer_binding_size;

        // Check what features the adapter supports
        let adapter_features = adapter.features();
        let desired_features = required_features.unwrap_or({
            // Request features needed for SHA-3 compute shader
            // SHADER_INT64 is required for u64 operations in the shader
            Features::SHADER_INT64
        });

        // Only request features that the adapter actually supports
        // This is important for fallback adapters which may not support all features
        let features = desired_features & adapter_features;

        // Request device and queue
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("SHA-3 GPU Device"),
                required_features: features,
                required_limits: limits,
                experimental_features: ExperimentalFeatures::disabled(),
                memory_hints: Default::default(),
                trace: Trace::Off,
            })
            .await
            .map_err(|e| GpuSha3Error::DeviceCreation(format!("Failed to create device: {e}")))?;

        Ok(Self { device, queue, adapter_info })
    }

    /// Get reference to the device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get reference to the queue
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Get adapter information
    pub fn adapter_info(&self) -> &AdapterInfo {
        &self.adapter_info
    }

    /// Get device limits
    pub fn limits(&self) -> Limits {
        self.device.limits()
    }
}

impl std::fmt::Debug for GpuContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuContext")
            .field("adapter", &self.adapter_info.name)
            .field("backend", &self.adapter_info.backend)
            .field("device_type", &self.adapter_info.device_type)
            .finish()
    }
}
