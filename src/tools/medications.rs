//! Medication MCP Tools
//!
//! Tools for managing medications including prescriptions, supplements, OTC, and natural remedies.

use serde::Serialize;

use crate::db::Database;
use crate::models::{
    DosageUnit, MedType, Medication, MedicationCreate, MedicationDeprecate, MedicationUpdate,
};

/// Response for add_medication
#[derive(Debug, Serialize)]
pub struct AddMedicationResponse {
    pub id: i64,
    pub name: String,
    pub med_type: String,
    pub dosage: String,
    pub is_active: bool,
    pub created_at: String,
}

/// Medication summary for listing
#[derive(Debug, Serialize)]
pub struct MedicationSummary {
    pub id: i64,
    pub name: String,
    pub med_type: String,
    pub dosage: String,
    pub frequency: Option<String>,
    pub is_active: bool,
    pub prescribing_doctor: Option<String>,
}

/// Full medication detail
#[derive(Debug, Serialize)]
pub struct MedicationDetail {
    pub id: i64,
    pub name: String,
    pub med_type: String,
    pub med_type_display: String,
    pub dosage_amount: f64,
    pub dosage_unit: String,
    pub dosage_display: String,
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

impl From<&Medication> for MedicationSummary {
    fn from(med: &Medication) -> Self {
        Self {
            id: med.id,
            name: med.name.clone(),
            med_type: med.med_type.as_str().to_string(),
            dosage: format!("{} {}", med.dosage_amount, med.dosage_unit.display_name()),
            frequency: med.frequency.clone(),
            is_active: med.is_active,
            prescribing_doctor: med.prescribing_doctor.clone(),
        }
    }
}

impl From<Medication> for MedicationDetail {
    fn from(med: Medication) -> Self {
        Self {
            id: med.id,
            name: med.name,
            med_type: med.med_type.as_str().to_string(),
            med_type_display: med.med_type.display_name().to_string(),
            dosage_amount: med.dosage_amount,
            dosage_unit: med.dosage_unit.as_str().to_string(),
            dosage_display: format!("{} {}", med.dosage_amount, med.dosage_unit.display_name()),
            instructions: med.instructions,
            frequency: med.frequency,
            prescribing_doctor: med.prescribing_doctor,
            prescribed_date: med.prescribed_date,
            pharmacy: med.pharmacy,
            rx_number: med.rx_number,
            refills_remaining: med.refills_remaining,
            is_active: med.is_active,
            start_date: med.start_date,
            end_date: med.end_date,
            discontinue_reason: med.discontinue_reason,
            notes: med.notes,
            created_at: med.created_at,
            updated_at: med.updated_at,
        }
    }
}

/// Response for list_medications
#[derive(Debug, Serialize)]
pub struct ListMedicationsResponse {
    pub medications: Vec<MedicationSummary>,
    pub total: usize,
    pub active_count: i64,
    pub inactive_count: i64,
}

/// Response for update_medication when blocked (no force flag)
#[derive(Debug, Serialize)]
pub struct UpdateMedicationBlockedResponse {
    pub error: String,
    pub requires_force: bool,
    pub recommendation: String,
}

/// Response for successful update_medication
#[derive(Debug, Serialize)]
pub struct UpdateMedicationSuccessResponse {
    pub success: bool,
    pub updated_at: String,
    pub warning: Option<String>,
}

/// Response for deprecate_medication
#[derive(Debug, Serialize)]
pub struct DeprecateMedicationResponse {
    pub success: bool,
    pub id: i64,
    pub name: String,
    pub end_date: String,
    pub discontinue_reason: Option<String>,
}

/// Response for delete_medication when blocked
#[derive(Debug, Serialize)]
pub struct DeleteMedicationBlockedResponse {
    pub error: String,
    pub requires_force: bool,
}

/// Response for successful delete_medication
#[derive(Debug, Serialize)]
pub struct DeleteMedicationSuccessResponse {
    pub success: bool,
    pub deleted_id: i64,
}

/// Response for export_medications_markdown
#[derive(Debug, Serialize)]
pub struct ExportMedicationsResponse {
    pub markdown: String,
    pub medication_count: usize,
    pub generated_at: String,
}

// ============================================================================
// Tool Functions
// ============================================================================

/// Add a new medication
pub fn add_medication(db: &Database, data: MedicationCreate) -> Result<AddMedicationResponse, String> {
    // Validate name
    let name = data.name.trim();
    if name.is_empty() {
        return Err("Medication name cannot be empty".to_string());
    }

    // Validate dosage
    if data.dosage_amount <= 0.0 {
        return Err("Dosage amount must be greater than 0".to_string());
    }

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let med = Medication::create(&conn, &data)
        .map_err(|e| format!("Failed to create medication: {}", e))?;

    Ok(AddMedicationResponse {
        id: med.id,
        name: med.name,
        med_type: med.med_type.as_str().to_string(),
        dosage: format!("{} {}", med.dosage_amount, med.dosage_unit.display_name()),
        is_active: med.is_active,
        created_at: med.created_at,
    })
}

/// Get a medication by ID
pub fn get_medication(db: &Database, id: i64) -> Result<Option<MedicationDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let med = Medication::get_by_id(&conn, id)
        .map_err(|e| format!("Failed to get medication: {}", e))?;

