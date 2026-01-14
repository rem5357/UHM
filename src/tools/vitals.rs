//! Vitals MCP Tools
//!
//! Tools for managing vital signs and health measurements.

use std::collections::HashMap;
use serde::Serialize;

use crate::db::Database;
use crate::models::{Vital, VitalCreate, VitalGroup, VitalGroupCreate, VitalType, VitalUpdate};

/// Response for create_vital_group
#[derive(Debug, Serialize)]
pub struct CreateVitalGroupResponse {
    pub id: i64,
    pub description: Option<String>,
    pub timestamp: String,
    pub created_at: String,
}

/// Vital group summary for listing
#[derive(Debug, Serialize)]
pub struct VitalGroupSummary {
    pub id: i64,
    pub description: Option<String>,
    pub timestamp: String,
    pub vital_count: usize,
    pub vital_types: Vec<String>,
}

/// Vital group detail with vitals
#[derive(Debug, Serialize)]
pub struct VitalGroupDetail {
    pub id: i64,
    pub description: Option<String>,
    pub timestamp: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub vitals: Vec<VitalSummary>,
}

/// Response for list_vital_groups
#[derive(Debug, Serialize)]
pub struct ListVitalGroupsResponse {
    pub groups: Vec<VitalGroupSummary>,
    pub total: usize,
}

/// Response for add_vital
#[derive(Debug, Serialize)]
pub struct AddVitalResponse {
    pub id: i64,
    pub vital_type: String,
    pub value: String,
    pub timestamp: String,
    pub group_id: Option<i64>,
    pub created_at: String,
}

/// Vital summary for listing
#[derive(Debug, Serialize)]
pub struct VitalSummary {
    pub id: i64,
    pub vital_type: String,
    pub vital_type_display: String,
    pub value: String,
    pub timestamp: String,
    pub group_id: Option<i64>,
    pub notes: Option<String>,
}

/// Full vital detail
#[derive(Debug, Serialize)]
pub struct VitalDetail {
    pub id: i64,
    pub vital_type: String,
    pub vital_type_display: String,
    pub value1: f64,
    pub value2: Option<f64>,
    pub value_formatted: String,
    pub unit: String,
    pub timestamp: String,
    pub group_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Vital> for VitalSummary {
    fn from(vital: &Vital) -> Self {
        Self {
            id: vital.id,
            vital_type: vital.vital_type.as_str().to_string(),
            vital_type_display: vital.vital_type.display_name().to_string(),
            value: vital.format_value(),
            timestamp: vital.timestamp.clone(),
            group_id: vital.group_id,
            notes: vital.notes.clone(),
        }
    }
}

impl From<Vital> for VitalDetail {
    fn from(vital: Vital) -> Self {
        let value_formatted = vital.format_value();
        Self {
            id: vital.id,
            vital_type: vital.vital_type.as_str().to_string(),
            vital_type_display: vital.vital_type.display_name().to_string(),
            value1: vital.value1,
            value2: vital.value2,
            value_formatted,
            unit: vital.unit,
            timestamp: vital.timestamp,
            group_id: vital.group_id,
            notes: vital.notes,
            created_at: vital.created_at,
            updated_at: vital.updated_at,
        }
    }
}

/// Response for list_vitals
#[derive(Debug, Serialize)]
pub struct ListVitalsResponse {
    pub vitals: Vec<VitalSummary>,
    pub total: usize,
}

/// Response for get_latest_vitals
#[derive(Debug, Serialize)]
pub struct LatestVitalsResponse {
    pub vitals: Vec<VitalSummary>,
    pub as_of: String,
}

/// Response for update_vital
#[derive(Debug, Serialize)]
pub struct UpdateVitalResponse {
    pub success: bool,
    pub updated_at: String,
}

/// Response for delete operations
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub deleted_id: i64,
}

// ============================================================================
// Vital Group Tool Functions
// ============================================================================

/// Create a new vital group
pub fn create_vital_group(
    db: &Database,
    description: Option<&str>,
    timestamp: Option<&str>,
    notes: Option<&str>,
) -> Result<CreateVitalGroupResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let data = VitalGroupCreate {
        description: description.map(String::from),
        timestamp: timestamp.map(String::from),
        notes: notes.map(String::from),
    };

    let group = VitalGroup::create(&conn, &data)
        .map_err(|e| format!("Failed to create vital group: {}", e))?;

    Ok(CreateVitalGroupResponse {
        id: group.id,
        description: group.description,
        timestamp: group.timestamp,
        created_at: group.created_at,
    })
}

/// Get a vital group by ID with its vitals
pub fn get_vital_group(db: &Database, id: i64) -> Result<Option<VitalGroupDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let group = VitalGroup::get_by_id(&conn, id)
        .map_err(|e| format!("Failed to get vital group: {}", e))?;

    match group {
        Some(g) => {
            let vitals = VitalGroup::get_vitals(&conn, id)
                .map_err(|e| format!("Failed to get group vitals: {}", e))?;

            let vital_summaries: Vec<VitalSummary> = vitals.iter().map(VitalSummary::from).collect();

            Ok(Some(VitalGroupDetail {
                id: g.id,
                description: g.description,
                timestamp: g.timestamp,
                notes: g.notes,
                created_at: g.created_at,
                vitals: vital_summaries,
            }))
        }
        None => Ok(None),
    }
}

/// List vital groups
pub fn list_vital_groups(db: &Database, limit: Option<i64>) -> Result<ListVitalGroupsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let groups = VitalGroup::list(&conn, limit)
        .map_err(|e| format!("Failed to list vital groups: {}", e))?;

    let mut summaries = Vec::new();
    for group in &groups {
        let vitals = VitalGroup::get_vitals(&conn, group.id)
            .map_err(|e| format!("Failed to get group vitals: {}", e))?;

        let vital_types: Vec<String> = vitals
            .iter()
            .map(|v| v.vital_type.display_name().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        summaries.push(VitalGroupSummary {
            id: group.id,
            description: group.description.clone(),
            timestamp: group.timestamp.clone(),
            vital_count: vitals.len(),
            vital_types,
        });
    }

    let total = summaries.len();
    Ok(ListVitalGroupsResponse {
        groups: summaries,
        total,
    })
}

