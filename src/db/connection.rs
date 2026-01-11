//! Database connection management
//!
//! Provides SQLite connection pooling and management.

use std::path::Path;
use std::sync::Arc;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;
use thiserror::Error;

/// Database error types
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database connection error: {0}")]
    Connection(#[from] r2d2::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Database not initialized")]
    NotInitialized,
}

/// Result type for database operations
pub type DbResult<T> = Result<T, DbError>;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct Database {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl Database {
    /// Create a new database connection pool
    pub fn new<P: AsRef<Path>>(path: P) -> DbResult<Self> {
        let manager = SqliteConnectionManager::file(path)
            .with_flags(
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_URI,
            )
            .with_init(|conn| {
                // Enable foreign keys
                conn.execute_batch(
                    "PRAGMA foreign_keys = ON;
                     PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA cache_size = -64000;
                     PRAGMA temp_store = MEMORY;",
                )?;
                Ok(())
            });

        let pool = Pool::builder()
            .max_size(10)
            .build(manager)?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get a connection from the pool
    pub fn get_conn(&self) -> DbResult<PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }

    /// Execute a closure with a database connection
    pub fn with_conn<F, T>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&rusqlite::Connection) -> DbResult<T>,
    {
        let conn = self.get_conn()?;
        f(&conn)
    }

    /// Execute a closure with a mutable database connection (for transactions)
    pub fn with_conn_mut<F, T>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&mut rusqlite::Connection) -> DbResult<T>,
    {
        let mut conn = self.get_conn()?;
        f(&mut conn)
    }
}
