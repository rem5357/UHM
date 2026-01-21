//! Patient model
//!
//! Stores patient information for report headers.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;

/// Patient information for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientInfo {
    pub id: i64,
    pub name: String,
    pub dob: String,
    pub created_at: String,
    pub updated_at: String,
}

impl PatientInfo {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            dob: row.get("dob")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Get patient info (single row table)
    pub fn get(conn: &Connection) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM patient_info WHERE id = 1")?;

        let result = stmt.query_row([], Self::from_row);
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set or update patient info (upsert)
    pub fn set(conn: &Connection, name: &str, dob: &str) -> DbResult<Self> {
        conn.execute(
            r#"
            INSERT INTO patient_info (id, name, dob)
            VALUES (1, ?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                dob = excluded.dob,
                updated_at = datetime('now')
            "#,
            params![name, dob],
        )?;

        Self::get(conn)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }
}
