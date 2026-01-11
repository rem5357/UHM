//! Build script for UHM
//!
//! Increments build number on each recompilation and embeds build metadata.

use std::fs;
use std::path::Path;

fn main() {
    // Only rerun when src/ files change (not on every cargo build)
    println!("cargo:rerun-if-changed=src");

    // Path to build number file
    let build_number_path = Path::new("build_number.txt");

    // Read current build number or start at 0
    let current_build: u64 = if build_number_path.exists() {
        fs::read_to_string(build_number_path)
            .unwrap_or_else(|_| "0".to_string())
            .trim()
            .parse()
            .unwrap_or(0)
    } else {
        0
    };

    // Increment build number
    let new_build = current_build + 1;

    // Write new build number back to file
    fs::write(build_number_path, new_build.to_string())
        .expect("Failed to write build number file");

    // Get current timestamp
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Set environment variables for compile-time embedding
    println!("cargo:rustc-env=UHM_BUILD_NUMBER={}", new_build);
    println!("cargo:rustc-env=UHM_BUILD_TIMESTAMP={}", timestamp);

    // Also output for build log visibility
    println!("cargo:warning=UHM Build #{} at {}", new_build, timestamp);
}
