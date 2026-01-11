//! Day model
//!
//! Represents a day with aggregated nutrition totals.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use super::Nutrition;

/// A day container for meal entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Day {
    pub id: i64,
    pub date: String,  // ISO date: "2025-01-09"
    pub cached_nutrition: Nutrition,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayCreate {
    pub date: String,
    pub notes: Option<String>,
}

/// Data for updating a day
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayUpdate {
    pub notes: Option<String>,
}

impl Day {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            date: row.get("date")?,
            cached_nutrition: Nutrition {
                calories: row.get("cached_calories")?,
                protein: row.get("cached_protein")?,
                carbs: row.get("cached_carbs")?,
                fat: row.get("cached_fat")?,
                fiber: row.get("cached_fiber")?,
                sodium: row.get("cached_sodium")?,
                sugar: row.get("cached_sugar")?,
                saturated_fat: row.get("cached_saturated_fat")?,
                cholesterol: row.get("cached_cholesterol")?,
            },
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Create a new day
    pub fn create(conn: &Connection, data: &DayCreate) -> DbResult<Self> {
        conn.execute(
            r#"
            INSERT INTO days (date, notes)
            VALUES (?1, ?2)
            "#,
            params![data.date, data.notes],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a day by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM days WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(day) => Ok(Some(day)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a day by date
    pub fn get_by_date(conn: &Connection, date: &str) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM days WHERE date = ?1")?;

        let result = stmt.query_row([date], Self::from_row);
        match result {
            Ok(day) => Ok(Some(day)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get or create a day by date
    pub fn get_or_create(conn: &Connection, date: &str) -> DbResult<Self> {
        if let Some(day) = Self::get_by_date(conn, date)? {
            return Ok(day);
        }

        Self::create(conn, &DayCreate {
            date: date.to_string(),
            notes: None,
        })
    }

    /// List days with optional date range
    pub fn list(
        conn: &Connection,
        start_date: Option<&str>,
        end_date: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<Self>> {
        let mut sql = String::from("SELECT * FROM days WHERE 1=1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(start) = start_date {
            params_vec.push(Box::new(start.to_string()));
            sql.push_str(&format!(" AND date >= ?{}", params_vec.len()));
        }

        if let Some(end) = end_date {
            params_vec.push(Box::new(end.to_string()));
            sql.push_str(&format!(" AND date <= ?{}", params_vec.len()));
        }

        sql.push_str(" ORDER BY date DESC");

        params_vec.push(Box::new(limit));
        sql.push_str(&format!(" LIMIT ?{}", params_vec.len()));

        params_vec.push(Box::new(offset));
        sql.push_str(&format!(" OFFSET ?{}", params_vec.len()));

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let days = stmt
            .query_map(params_refs.as_slice(), Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(days)
    }

    /// Count days with optional date range
    pub fn count(conn: &Connection, start_date: Option<&str>, end_date: Option<&str>) -> DbResult<i64> {
        let mut sql = String::from("SELECT COUNT(*) FROM days WHERE 1=1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(start) = start_date {
            params_vec.push(Box::new(start.to_string()));
            sql.push_str(&format!(" AND date >= ?{}", params_vec.len()));
        }

        if let Some(end) = end_date {
            params_vec.push(Box::new(end.to_string()));
            sql.push_str(&format!(" AND date <= ?{}", params_vec.len()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;
        Ok(count)
    }

    /// Update a day
    pub fn update(conn: &Connection, id: i64, data: &DayUpdate) -> DbResult<Option<Self>> {
        if let Some(ref notes) = data.notes {
            conn.execute(
                "UPDATE days SET notes = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![notes, id],
            )?;
        }

        Self::get_by_id(conn, id)
    }

    /// Update cached nutrition for a day
    pub fn update_cached_nutrition(conn: &Connection, id: i64, nutrition: &Nutrition) -> DbResult<()> {
        conn.execute(
            r#"
            UPDATE days SET
                cached_calories = ?1,
                cached_protein = ?2,
                cached_carbs = ?3,
                cached_fat = ?4,
                cached_fiber = ?5,
                cached_sodium = ?6,
                cached_sugar = ?7,
                cached_saturated_fat = ?8,
                cached_cholesterol = ?9,
                updated_at = datetime('now')
            WHERE id = ?10
            "#,
            params![
                nutrition.calories,
                nutrition.protein,
                nutrition.carbs,
                nutrition.fat,
                nutrition.fiber,
                nutrition.sodium,
                nutrition.sugar,
                nutrition.saturated_fat,
                nutrition.cholesterol,
                id,
            ],
        )?;
        Ok(())
    }

    /// Delete a day
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        let rows = conn.execute("DELETE FROM days WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }
}