/// Update a vital group
pub fn update_vital_group(
    db: &Database,
    id: i64,
    description: Option<&str>,
    notes: Option<&str>,
) -> Result<Option<VitalGroupDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let updated = VitalGroup::update(
        &conn,
        id,
        description.map(String::from),
        notes.map(String::from),
    )
    .map_err(|e| format!("Failed to update vital group: {}", e))?;

    match updated {
        Some(_) => get_vital_group(db, id),
        None => Ok(None),
    }
}

/// Delete a vital group (unlinks vitals but doesn't delete them)
pub fn delete_vital_group(db: &Database, id: i64) -> Result<DeleteResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if group exists
    let existing = VitalGroup::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Vital group not found with id: {}", id));
    }

    VitalGroup::delete(&conn, id)
        .map_err(|e| format!("Failed to delete vital group: {}", e))?;

    Ok(DeleteResponse {
        success: true,
        deleted_id: id,
    })
}

// ============================================================================
// Vital Tool Functions
// ============================================================================

/// Add a new vital reading
pub fn add_vital(
    db: &Database,
    vital_type: &str,
    value1: f64,
    value2: Option<f64>,
    unit: Option<&str>,
    timestamp: Option<&str>,
    group_id: Option<i64>,
    notes: Option<&str>,
) -> Result<AddVitalResponse, String> {
    let vt = VitalType::from_str(vital_type)
        .ok_or_else(|| format!("Invalid vital type: '{}'. Valid types: weight, blood_pressure (bp), heart_rate (hr), oxygen_saturation (o2/spo2), glucose", vital_type))?;

    // Validate value2 for blood pressure
    if vt == VitalType::BloodPressure && value2.is_none() {
        return Err("Blood pressure requires both systolic (value1) and diastolic (value2) values".to_string());
    }

    // Validate positive values
    if value1 <= 0.0 {
        return Err("Value must be greater than 0".to_string());
    }
    if let Some(v2) = value2 {
        if v2 <= 0.0 {
            return Err("Value2 must be greater than 0".to_string());
        }
    }

    // Validate group exists if specified
    if let Some(gid) = group_id {
        let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;
        let group = VitalGroup::get_by_id(&conn, gid)
            .map_err(|e| format!("Database error: {}", e))?;
        if group.is_none() {
            return Err(format!("Vital group not found with id: {}", gid));
        }
    }

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let data = VitalCreate {
        vital_type: vt,
        timestamp: timestamp.map(String::from),
        value1,
        value2,
        unit: unit.map(String::from),
        group_id,
        notes: notes.map(String::from),
    };

    let vital = Vital::create(&conn, &data)
        .map_err(|e| format!("Failed to create vital: {}", e))?;

    Ok(AddVitalResponse {
        id: vital.id,
        vital_type: vital.vital_type.as_str().to_string(),
        value: vital.format_value(),
        timestamp: vital.timestamp,
        group_id: vital.group_id,
        created_at: vital.created_at,
    })
}

/// Get a vital by ID
pub fn get_vital(db: &Database, id: i64) -> Result<Option<VitalDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let vital = Vital::get_by_id(&conn, id)
        .map_err(|e| format!("Failed to get vital: {}", e))?;

    Ok(vital.map(VitalDetail::from))
}

/// List vitals by type
pub fn list_vitals_by_type(
    db: &Database,
    vital_type: &str,
    limit: Option<i64>,
) -> Result<ListVitalsResponse, String> {
    let vt = VitalType::from_str(vital_type)
        .ok_or_else(|| format!("Invalid vital type: '{}'", vital_type))?;

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let vitals = Vital::list_by_type(&conn, vt, limit)
        .map_err(|e| format!("Failed to list vitals: {}", e))?;

    let summaries: Vec<VitalSummary> = vitals.iter().map(VitalSummary::from).collect();
    let total = summaries.len();

    Ok(ListVitalsResponse {
        vitals: summaries,
        total,
    })
}

/// List recent vitals across all types
pub fn list_recent_vitals(db: &Database, limit: i64) -> Result<ListVitalsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let vitals = Vital::list_recent(&conn, limit)
        .map_err(|e| format!("Failed to list vitals: {}", e))?;

    let summaries: Vec<VitalSummary> = vitals.iter().map(VitalSummary::from).collect();
    let total = summaries.len();

    Ok(ListVitalsResponse {
        vitals: summaries,
        total,
    })
}

/// List vitals by date range
pub fn list_vitals_by_date_range(
    db: &Database,
    start_date: &str,
    end_date: &str,
    vital_type: Option<&str>,
) -> Result<ListVitalsResponse, String> {
    let vt = match vital_type {
        Some(t) => Some(VitalType::from_str(t)
            .ok_or_else(|| format!("Invalid vital type: '{}'", t))?),
        None => None,
    };

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let vitals = Vital::list_by_date_range(&conn, start_date, end_date, vt)
        .map_err(|e| format!("Failed to list vitals: {}", e))?;

    let summaries: Vec<VitalSummary> = vitals.iter().map(VitalSummary::from).collect();
    let total = summaries.len();

    Ok(ListVitalsResponse {
        vitals: summaries,
        total,
    })
}

/// Get the latest reading for each vital type
pub fn get_latest_vitals(db: &Database) -> Result<LatestVitalsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let vitals = Vital::get_latest_by_type(&conn)
        .map_err(|e| format!("Failed to get latest vitals: {}", e))?;

    let summaries: Vec<VitalSummary> = vitals.iter().map(VitalSummary::from).collect();

    Ok(LatestVitalsResponse {
        vitals: summaries,
        as_of: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    })
}

/// Update a vital reading
pub fn update_vital(
    db: &Database,
    id: i64,
    value1: Option<f64>,
    value2: Option<f64>,
    unit: Option<&str>,
    notes: Option<&str>,
) -> Result<Option<UpdateVitalResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if vital exists
    let existing = Vital::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Vital not found with id: {}", id));
    }

    // Validate positive values
    if let Some(v1) = value1 {
        if v1 <= 0.0 {
            return Err("Value1 must be greater than 0".to_string());
        }
    }
    if let Some(v2) = value2 {
        if v2 <= 0.0 {
            return Err("Value2 must be greater than 0".to_string());
        }
    }

    let data = VitalUpdate {
        value1,
        value2,
        unit: unit.map(String::from),
        group_id: None, // Use assign_vital_to_group for this
        notes: notes.map(String::from),
    };

    let updated = Vital::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update vital: {}", e))?;

    Ok(updated.map(|v| UpdateVitalResponse {
        success: true,
        updated_at: v.updated_at,
    }))
}

