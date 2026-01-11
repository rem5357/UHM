//! Universal Health Manager (UHM)
//!
//! An MCP server for health and nutrition tracking.

use std::path::PathBuf;
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing_subscriber::EnvFilter;

mod build_info;
mod db;
mod mcp;
mod models;
mod nutrition;
mod tools;

use mcp::UhmService;

/// Get the database path from environment or use default
fn get_database_path() -> PathBuf {
    std::env::var("UHM_DATABASE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));

            // Go up from target/release or target/debug to project root
            if path.ends_with("release") || path.ends_with("debug") {
                if let Some(parent) = path.parent() {
                    if let Some(grandparent) = parent.parent() {
                        path = grandparent.to_path_buf();
                    }
                }
            }

            path.push("data");
            path.push("uhm.db");
            path
        })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (output to stderr to not interfere with MCP stdio)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("uhm=info".parse()?))
        .with_writer(std::io::stderr)
        .init();

    // Print startup banner to stderr
    build_info::print_startup_banner();
    eprintln!("Starting MCP server on stdio...");

    // Get database path
    let db_path = get_database_path();
    eprintln!("Database path: {}", db_path.display());

    // Ensure data directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize database
    eprintln!("Initializing database...");
    let database = db::Database::new(&db_path)?;

    // Run migrations
    database.with_conn(|conn| {
        db::migrations::run_migrations(conn)?;
        let version = db::migrations::get_schema_version(conn)?;
        eprintln!("Database schema version: {}", version);
        Ok(())
    })?;

    // Create the UHM service
    let service = UhmService::new(db_path, database);

    // Create stdio transport
    let transport = (stdin(), stdout());

    // Start the MCP server
    let server = service.serve(transport).await?;

    // Wait for the server to complete
    server.waiting().await?;

    Ok(())
}
