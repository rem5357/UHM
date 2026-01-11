//! Database module
//!
//! Handles SQLite connection and migrations.

pub mod connection;
pub mod migrations;

pub use connection::{Database, DbError, DbResult};