/// Assign a vital to a group (or remove from group with group_id = null)
pub fn assign_vital_to_group(
    db: &Database,
    vital_id: i64,
    group_id: Option<i64>,
) -> Result<VitalDetail, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Validate group exists if specified
    if let Some(gid) = group_id {
        let group = VitalGroup::get_by_id(&conn, gid)
            .map_err(|e| format!("Database error: {}", e))?;
        if group.is_none() {
            return Err(format!("Vital group not found with id: {}", gid));
        }
    }

    let updated = Vital::assign_to_group(&conn, vital_id, group_id)
        .map_err(|e| format!("Failed to assign vital to group: {}", e))?;

    match updated {
        Some(v) => Ok(VitalDetail::from(v)),
        None => Err(format!("Vital not found with id: {}", vital_id)),
    }
}

/// Delete a vital reading
pub fn delete_vital(db: &Database, id: i64) -> Result<DeleteResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if vital exists
    let existing = Vital::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Vital not found with id: {}", id));
    }

    Vital::delete(&conn, id)
        .map_err(|e| format!("Failed to delete vital: {}", e))?;

    Ok(DeleteResponse {
        success: true,
        deleted_id: id,
    })
}

// ============================================================================
// Omron CSV Import
// ============================================================================

/// Result for a single imported reading
#[derive(Debug, Serialize)]
pub struct OmronImportRow {
    pub row_num: usize,
    pub timestamp: String,
    pub systolic: i32,
    pub diastolic: i32,
    pub pulse: i32,
    pub truread: String,
    pub group_id: i64,
    pub bp_vital_id: i64,
    pub hr_vital_id: i64,
}

/// Response for Omron CSV import
#[derive(Debug, Serialize)]
pub struct OmronImportResponse {
    pub success: bool,
    pub file_path: String,
    pub total_rows: usize,
    pub imported: usize,
    pub duplicates: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub date_range: String,
    pub readings: Vec<OmronImportRow>,
    /// Number of standalone BP vitals deleted (duplicates of exercise-linked vitals)
    pub duplicates_cleaned_bp: usize,
    /// Number of standalone HR vitals deleted (duplicates of exercise-linked vitals)
    pub duplicates_cleaned_hr: usize,
}

/// Parse Omron date format "Jan 6 2026" to "2026-01-06"
fn parse_omron_date(date_str: &str) -> Result<String, String> {
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(format!("Invalid date format: {}", date_str));
    }

    let month = match parts[0].to_lowercase().as_str() {
        "jan" => "01", "feb" => "02", "mar" => "03", "apr" => "04",
        "may" => "05", "jun" => "06", "jul" => "07", "aug" => "08",
        "sep" => "09", "oct" => "10", "nov" => "11", "dec" => "12",
        _ => return Err(format!("Invalid month: {}", parts[0])),
    };

    let day: u32 = parts[1].parse().map_err(|_| format!("Invalid day: {}", parts[1]))?;
    let year: u32 = parts[2].parse().map_err(|_| format!("Invalid year: {}", parts[2]))?;

    Ok(format!("{:04}-{}-{:02}", year, month, day))
}

/// Parse Omron time format "8:18 am" to "08:18:00"
fn parse_omron_time(time_str: &str) -> Result<String, String> {
    let parts: Vec<&str> = time_str.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!("Invalid time format: {}", time_str));
    }

    let time_parts: Vec<&str> = parts[0].split(':').collect();
    if time_parts.len() != 2 {
        return Err(format!("Invalid time format: {}", time_str));
    }

    let mut hour: u32 = time_parts[0].parse().map_err(|_| format!("Invalid hour: {}", time_parts[0]))?;
    let minute: u32 = time_parts[1].parse().map_err(|_| format!("Invalid minute: {}", time_parts[1]))?;

    let am_pm = parts[1].to_lowercase();
    if am_pm == "pm" && hour != 12 {
        hour += 12;
    } else if am_pm == "am" && hour == 12 {
        hour = 0;
    }

    Ok(format!("{:02}:{:02}:00", hour, minute))
}

/// Check if a BP reading already exists with matching timestamp and values
fn bp_reading_exists(
    conn: &rusqlite::Connection,
    timestamp: &str,
    systolic: f64,
    diastolic: f64,
) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM vitals
               WHERE vital_type = 'blood_pressure'
               AND timestamp = ?1
               AND value1 = ?2
               AND value2 = ?3"#,
            rusqlite::params![timestamp, systolic, diastolic],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check for BP duplicates: {}", e))?;
    Ok(count > 0)
}

/// Check if an HR reading already exists with matching timestamp and value
fn hr_reading_exists(
    conn: &rusqlite::Connection,
    timestamp: &str,
    pulse: f64,
) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM vitals
               WHERE vital_type = 'heart_rate'
               AND timestamp = ?1
               AND value1 = ?2"#,
            rusqlite::params![timestamp, pulse],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check for HR duplicates: {}", e))?;
    Ok(count > 0)
}

