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
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // Request adapter (GPU)
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| GpuSha3Error::GpuError("Failed to find GPU adapter".to_string()))?;

        let adapter_info = adapter.get_info();

        // Get adapter limits and features
        let mut limits = Limits::default();

        // Increase buffer size limits for batch processing
        limits.max_buffer_size = 1 << 30; // 1GB max buffer
        limits.max_storage_buffer_binding_size = 1 << 30;
        limits.max_compute_workgroup_storage_size = 16384;
        limits.max_compute_invocations_per_workgroup = 256;
        limits.max_compute_workgroup_size_x = 256;

        let features = required_features.unwrap_or_else(|| {
            // Request features needed for SHA-3 compute shader
            Features::empty()
        });

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("SHA-3 GPU Device"),
                    required_features: features,
                    required_limits: limits,
                },
                None,
            )
            .await
            .map_err(|e| GpuSha3Error::GpuError(format!("Failed to create device: {}", e)))?;

        Ok(Self {
            device,
            queue,
            adapter_info,
        })
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
