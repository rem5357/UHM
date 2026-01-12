//! Vital model
//!
//! Represents vital signs and health measurements including weight, blood pressure,
//! heart rate, oxygen saturation, and glucose levels. Supports grouping related readings.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;

/// Vital type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VitalType {
    Weight,
    BloodPressure,
    HeartRate,
    OxygenSaturation,
    Glucose,
}

impl VitalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VitalType::Weight => "weight",
            VitalType::BloodPressure => "blood_pressure",
            VitalType::HeartRate => "heart_rate",
            VitalType::OxygenSaturation => "oxygen_saturation",
            VitalType::Glucose => "glucose",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "weight" => Some(VitalType::Weight),
            "blood_pressure" | "bp" => Some(VitalType::BloodPressure),
            "heart_rate" | "hr" | "pulse" => Some(VitalType::HeartRate),
            "oxygen_saturation" | "o2" | "spo2" | "oxygen" => Some(VitalType::OxygenSaturation),
            "glucose" | "blood_sugar" | "sugar" => Some(VitalType::Glucose),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            VitalType::Weight => "Weight",
            VitalType::BloodPressure => "Blood Pressure",
            VitalType::HeartRate => "Heart Rate",
            VitalType::OxygenSaturation => "Oxygen Saturation",
            VitalType::Glucose => "Blood Glucose",
        }
    }

    /// Default unit for this vital type
    pub fn default_unit(&self) -> &'static str {
        match self {
            VitalType::Weight => "lbs",
            VitalType::BloodPressure => "mmHg",
            VitalType::HeartRate => "bpm",
            VitalType::OxygenSaturation => "%",
            VitalType::Glucose => "mg/dL",
        }
    }

    /// Whether this vital type uses value2 (e.g., blood pressure has systolic/diastolic)
    pub fn uses_value2(&self) -> bool {
        matches!(self, VitalType::BloodPressure)
    }

    /// Value labels for this vital type
    pub fn value_labels(&self) -> (&'static str, Option<&'static str>) {
        match self {
            VitalType::Weight => ("Weight", None),
            VitalType::BloodPressure => ("Systolic", Some("Diastolic")),
            VitalType::HeartRate => ("BPM", None),
            VitalType::OxygenSaturation => ("SpO2 %", None),
            VitalType::Glucose => ("mg/dL", None),
        }
    }
}

/// A group of related vital readings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalGroup {
    pub id: i64,
    pub description: Option<String>,
    pub timestamp: String,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Data for creating a new vital group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalGroupCreate {
    pub description: Option<String>,
    pub timestamp: Option<String>,
    pub notes: Option<String>,
}