    Ok(med.map(MedicationDetail::from))
}

/// List medications with optional filtering
pub fn list_medications(
    db: &Database,
    active_only: bool,
    med_type: Option<&str>,
) -> Result<ListMedicationsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let med_type_filter = med_type.map(MedType::from_str);

    let meds = Medication::list(&conn, active_only, med_type_filter)
        .map_err(|e| format!("Failed to list medications: {}", e))?;

    let active_count = Medication::count(&conn, true)
        .map_err(|e| format!("Failed to count medications: {}", e))?;
    let total_count = Medication::count(&conn, false)
        .map_err(|e| format!("Failed to count medications: {}", e))?;

    let summaries: Vec<MedicationSummary> = meds.iter().map(MedicationSummary::from).collect();
    let total = summaries.len();

    Ok(ListMedicationsResponse {
        medications: summaries,
        total,
        active_count,
        inactive_count: total_count - active_count,
    })
}

/// Search medications by name
pub fn search_medications(
    db: &Database,
    query: &str,
    active_only: bool,
) -> Result<ListMedicationsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let meds = Medication::search(&conn, query, active_only)
        .map_err(|e| format!("Failed to search medications: {}", e))?;

    let active_count = Medication::count(&conn, true)
        .map_err(|e| format!("Failed to count medications: {}", e))?;
    let total_count = Medication::count(&conn, false)
        .map_err(|e| format!("Failed to count medications: {}", e))?;

    let summaries: Vec<MedicationSummary> = meds.iter().map(MedicationSummary::from).collect();
    let total = summaries.len();

    Ok(ListMedicationsResponse {
        medications: summaries,
        total,
        active_count,
        inactive_count: total_count - active_count,
    })
}

/// Update a medication (requires force flag)
pub fn update_medication(
    db: &Database,
    id: i64,
    data: MedicationUpdate,
    force: bool,
) -> Result<Result<UpdateMedicationSuccessResponse, UpdateMedicationBlockedResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if medication exists
    let existing = Medication::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Medication not found with id: {}", id));
    }

    // If not forced, block the update
    if !force {
        return Ok(Err(UpdateMedicationBlockedResponse {
            error: "Medication updates require explicit confirmation (force=true)".to_string(),
            requires_force: true,
            recommendation: "For dosage changes, consider deprecating this medication and adding a new one with the updated dosage to maintain history.".to_string(),
        }));
    }

    let updated = Medication::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update medication: {}", e))?;

    match updated {
        Some(med) => Ok(Ok(UpdateMedicationSuccessResponse {
            success: true,
            updated_at: med.updated_at,
            warning: Some("Medication modified. Consider if this change should have been a deprecation + new entry instead.".to_string()),
        })),
        None => Err("Medication not found or update failed".to_string()),
    }
}

/// Deprecate a medication (mark as inactive)
pub fn deprecate_medication(
    db: &Database,
    id: i64,
    end_date: Option<&str>,
    reason: Option<&str>,
) -> Result<DeprecateMedicationResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if medication exists
    let existing = Medication::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Medication not found with id: {}", id));
    }

    let data = MedicationDeprecate {
        end_date: end_date.map(String::from),
        discontinue_reason: reason.map(String::from),
    };

    let updated = Medication::deprecate(&conn, id, &data)
        .map_err(|e| format!("Failed to deprecate medication: {}", e))?;

    match updated {
        Some(med) => Ok(DeprecateMedicationResponse {
            success: true,
            id: med.id,
            name: med.name,
            end_date: med.end_date.unwrap_or_default(),
            discontinue_reason: med.discontinue_reason,
        }),
        None => Err("Medication not found".to_string()),
    }
}

