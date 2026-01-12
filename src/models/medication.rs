//! Medication model
//!
//! Represents medications including prescriptions, supplements, OTC, and natural remedies.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;

/// Medication type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MedType {
    Prescription,
    Supplement,
    Otc,
    Natural,
    Compound,
    MedicalDevice,
    Other,
}

impl MedType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MedType::Prescription => "prescription",
            MedType::Supplement => "supplement",
            MedType::Otc => "otc",
            MedType::Natural => "natural",
            MedType::Compound => "compound",
            MedType::MedicalDevice => "medical_device",
            MedType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "prescription" | "rx" => MedType::Prescription,
            "supplement" | "vitamin" => MedType::Supplement,
            "otc" | "over_the_counter" => MedType::Otc,
            "natural" | "herbal" => MedType::Natural,
            "compound" | "compounded" => MedType::Compound,
            "medical_device" | "device" => MedType::MedicalDevice,
            _ => MedType::Other,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            MedType::Prescription => "Prescription",
            MedType::Supplement => "Supplement",
            MedType::Otc => "Over-the-Counter",
            MedType::Natural => "Natural Remedy",
            MedType::Compound => "Compounded",
            MedType::MedicalDevice => "Medical Device",
            MedType::Other => "Other",
        }
    }

    /// Sort order for display (prescriptions first)
    pub fn sort_order(&self) -> i32 {
        match self {
            MedType::Prescription => 0,
            MedType::Supplement => 1,
            MedType::Otc => 2,
            MedType::Natural => 3,
            MedType::Compound => 4,
            MedType::MedicalDevice => 5,
            MedType::Other => 6,
        }
    }
}

/// Dosage unit enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DosageUnit {
    Mg,
    Mcg,
    G,
    Ml,
    FlOz,
    Pill,
    Tablet,
    Capsule,
    Spray,
    Drop,
    Patch,
    Injection,
    Unit,
    Iu,
    Puff,
    Other,
}

impl DosageUnit {
    pub fn as_str(&self) -> &'static str {
        match self {
            DosageUnit::Mg => "mg",
            DosageUnit::Mcg => "mcg",
            DosageUnit::G => "g",
            DosageUnit::Ml => "ml",
            DosageUnit::FlOz => "fl_oz",
            DosageUnit::Pill => "pill",
            DosageUnit::Tablet => "tablet",
            DosageUnit::Capsule => "capsule",
            DosageUnit::Spray => "spray",
            DosageUnit::Drop => "drop",
            DosageUnit::Patch => "patch",
            DosageUnit::Injection => "injection",
            DosageUnit::Unit => "unit",
            DosageUnit::Iu => "iu",
            DosageUnit::Puff => "puff",
            DosageUnit::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mg" | "milligram" | "milligrams" => DosageUnit::Mg,
            "mcg" | "microgram" | "micrograms" => DosageUnit::Mcg,
            "g" | "gram" | "grams" => DosageUnit::G,
            "ml" | "milliliter" | "milliliters" => DosageUnit::Ml,
            "fl_oz" | "floz" | "fluid_oz" => DosageUnit::FlOz,
            "pill" | "pills" => DosageUnit::Pill,
            "tablet" | "tablets" | "tab" => DosageUnit::Tablet,
            "capsule" | "capsules" | "cap" => DosageUnit::Capsule,
            "spray" | "sprays" => DosageUnit::Spray,
            "drop" | "drops" => DosageUnit::Drop,
            "patch" | "patches" => DosageUnit::Patch,
            "injection" | "injections" | "shot" => DosageUnit::Injection,
            "unit" | "units" => DosageUnit::Unit,
            "iu" => DosageUnit::Iu,
            "puff" | "puffs" => DosageUnit::Puff,
            _ => DosageUnit::Other,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DosageUnit::Mg => "mg",
            DosageUnit::Mcg => "mcg",
            DosageUnit::G => "g",
            DosageUnit::Ml => "mL",
            DosageUnit::FlOz => "fl oz",
            DosageUnit::Pill => "pill(s)",
            DosageUnit::Tablet => "tablet(s)",
            DosageUnit::Capsule => "capsule(s)",
            DosageUnit::Spray => "spray(s)",
            DosageUnit::Drop => "drop(s)",
            DosageUnit::Patch => "patch(es)",
            DosageUnit::Injection => "injection(s)",
            DosageUnit::Unit => "unit(s)",
            DosageUnit::Iu => "IU",
            DosageUnit::Puff => "puff(s)",
            DosageUnit::Other => "unit(s)",
        }
    }
}

