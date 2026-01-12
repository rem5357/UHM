//! Vitals MCP Tools
//!
//! Tools for managing vital signs and health measurements.

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
