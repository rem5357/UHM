//! Sprint 1 Regression Tests
//!
//! Tests for the 5 bug fixes from the Codex code review.
//! Each test validates that the bug is fixed and would fail if reverted.

use uhm::db::Database;
use uhm::db::migrations::run_migrations;
use uhm::models::{
    Day, Exercise, ExerciseCreate, ExerciseSegment, ExerciseSegmentCreate, ExerciseType,
    FoodItem, FoodItemCreate, FoodItemUpdate, Preference,
    MealEntry, MealEntryCreate, MealType,
    Nutrition, calculate_direct_log_multiplier,
};

/// Create a temp database with all migrations applied.
/// Returns the Database handle and a TempDir that must stay alive for the DB's lifetime.
fn setup_test_db() -> (Database, tempfile::TempDir) {
    let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");
    let db = Database::new(&db_path).expect("Failed to create test database");

    // Run migrations
    let conn = db.get_conn().expect("Failed to get connection");
    run_migrations(&conn).expect("Failed to run migrations");

    (db, tmp_dir)
}

// ============================================================================
// Bug #1: Exercise deletion stale cache
// ============================================================================

#[test]
fn test_exercise_delete_recalculates_day_calories() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    // Create a day
    let day = Day::get_or_create(&conn, "2026-01-15").unwrap();

    // Insert a weight vital so calorie calculation works
    conn.execute(
        "INSERT INTO vitals (vital_type, value1, unit) VALUES ('weight', 310.0, 'lbs')",
        [],
    ).unwrap();

    // Create an exercise
    let exercise = Exercise::create(&conn, &ExerciseCreate {
        day_id: day.id,
        exercise_type: ExerciseType::Treadmill,
        timestamp: Some("2026-01-15T10:00:00".to_string()),
        pre_vital_group_id: None,
        post_vital_group_id: None,
        notes: None,
    }).unwrap();

    // Add a segment (which triggers calorie calculation)
    ExerciseSegment::create(&conn, &ExerciseSegmentCreate {
        exercise_id: exercise.id,
        duration_minutes: Some(30.0),
        speed_mph: Some(3.0),
        distance_miles: None,
        incline_percent: Some(0.0),
        avg_heart_rate: None,
        notes: None,
    }).unwrap();

    // Verify day has non-zero cached_calories_burned
    let day_before: f64 = conn.query_row(
        "SELECT cached_calories_burned FROM days WHERE id = ?1",
        [day.id],
        |row| row.get(0),
    ).unwrap();
    assert!(day_before > 0.0, "Day should have burned calories after exercise: {}", day_before);

    // Delete the exercise
    let deleted = Exercise::delete(&conn, exercise.id).unwrap();
    assert!(deleted);

    // Verify day's cached_calories_burned is now 0
    let day_after: f64 = conn.query_row(
        "SELECT cached_calories_burned FROM days WHERE id = ?1",
        [day.id],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(day_after, 0.0, "Day calories burned should be 0 after exercise deletion, got {}", day_after);
}

// ============================================================================
// Bug #2: update_food_item missing validation
// ============================================================================

fn make_food_item_create(name: &str, serving_size: f64, calories: f64) -> FoodItemCreate {
    FoodItemCreate {
        name: name.to_string(),
        brand: None,
        serving_size,
        serving_unit: "g".to_string(),
        calories,
        protein: 20.0,
        carbs: 0.0,
        fat: 3.0,
        fiber: 0.0,
        sodium: 50.0,
        sugar: 0.0,
        saturated_fat: 1.0,
        cholesterol: 50.0,
        preference: Preference::Neutral,
        notes: None,
        base_unit_type: None,
        grams_per_serving: None,
        ml_per_serving: None,
        source: None,
        source_detail: None,
        ww_zero_point: None,
        ww_source: None,
        ww_points_override: None,
        scoop_grams: None,
    }
}

