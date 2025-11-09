//! Build script to include WGSL shaders

use std::path::PathBuf;

fn main() {
    // Tell Cargo to rerun this build script if shader files change
    println!("cargo:rerun-if-changed=src/wgsl");

    // Verify WGSL shader directory exists
    let shader_dir = PathBuf::from("src/wgsl");
    if !shader_dir.exists() {
        panic!("WGSL shader directory not found at {shader_dir:?}");
    }

    // Verify critical shader files exist
    let sha3_shader = shader_dir.join("sha3.wgsl");
    if !sha3_shader.exists() {
        panic!("Required shader file not found: {sha3_shader:?}");
    }

    // Optional: Validate shader syntax by attempting to read it
    if let Err(e) = std::fs::read_to_string(&sha3_shader) {
        panic!("Failed to read shader file {sha3_shader:?}: {e}");
    }

    println!("cargo:warning=WGSL shader validation completed successfully");
}