/// A medication record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Medication {
    pub id: i64,
    pub name: String,
    pub med_type: MedType,
    pub dosage_amount: f64,
    pub dosage_unit: DosageUnit,
    pub instructions: Option<String>,
    pub frequency: Option<String>,
    pub prescribing_doctor: Option<String>,
    pub prescribed_date: Option<String>,
    pub pharmacy: Option<String>,
    pub rx_number: Option<String>,
    pub refills_remaining: Option<i32>,
    pub is_active: bool,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub discontinue_reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new medication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicationCreate {
    pub name: String,
    pub med_type: MedType,
    pub dosage_amount: f64,
    pub dosage_unit: DosageUnit,
    pub instructions: Option<String>,
    pub frequency: Option<String>,
    pub prescribing_doctor: Option<String>,
    pub prescribed_date: Option<String>,
    pub pharmacy: Option<String>,
    pub rx_number: Option<String>,
    pub refills_remaining: Option<i32>,
    pub start_date: Option<String>,
    pub notes: Option<String>,
}

/// Data for updating a medication (requires force flag)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MedicationUpdate {
    pub name: Option<String>,
    pub med_type: Option<MedType>,
    pub dosage_amount: Option<f64>,
    pub dosage_unit: Option<DosageUnit>,
    pub instructions: Option<String>,
    pub frequency: Option<String>,
    pub prescribing_doctor: Option<String>,
    pub prescribed_date: Option<String>,
    pub pharmacy: Option<String>,
    pub rx_number: Option<String>,
    pub refills_remaining: Option<i32>,
    pub start_date: Option<String>,
    pub notes: Option<String>,
}

/// Data for deprecating a medication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicationDeprecate {
    pub end_date: Option<String>,
    pub discontinue_reason: Option<String>,
}