#[test]
fn test_update_food_item_rejects_zero_serving_size() {
    let (db, _tmp) = setup_test_db();

    let item = uhm::tools::food_items::add_food_item(
        &db, make_food_item_create("Test Chicken", 100.0, 165.0),
    ).unwrap();

    // Try to update with serving_size: 0 — should fail
    let result = uhm::tools::food_items::update_food_item(&db, item.id, FoodItemUpdate {
        serving_size: Some(0.0),
        ..Default::default()
    });
    assert!(result.is_err(), "Update with serving_size=0 should fail");
    assert!(result.unwrap_err().contains("serving_size"));
}

#[test]
fn test_update_food_item_rejects_negative_calories() {
    let (db, _tmp) = setup_test_db();

    let item = uhm::tools::food_items::add_food_item(
        &db, make_food_item_create("Test Rice", 100.0, 130.0),
    ).unwrap();

    let result = uhm::tools::food_items::update_food_item(&db, item.id, FoodItemUpdate {
        calories: Some(-5.0),
        ..Default::default()
    });
    assert!(result.is_err(), "Update with negative calories should fail");
    assert!(result.unwrap_err().contains("calories"));
}

#[test]
fn test_update_food_item_rejects_empty_name() {
    let (db, _tmp) = setup_test_db();

    let item = uhm::tools::food_items::add_food_item(
        &db, make_food_item_create("Test Egg", 1.0, 70.0),
    ).unwrap();

    let result = uhm::tools::food_items::update_food_item(&db, item.id, FoodItemUpdate {
        name: Some("   ".to_string()),
        ..Default::default()
    });
    assert!(result.is_err(), "Update with empty name should fail");
    assert!(result.unwrap_err().contains("name"));
}

#[test]
fn test_update_food_item_valid_update_succeeds() {
    let (db, _tmp) = setup_test_db();

    let item = uhm::tools::food_items::add_food_item(
        &db, make_food_item_create("Test Salmon", 100.0, 208.0),
    ).unwrap();

    let result = uhm::tools::food_items::update_food_item(&db, item.id, FoodItemUpdate {
        calories: Some(210.0),
        protein: Some(22.0),
        ..Default::default()
    });
    assert!(result.is_ok(), "Valid update should succeed: {:?}", result.err());
}

// ============================================================================
// Bug #3: Divide-by-zero in direct meal multiplier
// ============================================================================