impl VitalGroup {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            description: row.get("description")?,
            timestamp: row.get("timestamp")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
        })
    }

    /// Create a new vital group
    pub fn create(conn: &Connection, data: &VitalGroupCreate) -> DbResult<Self> {
        let timestamp = data.timestamp.clone().unwrap_or_else(|| {
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
        });

        conn.execute(
            r#"
            INSERT INTO vital_groups (description, timestamp, notes)
            VALUES (?1, ?2, ?3)
            "#,
            params![data.description, timestamp, data.notes],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a vital group by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM vital_groups WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(group) => Ok(Some(group)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all vital groups, ordered by timestamp descending
    pub fn list(conn: &Connection, limit: Option<i64>) -> DbResult<Vec<Self>> {
        let sql = match limit {
            Some(n) => format!(
                "SELECT * FROM vital_groups ORDER BY timestamp DESC LIMIT {}",
                n
            ),
            None => "SELECT * FROM vital_groups ORDER BY timestamp DESC".to_string(),
        };

        let mut stmt = conn.prepare(&sql)?;
        let groups = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(groups)
    }

    /// Update a vital group
    pub fn update(
        conn: &Connection,
        id: i64,
        description: Option<String>,
        notes: Option<String>,
    ) -> DbResult<Option<Self>> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref desc) = description {
            updates.push(format!("description = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(desc.clone()));
        }
        if let Some(ref n) = notes {
            updates.push(format!("notes = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(n.clone()));
        }

        if updates.is_empty() {
            return Self::get_by_id(conn, id);
        }

        let sql = format!(
            "UPDATE vital_groups SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Delete a vital group (unlinks vitals but doesn't delete them)
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Unlink any vitals from this group
        conn.execute("UPDATE vitals SET group_id = NULL WHERE group_id = ?1", [id])?;

        // Delete the group
        let rows = conn.execute("DELETE FROM vital_groups WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Get all vitals in this group
    pub fn get_vitals(conn: &Connection, group_id: i64) -> DbResult<Vec<Vital>> {
        Vital::list_by_group(conn, group_id)
    }
}

/// A vital sign reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vital {
    pub id: i64,
    pub vital_type: VitalType,
    pub timestamp: String,
    pub value1: f64,
    pub value2: Option<f64>,
    pub unit: String,
    pub group_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new vital
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalCreate {
    pub vital_type: VitalType,
    pub timestamp: Option<String>,
    pub value1: f64,
    pub value2: Option<f64>,
    pub unit: Option<String>,
    pub group_id: Option<i64>,
    pub notes: Option<String>,
}

/// Data for updating a vital
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VitalUpdate {
    pub value1: Option<f64>,
    pub value2: Option<f64>,
    pub unit: Option<String>,
    pub group_id: Option<i64>,
    pub notes: Option<String>,
}

impl Vital {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let vital_type_str: String = row.get("vital_type")?;
        let vital_type = VitalType::from_str(&vital_type_str)
            .unwrap_or(VitalType::Weight);

        Ok(Self {
            id: row.get("id")?,
            vital_type,
            timestamp: row.get("timestamp")?,
            value1: row.get("value1")?,
            value2: row.get("value2")?,
            unit: row.get("unit")?,
            group_id: row.get("group_id")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Create a new vital reading
    pub fn create(conn: &Connection, data: &VitalCreate) -> DbResult<Self> {
        let timestamp = data.timestamp.clone().unwrap_or_else(|| {
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
        });
        let unit = data.unit.clone().unwrap_or_else(|| {
            data.vital_type.default_unit().to_string()
        });

        conn.execute(
            r#"
            INSERT INTO vitals (vital_type, timestamp, value1, value2, unit, group_id, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                data.vital_type.as_str(),
                timestamp,
                data.value1,
                data.value2,
                unit,
                data.group_id,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a vital by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM vitals WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(vital) => Ok(Some(vital)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List vitals by type
    pub fn list_by_type(
        conn: &Connection,
        vital_type: VitalType,
        limit: Option<i64>,
    ) -> DbResult<Vec<Self>> {
        let sql = match limit {
            Some(n) => format!(
                "SELECT * FROM vitals WHERE vital_type = ?1 ORDER BY timestamp DESC LIMIT {}",
                n
            ),
            None => "SELECT * FROM vitals WHERE vital_type = ?1 ORDER BY timestamp DESC".to_string(),
        };

        let mut stmt = conn.prepare(&sql)?;
        let vitals = stmt
            .query_map([vital_type.as_str()], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vitals)
    }

    /// List vitals by group
    pub fn list_by_group(conn: &Connection, group_id: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM vitals WHERE group_id = ?1 ORDER BY vital_type, timestamp"
        )?;
        let vitals = stmt
            .query_map([group_id], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vitals)
    }

    /// List recent vitals across all types
    pub fn list_recent(conn: &Connection, limit: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM vitals ORDER BY timestamp DESC LIMIT ?1"
        )?;
        let vitals = stmt
            .query_map([limit], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vitals)
    }

    /// List vitals by date range
    pub fn list_by_date_range(
        conn: &Connection,
        start_date: &str,
        end_date: &str,
        vital_type: Option<VitalType>,
    ) -> DbResult<Vec<Self>> {
        let sql = match vital_type {
            Some(_) => {
                "SELECT * FROM vitals WHERE timestamp >= ?1 AND timestamp <= ?2 AND vital_type = ?3 ORDER BY timestamp DESC"
            }
            None => {
                "SELECT * FROM vitals WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp DESC"
            }
        };

        let mut stmt = conn.prepare(sql)?;
        let vitals = match vital_type {
            Some(vt) => stmt
                .query_map(params![start_date, end_date, vt.as_str()], Self::from_row)?
                .collect::<Result<Vec<_>, _>>()?,
            None => stmt
                .query_map(params![start_date, end_date], Self::from_row)?
                .collect::<Result<Vec<_>, _>>()?,
        };

        Ok(vitals)
    }

    /// Update a vital
    pub fn update(conn: &Connection, id: i64, data: &VitalUpdate) -> DbResult<Option<Self>> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(v1) = data.value1 {
            updates.push(format!("value1 = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(v1));
        }
        if let Some(v2) = data.value2 {
            updates.push(format!("value2 = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(v2));
        }
        if let Some(ref unit) = data.unit {
            updates.push(format!("unit = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(unit.clone()));
        }
        if let Some(gid) = data.group_id {
            updates.push(format!("group_id = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(gid));
        }
        if let Some(ref notes) = data.notes {
            updates.push(format!("notes = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(notes.clone()));
        }

        if updates.is_empty() {
            return Self::get_by_id(conn, id);
        }

        updates.push("updated_at = datetime('now')".to_string());

        let sql = format!(
            "UPDATE vitals SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Assign a vital to a group
    pub fn assign_to_group(conn: &Connection, id: i64, group_id: Option<i64>) -> DbResult<Option<Self>> {
        conn.execute(
            "UPDATE vitals SET group_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![group_id, id],
        )?;

        Self::get_by_id(conn, id)
    }

    /// Delete a vital
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        let rows = conn.execute("DELETE FROM vitals WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Get the latest reading for each vital type
    pub fn get_latest_by_type(conn: &Connection) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT v.* FROM vitals v
            INNER JOIN (
                SELECT vital_type, MAX(timestamp) as max_ts
                FROM vitals
                GROUP BY vital_type
            ) latest ON v.vital_type = latest.vital_type AND v.timestamp = latest.max_ts
            ORDER BY v.vital_type
            "#
        )?;
        let vitals = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vitals)
    }

    /// Format the vital reading for display
    pub fn format_value(&self) -> String {
        match self.vital_type {
            VitalType::BloodPressure => {
                let diastolic = self.value2.unwrap_or(0.0);
                format!("{}/{} {}", self.value1 as i32, diastolic as i32, self.unit)
            }
            VitalType::Weight => {
                format!("{:.1} {}", self.value1, self.unit)
            }
            VitalType::HeartRate => {
                format!("{} {}", self.value1 as i32, self.unit)
            }
            VitalType::OxygenSaturation => {
                format!("{}%", self.value1 as i32)
            }
            VitalType::Glucose => {
                format!("{} {}", self.value1 as i32, self.unit)
            }
        }
    }
}
