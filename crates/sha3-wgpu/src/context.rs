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
        let instance =
            Instance::new(InstanceDescriptor { backends: Backends::all(), ..Default::default() });

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
            .ok_or_else(|| GpuSha3Error::GpuError("Failed to find GPU adapter".to_string()))?;

        let adapter_info = adapter.get_info();

        // Get adapter's supported limits (these are the maximums the adapter supports)
        // Using adapter limits directly avoids requesting unsupported limits like maxInterStageShaderComponents
        // that might be in Default::default() but not supported by the browser's WebGPU implementation
        let limits = adapter.limits();

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
            .request_device(
                &DeviceDescriptor {
                    label: Some("SHA-3 GPU Device"),
                    required_features: features,
                    required_limits: limits,
                },
                None,
            )
            .await
            .map_err(|e| GpuSha3Error::GpuError(format!("Failed to create device: {e}")))?;

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