#[test]
fn test_direct_log_multiplier_zero_serving_size_no_nan() {
    // Create a FoodItem with serving_size = 0 (simulating bad data)
    let food_item = FoodItem {
        id: 999,
        name: "Bad Data Item".to_string(),
        brand: None,
        serving_size: 0.0,
        serving_unit: "g".to_string(),
        nutrition: Nutrition::zero(),
        preference: Preference::Neutral,
        notes: None,
        base_unit_type: None,
        grams_per_serving: None, // Falls back to serving_size (0.0)
        ml_per_serving: None,
        source: None,
        source_detail: None,
        ww_points: None,
        ww_zero_point: false,
        ww_source: None,
        scoop_grams: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    // Test with grams — would divide by 0 without the fix
    let multiplier_g = calculate_direct_log_multiplier(150.0, "g", &food_item);
    assert!(multiplier_g.is_finite(), "Multiplier should be finite, got {}", multiplier_g);
    assert_eq!(multiplier_g, 150.0, "Should fall back to quantity as multiplier");

    // Test with ml
    let multiplier_ml = calculate_direct_log_multiplier(240.0, "ml", &food_item);
    assert!(multiplier_ml.is_finite(), "ML multiplier should be finite, got {}", multiplier_ml);
    assert_eq!(multiplier_ml, 240.0);

    // Test with count
    let multiplier_count = calculate_direct_log_multiplier(2.0, "count", &food_item);
    assert!(multiplier_count.is_finite(), "Count multiplier should be finite, got {}", multiplier_count);
    assert_eq!(multiplier_count, 2.0);
}

#[test]
fn test_direct_log_multiplier_normal_case_still_works() {
    let food_item = FoodItem {
        id: 1,
        name: "Chicken Breast".to_string(),
        brand: None,
        serving_size: 100.0,
        serving_unit: "g".to_string(),
        nutrition: Nutrition::zero(),
        preference: Preference::Neutral,
        notes: None,
        base_unit_type: None,
        grams_per_serving: Some(100.0),
        ml_per_serving: None,
        source: None,
        source_detail: None,
        ww_points: None,
        ww_zero_point: false,
        ww_source: None,
        scoop_grams: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    let multiplier = calculate_direct_log_multiplier(150.0, "g", &food_item);
    assert!((multiplier - 1.5).abs() < 0.001, "150g / 100g should be 1.5, got {}", multiplier);
}

// ============================================================================
// Bug #5: update_meal_entry missing bounds validation
// ============================================================================

#[test]
fn test_update_meal_entry_rejects_negative_servings() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    let day = Day::get_or_create(&conn, "2026-01-20").unwrap();

    conn.execute(
        "INSERT INTO food_items (name, serving_size, serving_unit, calories, protein, carbs, fat) \
         VALUES ('Test Food', 100.0, 'g', 200.0, 20.0, 10.0, 5.0)",
        [],
    ).unwrap();
    let food_id: i64 = conn.query_row("SELECT last_insert_rowid()", [], |r| r.get(0)).unwrap();

    let entry = MealEntry::create(&conn, &MealEntryCreate {
        day_id: day.id,
        meal_type: MealType::Lunch,
        recipe_id: None,
        food_item_id: Some(food_id),
        servings: 1.0,
        percent_eaten: None,
        notes: None,
    }).unwrap();

    let result = uhm::tools::days::update_meal_entry(
        &db, entry.id, None, Some(-1.0), None, None,
    );
    assert!(result.is_err(), "Update with negative servings should fail");
    assert!(result.unwrap_err().contains("Servings"));
}

#[test]
fn test_update_meal_entry_rejects_percent_over_100() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    let day = Day::get_or_create(&conn, "2026-01-21").unwrap();

    conn.execute(
        "INSERT INTO food_items (name, serving_size, serving_unit, calories, protein, carbs, fat) \
         VALUES ('Test Food 2', 100.0, 'g', 150.0, 15.0, 8.0, 3.0)",
        [],
    ).unwrap();
    let food_id: i64 = conn.query_row("SELECT last_insert_rowid()", [], |r| r.get(0)).unwrap();

    let entry = MealEntry::create(&conn, &MealEntryCreate {
        day_id: day.id,
        meal_type: MealType::Dinner,
        recipe_id: None,
        food_item_id: Some(food_id),
        servings: 1.0,
        percent_eaten: None,
        notes: None,
    }).unwrap();

    let result = uhm::tools::days::update_meal_entry(
        &db, entry.id, None, None, Some(150.0), None,
    );
    assert!(result.is_err(), "Update with percent_eaten=150 should fail");
    assert!(result.unwrap_err().contains("percent_eaten"));
}

#[test]
fn test_update_meal_entry_valid_update_succeeds() {
    let (db, _tmp) = setup_test_db();
    let conn = db.get_conn().unwrap();

    let day = Day::get_or_create(&conn, "2026-01-22").unwrap();

    conn.execute(
        "INSERT INTO food_items (name, serving_size, serving_unit, calories, protein, carbs, fat) \
         VALUES ('Test Food 3', 100.0, 'g', 100.0, 10.0, 5.0, 2.0)",
        [],
    ).unwrap();
    let food_id: i64 = conn.query_row("SELECT last_insert_rowid()", [], |r| r.get(0)).unwrap();

    let entry = MealEntry::create(&conn, &MealEntryCreate {
        day_id: day.id,
        meal_type: MealType::Snack,
        recipe_id: None,
        food_item_id: Some(food_id),
        servings: 1.0,
        percent_eaten: None,
        notes: None,
    }).unwrap();

    let result = uhm::tools::days::update_meal_entry(
        &db, entry.id, None, Some(2.0), Some(50.0), None,
    );
    assert!(result.is_ok(), "Valid update should succeed: {:?}", result.err());
    let updated = result.unwrap().unwrap();
    assert_eq!(updated.servings, 2.0);
    assert_eq!(updated.percent_eaten, 50.0);
}
