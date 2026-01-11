//! UHM Status Tool
//!
//! Provides runtime status information about the UHM service.

use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::build_info::BuildInfo;

/// Runtime status of the UHM service
#[derive(Debug, Clone, Serialize)]
pub struct UhmStatus {
    /// Build information
    pub build_number: u64,
    pub build_timestamp: &'static str,
    pub version: &'static str,

    /// Database information
    pub database_path: String,
    pub database_size_bytes: Option<u64>,

    /// Process information
    pub uptime_seconds: u64,
    pub process_id: u32,
    pub memory_usage_bytes: u64,
}

/// Status tracker for collecting runtime information
pub struct StatusTracker {
    start_time: Instant,
    database_path: PathBuf,
}

impl StatusTracker {
    /// Create a new status tracker
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            start_time: Instant::now(),
            database_path,
        }
    }

    /// Get the current status
    pub fn get_status(&self) -> UhmStatus {
        let build_info = BuildInfo::current();

        // Get database size if it exists
        let database_size_bytes = std::fs::metadata(&self.database_path)
            .ok()
            .map(|m| m.len());

        // Get process info
        let pid = std::process::id();
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));

        let memory_usage_bytes = sys
            .process(Pid::from_u32(pid))
            .map(|p| p.memory())
            .unwrap_or(0);

        UhmStatus {
            build_number: build_info.build_number,
            build_timestamp: build_info.build_timestamp,
            version: build_info.version,
            database_path: self.database_path.display().to_string(),
            database_size_bytes,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            process_id: pid,
            memory_usage_bytes,
        }
    }
}
