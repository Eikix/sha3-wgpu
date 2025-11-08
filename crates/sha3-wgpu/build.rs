//! Build script to include WGSL shaders

use std::path::PathBuf;

fn main() {
    // Tell Cargo to rerun this build script if shader files change
    println!("cargo:rerun-if-changed=src/wgsl");

    // Verify WGSL shader directory exists
    let shader_dir = PathBuf::from("src/wgsl");
    if shader_dir.exists() {
        println!("cargo:warning=WGSL shaders directory found");
    }
}
