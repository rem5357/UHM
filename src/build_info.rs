//! Build information module
//!
//! Contains compile-time constants for build number and timestamp.

use serde::Serialize;

/// Build number, incremented on each recompilation
pub const BUILD_NUMBER: u64 = match option_env!("UHM_BUILD_NUMBER") {
    Some(s) => match parse_u64(s) {
        Some(n) => n,
        None => 0,
    },
    None => 0,
};

/// Build timestamp in ISO 8601 format
pub const BUILD_TIMESTAMP: &str = match option_env!("UHM_BUILD_TIMESTAMP") {
    Some(s) => s,
    None => "unknown",
};

/// Package version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Package name from Cargo.toml
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Package description from Cargo.toml
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Const function to parse u64 at compile time
const fn parse_u64(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    let mut result: u64 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < b'0' || b > b'9' {
            return None;
        }
        result = result * 10 + (b - b'0') as u64;
        i += 1;
    }
    Some(result)
}

/// Build information structure for serialization
#[derive(Debug, Clone, Serialize)]
pub struct BuildInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub build_number: u64,
    pub build_timestamp: &'static str,
    pub description: &'static str,
}

impl BuildInfo {
    /// Get the current build info
    pub fn current() -> Self {
        Self {
            name: NAME,
            version: VERSION,
            build_number: BUILD_NUMBER,
            build_timestamp: BUILD_TIMESTAMP,
            description: DESCRIPTION,
        }
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self::current()
    }
}

/// Print the startup banner to stderr
pub fn print_startup_banner() {
    let info = BuildInfo::current();
    eprintln!("===============================================");
    eprintln!("  Universal Health Manager (UHM)");
    eprintln!("  Version: {} | Build: {}", info.version, info.build_number);
    eprintln!("  Compiled: {}", info.build_timestamp);
    eprintln!("===============================================");
}