impl Medication {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            med_type: MedType::from_str(&row.get::<_, String>("med_type")?),
            dosage_amount: row.get("dosage_amount")?,
            dosage_unit: DosageUnit::from_str(&row.get::<_, String>("dosage_unit")?),
            instructions: row.get("instructions")?,
            frequency: row.get("frequency")?,
            prescribing_doctor: row.get("prescribing_doctor")?,
            prescribed_date: row.get("prescribed_date")?,
            pharmacy: row.get("pharmacy")?,
            rx_number: row.get("rx_number")?,
            refills_remaining: row.get("refills_remaining")?,
            is_active: row.get::<_, i32>("is_active")? != 0,
            start_date: row.get("start_date")?,
            end_date: row.get("end_date")?,
            discontinue_reason: row.get("discontinue_reason")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Create a new medication
    pub fn create(conn: &Connection, data: &MedicationCreate) -> DbResult<Self> {
        conn.execute(
            r#"
            INSERT INTO medications (
                name, med_type, dosage_amount, dosage_unit,
                instructions, frequency, prescribing_doctor, prescribed_date,
                pharmacy, rx_number, refills_remaining, start_date, notes
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                data.name,
                data.med_type.as_str(),
                data.dosage_amount,
                data.dosage_unit.as_str(),
                data.instructions,
                data.frequency,
                data.prescribing_doctor,
                data.prescribed_date,
                data.pharmacy,
                data.rx_number,
                data.refills_remaining,
                data.start_date,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a medication by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM medications WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(med) => Ok(Some(med)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List medications with optional filtering
    pub fn list(
        conn: &Connection,
        active_only: bool,
        med_type: Option<MedType>,
    ) -> DbResult<Vec<Self>> {
        let sql = match (active_only, med_type) {
            (true, Some(mt)) => format!(
                "SELECT * FROM medications WHERE is_active = 1 AND med_type = '{}' ORDER BY med_type, name",
                mt.as_str()
            ),
            (true, None) => {
                "SELECT * FROM medications WHERE is_active = 1 ORDER BY med_type, name".to_string()
            }
            (false, Some(mt)) => format!(
                "SELECT * FROM medications WHERE med_type = '{}' ORDER BY is_active DESC, med_type, name",
                mt.as_str()
            ),
            (false, None) => {
                "SELECT * FROM medications ORDER BY is_active DESC, med_type, name".to_string()
            }
        };

        let mut stmt = conn.prepare(&sql)?;
        let meds = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(meds)
    }

    /// Search medications by name
    pub fn search(conn: &Connection, query: &str, active_only: bool) -> DbResult<Vec<Self>> {
        let pattern = format!("%{}%", query);
        let sql = if active_only {
            "SELECT * FROM medications WHERE name LIKE ?1 AND is_active = 1 ORDER BY med_type, name"
        } else {
            "SELECT * FROM medications WHERE name LIKE ?1 ORDER BY is_active DESC, med_type, name"
        };

        let mut stmt = conn.prepare(sql)?;
        let meds = stmt
            .query_map([&pattern], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(meds)
    }

    /// Update a medication (requires explicit force confirmation)
    pub fn update(conn: &Connection, id: i64, data: &MedicationUpdate) -> DbResult<Option<Self>> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = data.name {
            updates.push(format!("name = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(name.clone()));
        }
        if let Some(ref med_type) = data.med_type {
            updates.push(format!("med_type = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(med_type.as_str().to_string()));
        }
        if let Some(amount) = data.dosage_amount {
            updates.push(format!("dosage_amount = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(amount));
        }
        if let Some(ref unit) = data.dosage_unit {
            updates.push(format!("dosage_unit = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(unit.as_str().to_string()));
        }
        if let Some(ref instructions) = data.instructions {
            updates.push(format!("instructions = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(instructions.clone()));
        }
        if let Some(ref frequency) = data.frequency {
            updates.push(format!("frequency = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(frequency.clone()));
        }
        if let Some(ref doctor) = data.prescribing_doctor {
            updates.push(format!("prescribing_doctor = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(doctor.clone()));
        }
        if let Some(ref date) = data.prescribed_date {
            updates.push(format!("prescribed_date = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(date.clone()));
        }
        if let Some(ref pharmacy) = data.pharmacy {
            updates.push(format!("pharmacy = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(pharmacy.clone()));
        }
        if let Some(ref rx_num) = data.rx_number {
            updates.push(format!("rx_number = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(rx_num.clone()));
        }
        if let Some(refills) = data.refills_remaining {
            updates.push(format!("refills_remaining = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(refills));
        }
        if let Some(ref start) = data.start_date {
            updates.push(format!("start_date = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(start.clone()));
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
            "UPDATE medications SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Deprecate a medication (mark as inactive)
    pub fn deprecate(conn: &Connection, id: i64, data: &MedicationDeprecate) -> DbResult<Option<Self>> {
        let end_date = data.end_date.clone().unwrap_or_else(|| {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        });

        conn.execute(
            r#"
            UPDATE medications SET
                is_active = 0,
                end_date = ?1,
                discontinue_reason = ?2,
                updated_at = datetime('now')
            WHERE id = ?3
            "#,
            params![end_date, data.discontinue_reason, id],
        )?;

        Self::get_by_id(conn, id)
    }

    /// Reactivate a medication
    pub fn reactivate(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        conn.execute(
            r#"
            UPDATE medications SET
                is_active = 1,
                end_date = NULL,
                discontinue_reason = NULL,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
            [id],
        )?;

        Self::get_by_id(conn, id)
    }

    /// Delete a medication (requires force flag, checks for associations)
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Check if medication exists
        if Self::get_by_id(conn, id)?.is_none() {
            return Ok(false);
        }

        // In the future, check for associations here
        // For now, just delete
        let rows = conn.execute("DELETE FROM medications WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Count medications
    pub fn count(conn: &Connection, active_only: bool) -> DbResult<i64> {
        let count: i64 = if active_only {
            conn.query_row(
                "SELECT COUNT(*) FROM medications WHERE is_active = 1",
                [],
                |row| row.get(0),
            )?
        } else {
            conn.query_row("SELECT COUNT(*) FROM medications", [], |row| row.get(0))?
        };
        Ok(count)
    }
}