/// Reactivate a deprecated medication
pub fn reactivate_medication(db: &Database, id: i64) -> Result<MedicationDetail, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let updated = Medication::reactivate(&conn, id)
        .map_err(|e| format!("Failed to reactivate medication: {}", e))?;

    match updated {
        Some(med) => Ok(MedicationDetail::from(med)),
        None => Err(format!("Medication not found with id: {}", id)),
    }
}

/// Delete a medication (requires force flag)
pub fn delete_medication(
    db: &Database,
    id: i64,
    force: bool,
) -> Result<Result<DeleteMedicationSuccessResponse, DeleteMedicationBlockedResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if medication exists
    let existing = Medication::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Medication not found with id: {}", id));
    }

    // If not forced, block the delete
    if !force {
        return Ok(Err(DeleteMedicationBlockedResponse {
            error: "Medication deletion requires explicit confirmation (force=true). Consider deprecating instead to preserve history.".to_string(),
            requires_force: true,
        }));
    }

    // In the future, check for associations here
    // For now, just delete
    Medication::delete(&conn, id)
        .map_err(|e| format!("Failed to delete medication: {}", e))?;

    Ok(Ok(DeleteMedicationSuccessResponse {
        success: true,
        deleted_id: id,
    }))
}

/// Export medications to markdown document
pub fn export_medications_markdown(
    db: &Database,
    patient_name: &str,
) -> Result<ExportMedicationsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get all active medications
    let meds = Medication::list(&conn, true, None)
        .map_err(|e| format!("Failed to list medications: {}", e))?;

    let now = chrono::Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M:%S UTC").to_string();
    let generated_at = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let mut markdown = String::new();

    // Header
    markdown.push_str(&format!("# Medication List\n\n"));
    markdown.push_str(&format!("**Patient:** {}\n\n", patient_name));
    markdown.push_str(&format!("**Date:** {}\n\n", date_str));
    markdown.push_str(&format!("**Time:** {}\n\n", time_str));
    markdown.push_str("---\n\n");

    // Group medications by type, sorted by sort_order (prescriptions first)
    let mut grouped: std::collections::HashMap<MedType, Vec<&Medication>> = std::collections::HashMap::new();
    for med in &meds {
        grouped.entry(med.med_type).or_default().push(med);
    }

    // Sort by type order
    let mut types: Vec<MedType> = grouped.keys().cloned().collect();
    types.sort_by_key(|t| t.sort_order());

    for med_type in types {
        let type_meds = grouped.get(&med_type).unwrap();

        markdown.push_str(&format!("## {}\n\n", med_type.display_name()));

        for med in type_meds {
            markdown.push_str(&format!("### {}\n\n", med.name));
            markdown.push_str(&format!("- **Dosage:** {} {}\n", med.dosage_amount, med.dosage_unit.display_name()));

            if let Some(ref freq) = med.frequency {
                markdown.push_str(&format!("- **Frequency:** {}\n", freq));
            }

            if let Some(ref instructions) = med.instructions {
                markdown.push_str(&format!("- **Instructions:** {}\n", instructions));
            }

            if med.med_type == MedType::Prescription {
                if let Some(ref doctor) = med.prescribing_doctor {
                    markdown.push_str(&format!("- **Prescribing Doctor:** {}\n", doctor));
                }
                if let Some(ref date) = med.prescribed_date {
                    markdown.push_str(&format!("- **Date Prescribed:** {}\n", date));
                }
                if let Some(ref pharmacy) = med.pharmacy {
                    markdown.push_str(&format!("- **Pharmacy:** {}\n", pharmacy));
                }
                if let Some(ref rx) = med.rx_number {
                    markdown.push_str(&format!("- **Rx Number:** {}\n", rx));
                }
                if let Some(refills) = med.refills_remaining {
                    markdown.push_str(&format!("- **Refills Remaining:** {}\n", refills));
                }
            }

            if let Some(ref start) = med.start_date {
                markdown.push_str(&format!("- **Started:** {}\n", start));
            }

            if let Some(ref notes) = med.notes {
                markdown.push_str(&format!("- **Notes:** {}\n", notes));
            }

            markdown.push_str("\n");
        }
    }

    if meds.is_empty() {
        markdown.push_str("*No active medications on file.*\n\n");
    }

    markdown.push_str("---\n\n");
    markdown.push_str(&format!("*Generated: {}*\n", generated_at));

    Ok(ExportMedicationsResponse {
        markdown,
        medication_count: meds.len(),
        generated_at,
    })
}