/// Import Omron BP CSV file
pub fn import_omron_bp_csv(db: &Database, file_path: &str) -> Result<OmronImportResponse, String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // Read the file
    let file = File::open(file_path)
        .map_err(|e| format!("Failed to open file '{}': {}", file_path, e))?;
    let reader = BufReader::new(file);

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let mut readings = Vec::new();
    let mut errors = Vec::new();
    let mut skipped = 0;
    let mut duplicates = 0;
    let mut first_date: Option<String> = None;
    let mut last_date: Option<String> = None;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|e| format!("Error reading line {}: {}", line_num + 1, e))?;

        // Skip header row
        if line_num == 0 && line.starts_with("Date,") {
            continue;
        }

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse CSV row
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 5 {
            errors.push(format!("Row {}: Not enough fields", line_num + 1));
            skipped += 1;
            continue;
        }

        // Parse date and time
        let date = match parse_omron_date(fields[0].trim()) {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("Row {}: {}", line_num + 1, e));
                skipped += 1;
                continue;
            }
        };

        let time = match parse_omron_time(fields[1].trim()) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("Row {}: {}", line_num + 1, e));
                skipped += 1;
                continue;
            }
        };

        let timestamp = format!("{}T{}", date, time);

        // Track date range
        if first_date.is_none() {
            first_date = Some(date.clone());
        }
        last_date = Some(date.clone());

        // Parse vitals
        let systolic: i32 = match fields[2].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!("Row {}: Invalid systolic value", line_num + 1));
                skipped += 1;
                continue;
            }
        };

        let diastolic: i32 = match fields[3].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!("Row {}: Invalid diastolic value", line_num + 1));
                skipped += 1;
                continue;
            }
        };

        let pulse: i32 = match fields[4].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!("Row {}: Invalid pulse value", line_num + 1));
                skipped += 1;
                continue;
            }
        };

        // Get TruRead status
        let truread = if fields.len() > 7 {
            let tr = fields[7].trim();
            if tr == "-" { "single".to_string() } else { tr.to_lowercase() }
        } else {
            "single".to_string()
        };

        // Check for duplicate reading (same timestamp + BP values OR same timestamp + HR value)
        let bp_exists = match bp_reading_exists(&conn, &timestamp, systolic as f64, diastolic as f64) {
            Ok(exists) => exists,
            Err(e) => {
                errors.push(format!("Row {}: {}", line_num + 1, e));
                skipped += 1;
                continue;
            }
        };

        let hr_exists = match hr_reading_exists(&conn, &timestamp, pulse as f64) {
            Ok(exists) => exists,
            Err(e) => {
                errors.push(format!("Row {}: {}", line_num + 1, e));
                skipped += 1;
                continue;
            }
        };

        // If either BP or HR already exists, consider it a duplicate
        if bp_exists || hr_exists {
            duplicates += 1;
            continue;
        }

        // Create vital group for this reading
        let group_data = VitalGroupCreate {
            description: Some(format!("Omron BP reading")),
            timestamp: Some(timestamp.clone()),
            notes: if truread != "single" { Some(format!("TruRead: {}", truread)) } else { None },
        };

        let group = VitalGroup::create(&conn, &group_data)
            .map_err(|e| format!("Row {}: Failed to create group: {}", line_num + 1, e))?;

        // Create BP vital
        let bp_data = VitalCreate {
            vital_type: VitalType::BloodPressure,
            timestamp: Some(timestamp.clone()),
            value1: systolic as f64,
            value2: Some(diastolic as f64),
            unit: Some("mmHg".to_string()),
            group_id: Some(group.id),
            notes: None,
        };

        let bp_vital = Vital::create(&conn, &bp_data)
            .map_err(|e| format!("Row {}: Failed to create BP vital: {}", line_num + 1, e))?;

        // Create HR vital
        let hr_data = VitalCreate {
            vital_type: VitalType::HeartRate,
            timestamp: Some(timestamp.clone()),
            value1: pulse as f64,
            value2: None,
            unit: Some("bpm".to_string()),
            group_id: Some(group.id),
            notes: None,
        };

        let hr_vital = Vital::create(&conn, &hr_data)
            .map_err(|e| format!("Row {}: Failed to create HR vital: {}", line_num + 1, e))?;

        readings.push(OmronImportRow {
            row_num: line_num + 1,
            timestamp,
            systolic,
            diastolic,
            pulse,
            truread,
            group_id: group.id,
            bp_vital_id: bp_vital.id,
            hr_vital_id: hr_vital.id,
        });
    }

    let imported = readings.len();
    let total_rows = imported + duplicates + skipped;
    let date_range = match (last_date, first_date) {
        (Some(start), Some(end)) => format!("{} to {}", start, end),
        _ => "N/A".to_string(),
    };

    // Auto-cleanup: delete standalone vitals that duplicate exercise-linked vitals
    // Use 60 minute window to match readings that might have slight timestamp differences
    let cleanup_result = auto_cleanup_exercise_duplicates(&conn, 60)
        .unwrap_or_default();

    Ok(OmronImportResponse {
        success: errors.is_empty(),
        file_path: file_path.to_string(),
        total_rows,
        imported,
        duplicates,
        skipped,
        errors: if errors.len() > 10 { errors[..10].to_vec() } else { errors },
        date_range,
        readings,
        duplicates_cleaned_bp: cleanup_result.bp_deleted,
        duplicates_cleaned_hr: cleanup_result.hr_deleted,
    })
}

// ============================================================================
// Vital Statistics
// ============================================================================

/// An outlier reading (value outside standard deviation bounds)
#[derive(Debug, Serialize)]
pub struct VitalOutlier {
    pub timestamp: String,
    pub value: f64,
    /// How many standard deviations from mean (positive = above, negative = below)
    pub z_score: f64,
}

/// Statistics for a single vital value
#[derive(Debug, Serialize)]
pub struct SingleValueStats {
    pub count: i64,
    pub average: f64,
    pub median: f64,
    pub mode: Option<f64>,
    pub standard_deviation: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub range: f64,
    /// 25th percentile (Q1)
    pub percentile_25: f64,
    /// 75th percentile (Q3)
    pub percentile_75: f64,
    /// Interquartile range (Q3 - Q1)
    pub iqr: f64,
    /// Coefficient of variation (SD/mean * 100) - relative variability
    pub coefficient_of_variation: f64,
    /// Readings outside 1 standard deviation
    pub outliers: Vec<VitalOutlier>,
}

/// Statistics for blood pressure (dual values)
#[derive(Debug, Serialize)]
pub struct BloodPressureStats {
    pub count: i64,
    pub systolic: SingleValueStats,
    pub diastolic: SingleValueStats,
    /// Pulse pressure (systolic - diastolic) stats
    pub pulse_pressure: SingleValueStats,
}

/// Statistics for weight
#[derive(Debug, Serialize)]
pub struct WeightStats {
    pub count: i64,
    pub unit: String,
    pub stats: SingleValueStats,
    /// Weight change from first to last reading
    pub total_change: f64,
    /// Average change per reading
    pub avg_change_per_reading: f64,
}

/// Statistics for heart rate
#[derive(Debug, Serialize)]
pub struct HeartRateStats {
    pub count: i64,
    pub unit: String,
    pub stats: SingleValueStats,
}

/// Statistics for oxygen saturation
#[derive(Debug, Serialize)]
pub struct OxygenSaturationStats {
    pub count: i64,
    pub unit: String,
    pub stats: SingleValueStats,
    /// Count of readings below 95% (potential concern)
    pub below_95_count: i64,
    /// Count of readings below 90% (critical)
    pub below_90_count: i64,
}

