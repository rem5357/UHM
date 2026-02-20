//! Sprint 2 Regression Tests
//!
//! Tests for CSV hardening, batch update atomicity, and vital group unlink fixes.

use uhm::db::Database;
use uhm::db::migrations::run_migrations;
use uhm::models::{Vital, VitalCreate, VitalType, VitalGroup, VitalGroupCreate};

/// Create a temp database with all migrations applied.
fn setup_test_db() -> (Database, tempfile::TempDir) {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");
    let db = Database::new(&db_path).expect("Failed to create test database");

    let conn = db.get_conn().expect("Failed to get connection");
    run_migrations(&conn).expect("Failed to run migrations");

    (db, tmp_dir)
}

// ============================================================================
// Item #8: Vital group unlink semantics — reject group_id=0
// ============================================================================

#[test]
fn test_update_vital_rejects_group_id_zero() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    // Create a vital
    let vital = Vital::create(&conn, &VitalCreate {
        vital_type: VitalType::Weight,
        timestamp: Some("2026-01-15T10:00:00".to_string()),
        value1: 310.0,
        value2: None,
        unit: Some("lbs".to_string()),
        group_id: None,
        notes: None,
    }).unwrap();

    // Try to update with group_id = 0 (old docs said this would unlink)
    let result = uhm::tools::vitals::update_vital(
        &db, vital.id,
        None, None, None, None,
        Some(0), // group_id = 0 should be rejected
        None,
    );
    assert!(result.is_err(), "group_id=0 should be rejected");
    assert!(result.unwrap_err().contains("group_id"), "Error should mention group_id");
}

#[test]
fn test_update_vital_rejects_negative_group_id() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    let vital = Vital::create(&conn, &VitalCreate {
        vital_type: VitalType::HeartRate,
        timestamp: Some("2026-01-15T10:05:00".to_string()),
        value1: 72.0,
        value2: None,
        unit: Some("bpm".to_string()),
        group_id: None,
        notes: None,
    }).unwrap();

    let result = uhm::tools::vitals::update_vital(
        &db, vital.id,
        None, None, None, None,
        Some(-1), // negative group_id should be rejected
        None,
    );
    assert!(result.is_err(), "Negative group_id should be rejected");
}

#[test]
fn test_update_vital_accepts_valid_group_id() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    // Create a group first
    let group = VitalGroup::create(&conn, &VitalGroupCreate {
        description: None,
        timestamp: None,
        notes: None,
    }).unwrap();

    let vital = Vital::create(&conn, &VitalCreate {
        vital_type: VitalType::Weight,
        timestamp: Some("2026-01-15T11:00:00".to_string()),
        value1: 308.0,
        value2: None,
        unit: Some("lbs".to_string()),
        group_id: None,
        notes: None,
    }).unwrap();

    let result = uhm::tools::vitals::update_vital(
        &db, vital.id,
        None, None, None, None,
        Some(group.id), // Valid group_id should work
        None,
    );
    assert!(result.is_ok(), "Valid group_id should succeed: {:?}", result.err());
}