/// Statistics for glucose
#[derive(Debug, Serialize)]
pub struct GlucoseStats {
    pub count: i64,
    pub unit: String,
    pub stats: SingleValueStats,
    /// Count of readings below 70 (hypoglycemia)
    pub low_count: i64,
    /// Count of readings above 180 (hyperglycemia)
    pub high_count: i64,
}

/// Response for list_vitals_stats
#[derive(Debug, Serialize)]
pub struct ListVitalsStatsResponse {
    pub vital_type: String,
    pub readings_analyzed: i64,
    pub date_range: Option<VitalDateRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<WeightStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_pressure: Option<BloodPressureStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heart_rate: Option<HeartRateStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oxygen_saturation: Option<OxygenSaturationStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glucose: Option<GlucoseStats>,
}

/// Date range for stats
#[derive(Debug, Serialize)]
pub struct VitalDateRange {
    pub start: String,
    pub end: String,
}

/// Internal: timestamped value for stats calculation
struct TimestampedValue {
    timestamp: String,
    value: f64,
}

/// Calculate percentile using linear interpolation
fn vital_percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len();
    let rank = (p / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let weight = rank - lower as f64;

    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

/// Calculate statistics for a list of timestamped values
fn calculate_single_stats(values: &[TimestampedValue]) -> SingleValueStats {
    if values.is_empty() {
        return SingleValueStats {
            count: 0,
            average: 0.0,
            median: 0.0,
            mode: None,
            standard_deviation: 0.0,
            variance: 0.0,
            min: 0.0,
            max: 0.0,
            range: 0.0,
            percentile_25: 0.0,
            percentile_75: 0.0,
            iqr: 0.0,
            coefficient_of_variation: 0.0,
            outliers: Vec::new(),
        };
    }

    let count = values.len() as i64;
    let mut sorted_values: Vec<f64> = values.iter().map(|v| v.value).collect();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Sum and average
    let sum: f64 = sorted_values.iter().sum();
    let average = sum / count as f64;

    // Min, max, range
    let min = sorted_values[0];
    let max = sorted_values[sorted_values.len() - 1];
    let range = max - min;

    // Median
    let median = if count % 2 == 0 {
        let mid = count as usize / 2;
        (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
    } else {
        sorted_values[count as usize / 2]
    };

    // Mode (most frequent value, rounded to 1 decimal for grouping)
    let mode = {
        let mut freq: HashMap<i64, i64> = HashMap::new();
        for v in &sorted_values {
            let key = (v * 10.0).round() as i64;
            *freq.entry(key).or_insert(0) += 1;
        }
        let max_freq = freq.values().max().copied().unwrap_or(0);
        if max_freq > 1 {
            freq.iter()
                .find(|(_, &f)| f == max_freq)
                .map(|(&k, _)| k as f64 / 10.0)
        } else {
            None
        }
    };

    // Variance and standard deviation
    let variance = if count > 1 {
        let sum_sq_diff: f64 = sorted_values.iter().map(|v| (v - average).powi(2)).sum();
        sum_sq_diff / (count - 1) as f64
    } else {
        0.0
    };
    let standard_deviation = variance.sqrt();

    // Percentiles
    let percentile_25 = vital_percentile(&sorted_values, 25.0);
    let percentile_75 = vital_percentile(&sorted_values, 75.0);
    let iqr = percentile_75 - percentile_25;

    // Coefficient of variation
    let coefficient_of_variation = if average != 0.0 {
        (standard_deviation / average) * 100.0
    } else {
        0.0
    };

    // Outliers (outside 1 standard deviation)
    let outliers: Vec<VitalOutlier> = if standard_deviation > 0.0 {
        values
            .iter()
            .filter_map(|tv| {
                let z_score = (tv.value - average) / standard_deviation;
                if z_score.abs() > 1.0 {
                    Some(VitalOutlier {
                        timestamp: tv.timestamp.clone(),
                        value: tv.value,
                        z_score: (z_score * 100.0).round() / 100.0,
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    SingleValueStats {
        count,
        average: (average * 100.0).round() / 100.0,
        median: (median * 100.0).round() / 100.0,
        mode: mode.map(|m| (m * 100.0).round() / 100.0),
        standard_deviation: (standard_deviation * 100.0).round() / 100.0,
        variance: (variance * 100.0).round() / 100.0,
        min: (min * 100.0).round() / 100.0,
        max: (max * 100.0).round() / 100.0,
        range: (range * 100.0).round() / 100.0,
        percentile_25: (percentile_25 * 100.0).round() / 100.0,
        percentile_75: (percentile_75 * 100.0).round() / 100.0,
        iqr: (iqr * 100.0).round() / 100.0,
        coefficient_of_variation: (coefficient_of_variation * 100.0).round() / 100.0,
        outliers,
    }
}

/// Get comprehensive statistics for vitals by type
pub fn list_vitals_stats(
    db: &Database,
    vital_type: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<ListVitalsStatsResponse, String> {
    let vt = VitalType::from_str(vital_type)
        .ok_or_else(|| format!("Invalid vital type: '{}'. Valid types: weight, blood_pressure (bp), heart_rate (hr), oxygen_saturation (o2/spo2), glucose", vital_type))?;

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get all vitals of this type in date range
    let vitals = if start_date.is_some() || end_date.is_some() {
        let start = start_date.unwrap_or("1900-01-01");
        let end = end_date.unwrap_or("2100-12-31");
        Vital::list_by_date_range(&conn, start, end, Some(vt))
            .map_err(|e| format!("Failed to list vitals: {}", e))?
    } else {
        Vital::list_by_type(&conn, vt, Some(10000))
            .map_err(|e| format!("Failed to list vitals: {}", e))?
    };

    if vitals.is_empty() {
        return Ok(ListVitalsStatsResponse {
            vital_type: vt.as_str().to_string(),
            readings_analyzed: 0,
            date_range: None,
            weight: None,
            blood_pressure: None,
            heart_rate: None,
            oxygen_saturation: None,
            glucose: None,
        });
    }

    // Determine date range
    let mut timestamps: Vec<&str> = vitals.iter().map(|v| v.timestamp.as_str()).collect();
    timestamps.sort();
    let date_range = Some(VitalDateRange {
        start: timestamps.first().unwrap().to_string(),
        end: timestamps.last().unwrap().to_string(),
    });

    let readings_analyzed = vitals.len() as i64;

    match vt {
        VitalType::Weight => {
            let values: Vec<TimestampedValue> = vitals
                .iter()
                .map(|v| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: v.value1,
                })
                .collect();

            let stats = calculate_single_stats(&values);

            // Calculate weight change
            let (total_change, avg_change) = if values.len() >= 2 {
                // Sort by timestamp to get first and last chronologically
                let mut sorted: Vec<_> = vitals.iter().collect();
                sorted.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                let first = sorted.first().unwrap().value1;
                let last = sorted.last().unwrap().value1;
                let total = last - first;
                let avg = if sorted.len() > 1 {
                    total / (sorted.len() - 1) as f64
                } else {
                    0.0
                };
                (total, avg)
            } else {
                (0.0, 0.0)
            };

            let unit = vitals.first().map(|v| v.unit.clone()).unwrap_or("lbs".to_string());

            Ok(ListVitalsStatsResponse {
                vital_type: vt.as_str().to_string(),
                readings_analyzed,
                date_range,
                weight: Some(WeightStats {
                    count: readings_analyzed,
                    unit,
                    stats,
                    total_change: (total_change * 100.0).round() / 100.0,
                    avg_change_per_reading: (avg_change * 100.0).round() / 100.0,
                }),
                blood_pressure: None,
                heart_rate: None,
                oxygen_saturation: None,
                glucose: None,
            })
        }

        VitalType::BloodPressure => {
            let systolic_values: Vec<TimestampedValue> = vitals
                .iter()
                .map(|v| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: v.value1,
                })
                .collect();

            let diastolic_values: Vec<TimestampedValue> = vitals
                .iter()
                .filter_map(|v| v.value2.map(|val| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: val,
                }))
                .collect();

            // Pulse pressure = systolic - diastolic
            let pulse_pressure_values: Vec<TimestampedValue> = vitals
                .iter()
                .filter_map(|v| v.value2.map(|d| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: v.value1 - d,
                }))
                .collect();

            let systolic_stats = calculate_single_stats(&systolic_values);
            let diastolic_stats = calculate_single_stats(&diastolic_values);
            let pulse_pressure_stats = calculate_single_stats(&pulse_pressure_values);

            Ok(ListVitalsStatsResponse {
                vital_type: vt.as_str().to_string(),
                readings_analyzed,
                date_range,
                weight: None,
                blood_pressure: Some(BloodPressureStats {
                    count: readings_analyzed,
                    systolic: systolic_stats,
                    diastolic: diastolic_stats,
                    pulse_pressure: pulse_pressure_stats,
                }),
                heart_rate: None,
                oxygen_saturation: None,
                glucose: None,
            })
        }

        VitalType::HeartRate => {
            let values: Vec<TimestampedValue> = vitals
                .iter()
                .map(|v| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: v.value1,
                })
                .collect();

            let stats = calculate_single_stats(&values);
            let unit = vitals.first().map(|v| v.unit.clone()).unwrap_or("bpm".to_string());

            Ok(ListVitalsStatsResponse {
                vital_type: vt.as_str().to_string(),
                readings_analyzed,
                date_range,
                weight: None,
                blood_pressure: None,
                heart_rate: Some(HeartRateStats {
                    count: readings_analyzed,
                    unit,
                    stats,
                }),
                oxygen_saturation: None,
                glucose: None,
            })
        }

        VitalType::OxygenSaturation => {
            let values: Vec<TimestampedValue> = vitals
                .iter()
                .map(|v| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: v.value1,
                })
                .collect();

            let stats = calculate_single_stats(&values);
            let unit = vitals.first().map(|v| v.unit.clone()).unwrap_or("%".to_string());

            // Count concerning readings
            let below_95_count = vitals.iter().filter(|v| v.value1 < 95.0).count() as i64;
            let below_90_count = vitals.iter().filter(|v| v.value1 < 90.0).count() as i64;

            Ok(ListVitalsStatsResponse {
                vital_type: vt.as_str().to_string(),
                readings_analyzed,
                date_range,
                weight: None,
                blood_pressure: None,
                heart_rate: None,
                oxygen_saturation: Some(OxygenSaturationStats {
                    count: readings_analyzed,
                    unit,
                    stats,
                    below_95_count,
                    below_90_count,
                }),
                glucose: None,
            })
        }

        VitalType::Glucose => {
            let values: Vec<TimestampedValue> = vitals
                .iter()
                .map(|v| TimestampedValue {
                    timestamp: v.timestamp.clone(),
                    value: v.value1,
                })
                .collect();

            let stats = calculate_single_stats(&values);
            let unit = vitals.first().map(|v| v.unit.clone()).unwrap_or("mg/dL".to_string());

            // Count concerning readings
            let low_count = vitals.iter().filter(|v| v.value1 < 70.0).count() as i64;
            let high_count = vitals.iter().filter(|v| v.value1 > 180.0).count() as i64;

            Ok(ListVitalsStatsResponse {
                vital_type: vt.as_str().to_string(),
                readings_analyzed,
                date_range,
                weight: None,
                blood_pressure: None,
                heart_rate: None,
                oxygen_saturation: None,
                glucose: Some(GlucoseStats {
                    count: readings_analyzed,
                    unit,
                    stats,
                    low_count,
                    high_count,
                }),
            })
        }
    }
}

// ============================================================================
// Duplicate Detection
// ============================================================================

/// A potential duplicate pair of vitals
#[derive(Debug, Serialize)]
pub struct DuplicatePair {
    /// The vital in an exercise-linked group (likely the one to keep)
    pub exercise_vital: VitalSummary,
    /// The exercise this vital is linked to (pre or post)
    pub exercise_context: String,
    /// The standalone vital that may be a duplicate
    pub standalone_vital: VitalSummary,
    /// Time difference in minutes between the two readings
    pub time_diff_minutes: f64,
    /// Whether the values match exactly
    pub values_match: bool,
}

/// Response for find_duplicate_vitals
#[derive(Debug, Serialize)]
pub struct FindDuplicateVitalsResponse {
    pub duplicate_pairs: Vec<DuplicatePair>,
    pub total_found: usize,
    pub exercise_vitals_checked: i64,
    pub standalone_vitals_checked: i64,
    pub time_window_minutes: i64,
}

/// Find potential duplicate BP/HR vitals between exercise groups and standalone readings
pub fn find_duplicate_vitals(
    db: &Database,
    vital_type: Option<&str>,
    time_window_minutes: Option<i64>,
) -> Result<FindDuplicateVitalsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;
    let window = time_window_minutes.unwrap_or(60); // Default 60 minute window

    // Filter by vital type if specified (default to BP and HR which are most common with exercises)
    let type_filter = match vital_type {
        Some(t) => {
            let vt = VitalType::from_str(t)
                .ok_or_else(|| format!("Invalid vital type: '{}'", t))?;
            format!("AND v.vital_type = '{}'", vt.as_str())
        }
        None => "AND v.vital_type IN ('blood_pressure', 'heart_rate')".to_string(),
    };

    // Find all vitals that are in exercise-linked vital groups
    let exercise_vitals_sql = format!(
        r#"
        SELECT v.*,
               CASE
                   WHEN e.pre_vital_group_id = v.group_id THEN 'PRE'
                   WHEN e.post_vital_group_id = v.group_id THEN 'POST'
               END as exercise_context,
               e.id as exercise_id,
               e.exercise_type,
               e.timestamp as exercise_timestamp
        FROM vitals v
        JOIN vital_groups vg ON v.group_id = vg.id
        JOIN exercises e ON (e.pre_vital_group_id = vg.id OR e.post_vital_group_id = vg.id)
        WHERE v.group_id IS NOT NULL
        {}
        ORDER BY v.timestamp
        "#,
        type_filter
    );

    let mut exercise_stmt = conn.prepare(&exercise_vitals_sql)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    struct ExerciseVital {
        vital: Vital,
        exercise_context: String,
        exercise_id: i64,
        exercise_type: String,
        exercise_timestamp: String,
    }

    let exercise_vitals: Vec<ExerciseVital> = exercise_stmt
        .query_map([], |row| {
            let vital_type_str: String = row.get("vital_type")?;
            let vital_type = VitalType::from_str(&vital_type_str)
                .unwrap_or(VitalType::Weight);

            Ok(ExerciseVital {
                vital: Vital {
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
                },
                exercise_context: row.get("exercise_context")?,
                exercise_id: row.get("exercise_id")?,
                exercise_type: row.get("exercise_type")?,
                exercise_timestamp: row.get("exercise_timestamp")?,
            })
        })
        .map_err(|e| format!("Failed to query exercise vitals: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect exercise vitals: {}", e))?;

    let exercise_vitals_count = exercise_vitals.len() as i64;

    // Collect group IDs that are linked to exercises (for reference)
    let _exercise_group_ids: Vec<i64> = exercise_vitals
        .iter()
        .filter_map(|ev| ev.vital.group_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Find standalone vitals (not in exercise-linked groups)
    let standalone_sql = format!(
        r#"
        SELECT * FROM vitals v
        WHERE (v.group_id IS NULL OR v.group_id NOT IN (
            SELECT vg.id FROM vital_groups vg
            JOIN exercises e ON (e.pre_vital_group_id = vg.id OR e.post_vital_group_id = vg.id)
        ))
        {}
        ORDER BY v.timestamp
        "#,
        type_filter
    );

    let mut standalone_stmt = conn.prepare(&standalone_sql)
        .map_err(|e| format!("Failed to prepare standalone query: {}", e))?;

    let standalone_vitals: Vec<Vital> = standalone_stmt
        .query_map([], |row| {
            let vital_type_str: String = row.get("vital_type")?;
            let vital_type = VitalType::from_str(&vital_type_str)
                .unwrap_or(VitalType::Weight);

            Ok(Vital {
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
        })
        .map_err(|e| format!("Failed to query standalone vitals: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect standalone vitals: {}", e))?;

    let standalone_vitals_count = standalone_vitals.len() as i64;

    // Find duplicates by matching values and timestamps within window
    let mut duplicate_pairs = Vec::new();

    for ev in &exercise_vitals {
        let ev_ts = chrono::NaiveDateTime::parse_from_str(&ev.vital.timestamp, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(&ev.vital.timestamp, "%Y-%m-%dT%H:%M:%SZ"))
            .ok();

        for sv in &standalone_vitals {
            // Skip if different vital types
            if ev.vital.vital_type != sv.vital_type {
                continue;
            }

            // Check if values match
            let values_match = match ev.vital.vital_type {
                VitalType::BloodPressure => {
                    ev.vital.value1 == sv.value1 && ev.vital.value2 == sv.value2
                }
                _ => ev.vital.value1 == sv.value1,
            };

            // Parse timestamps and check time difference
            let sv_ts = chrono::NaiveDateTime::parse_from_str(&sv.timestamp, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&sv.timestamp, "%Y-%m-%dT%H:%M:%SZ"))
                .ok();

            let time_diff_minutes = match (ev_ts, sv_ts) {
                (Some(e), Some(s)) => {
                    let diff = (e - s).num_seconds().abs() as f64 / 60.0;
                    diff
                }
                _ => f64::MAX, // Can't compare, skip
            };

            // If values match and within time window, it's a potential duplicate
            if values_match && time_diff_minutes <= window as f64 {
                duplicate_pairs.push(DuplicatePair {
                    exercise_vital: VitalSummary::from(&ev.vital),
                    exercise_context: format!(
                        "{} exercise #{} ({}) at {}",
                        ev.exercise_context,
                        ev.exercise_id,
                        ev.exercise_type,
                        ev.exercise_timestamp
                    ),
                    standalone_vital: VitalSummary::from(sv),
                    time_diff_minutes: (time_diff_minutes * 10.0).round() / 10.0,
                    values_match,
                });
            }
        }
    }

    let total_found = duplicate_pairs.len();

    Ok(FindDuplicateVitalsResponse {
        duplicate_pairs,
        total_found,
        exercise_vitals_checked: exercise_vitals_count,
        standalone_vitals_checked: standalone_vitals_count,
        time_window_minutes: window,
    })
}

/// Delete multiple vitals by ID (for bulk duplicate removal)
pub fn delete_vitals_bulk(
    db: &Database,
    vital_ids: &[i64],
) -> Result<BulkDeleteResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let mut deleted = Vec::new();
    let mut not_found = Vec::new();
    let mut errors = Vec::new();

    for &id in vital_ids {
        // Check if vital exists
        match Vital::get_by_id(&conn, id) {
            Ok(Some(_)) => {
                match Vital::delete(&conn, id) {
                    Ok(true) => deleted.push(id),
                    Ok(false) => not_found.push(id),
                    Err(e) => errors.push(format!("ID {}: {}", id, e)),
                }
            }
            Ok(None) => not_found.push(id),
            Err(e) => errors.push(format!("ID {}: {}", id, e)),
        }
    }

    Ok(BulkDeleteResponse {
        success: errors.is_empty() && not_found.is_empty(),
        deleted_count: deleted.len(),
        deleted_ids: deleted,
        not_found_ids: not_found,
        errors,
    })
}

/// Response for bulk delete
#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub success: bool,
    pub deleted_count: usize,
    pub deleted_ids: Vec<i64>,
    pub not_found_ids: Vec<i64>,
    pub errors: Vec<String>,
}

/// Result of auto-cleanup of duplicate vitals
#[derive(Debug, Default)]
pub struct AutoCleanupResult {
    pub bp_deleted: usize,
    pub hr_deleted: usize,
}

/// Automatically delete standalone vitals that duplicate exercise-linked vitals.
/// Keeps the exercise-linked vitals (which have context like PRE/POST workout notes).
/// Returns count of BP and HR vitals deleted.
pub fn auto_cleanup_exercise_duplicates(
    conn: &rusqlite::Connection,
    time_window_minutes: i64,
) -> Result<AutoCleanupResult, String> {
    // Find all vitals that are in exercise-linked vital groups
    let exercise_vitals_sql = r#"
        SELECT v.id, v.vital_type, v.timestamp, v.value1, v.value2, v.group_id
        FROM vitals v
        JOIN vital_groups vg ON v.group_id = vg.id
        JOIN exercises e ON (e.pre_vital_group_id = vg.id OR e.post_vital_group_id = vg.id)
        WHERE v.group_id IS NOT NULL
        AND v.vital_type IN ('blood_pressure', 'heart_rate')
        ORDER BY v.timestamp
    "#;

    struct ExerciseVitalRef {
        #[allow(dead_code)]
        id: i64,
        vital_type: String,
        timestamp: String,
        value1: f64,
        value2: Option<f64>,
        #[allow(dead_code)]
        group_id: i64,
    }

    let mut stmt = conn.prepare(exercise_vitals_sql)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let exercise_vitals: Vec<ExerciseVitalRef> = stmt
        .query_map([], |row| {
            Ok(ExerciseVitalRef {
                id: row.get("id")?,
                vital_type: row.get("vital_type")?,
                timestamp: row.get("timestamp")?,
                value1: row.get("value1")?,
                value2: row.get("value2")?,
                group_id: row.get("group_id")?,
            })
        })
        .map_err(|e| format!("Failed to query exercise vitals: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect exercise vitals: {}", e))?;

    if exercise_vitals.is_empty() {
        return Ok(AutoCleanupResult::default());
    }

    // Find standalone vitals (not in exercise-linked groups)
    let standalone_sql = r#"
        SELECT v.id, v.vital_type, v.timestamp, v.value1, v.value2, v.group_id
        FROM vitals v
        WHERE (v.group_id IS NULL OR v.group_id NOT IN (
            SELECT vg.id FROM vital_groups vg
            JOIN exercises e ON (e.pre_vital_group_id = vg.id OR e.post_vital_group_id = vg.id)
        ))
        AND v.vital_type IN ('blood_pressure', 'heart_rate')
        ORDER BY v.timestamp
    "#;

    struct StandaloneVitalRef {
        id: i64,
        vital_type: String,
        timestamp: String,
        value1: f64,
        value2: Option<f64>,
    }

    let mut standalone_stmt = conn.prepare(standalone_sql)
        .map_err(|e| format!("Failed to prepare standalone query: {}", e))?;

    let standalone_vitals: Vec<StandaloneVitalRef> = standalone_stmt
        .query_map([], |row| {
            Ok(StandaloneVitalRef {
                id: row.get("id")?,
                vital_type: row.get("vital_type")?,
                timestamp: row.get("timestamp")?,
                value1: row.get("value1")?,
                value2: row.get("value2")?,
            })
        })
        .map_err(|e| format!("Failed to query standalone vitals: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect standalone vitals: {}", e))?;

    // Find duplicates and collect IDs to delete
    let mut bp_to_delete = Vec::new();
    let mut hr_to_delete = Vec::new();

    for ev in &exercise_vitals {
        let ev_ts = chrono::NaiveDateTime::parse_from_str(&ev.timestamp, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(&ev.timestamp, "%Y-%m-%dT%H:%M:%SZ"))
            .ok();

        for sv in &standalone_vitals {
            // Must be same vital type
            if ev.vital_type != sv.vital_type {
                continue;
            }

            // Check if values match
            let values_match = if ev.vital_type == "blood_pressure" {
                ev.value1 == sv.value1 && ev.value2 == sv.value2
            } else {
                ev.value1 == sv.value1
            };

            if !values_match {
                continue;
            }

            // Check timestamp within window
            let sv_ts = chrono::NaiveDateTime::parse_from_str(&sv.timestamp, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&sv.timestamp, "%Y-%m-%dT%H:%M:%SZ"))
                .ok();

            let within_window = match (ev_ts, sv_ts) {
                (Some(e), Some(s)) => {
                    let diff_minutes = (e - s).num_seconds().abs() as f64 / 60.0;
                    diff_minutes <= time_window_minutes as f64
                }
                _ => false,
            };

            if within_window {
                if sv.vital_type == "blood_pressure" {
                    if !bp_to_delete.contains(&sv.id) {
                        bp_to_delete.push(sv.id);
                    }
                } else {
                    if !hr_to_delete.contains(&sv.id) {
                        hr_to_delete.push(sv.id);
                    }
                }
            }
        }
    }

    // Delete the duplicates
    for id in &bp_to_delete {
        let _ = Vital::delete(conn, *id);
    }
    for id in &hr_to_delete {
        let _ = Vital::delete(conn, *id);
    }

    Ok(AutoCleanupResult {
        bp_deleted: bp_to_delete.len(),
        hr_deleted: hr_to_delete.len(),
    })
}
