//! Database migrations
//!
//! Schema creation and migration logic.

use rusqlite::Connection;

use super::connection::DbResult;

/// Current schema version
const SCHEMA_VERSION: i32 = 7;

/// Run all migrations to bring the database up to the current schema version
pub fn run_migrations(conn: &Connection) -> DbResult<()> {
    // Create migrations table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    // Get current version
    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Run migrations
    if current_version < 1 {
        migrate_v1(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (1)", [])?;
    }

    if current_version < 2 {
        migrate_v2(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (2)", [])?;
    }

    if current_version < 3 {
        migrate_v3(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (3)", [])?;
    }

    if current_version < 4 {
        migrate_v4(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (4)", [])?;
    }

    if current_version < 5 {
        migrate_v5(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (5)", [])?;
    }

    if current_version < 6 {
        migrate_v6(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (6)", [])?;
    }

    if current_version < 7 {
        migrate_v7(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (7)", [])?;
    }

    Ok(())
}

/// Migration v1: Initial schema
fn migrate_v1(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- FOOD ITEMS
        -- Base nutritional data for ingredients
        -- ============================================
        CREATE TABLE food_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            brand TEXT,                          -- nullable, for branded products
            serving_size REAL NOT NULL,          -- e.g., 100.0
            serving_unit TEXT NOT NULL,          -- e.g., "g", "ml", "each"

            -- Nutritional values (per serving)
            calories REAL NOT NULL DEFAULT 0,
            protein REAL NOT NULL DEFAULT 0,     -- grams
            carbs REAL NOT NULL DEFAULT 0,       -- grams
            fat REAL NOT NULL DEFAULT 0,         -- grams
            fiber REAL NOT NULL DEFAULT 0,       -- grams
            sodium REAL NOT NULL DEFAULT 0,      -- milligrams
            sugar REAL NOT NULL DEFAULT 0,       -- grams
            saturated_fat REAL NOT NULL DEFAULT 0, -- grams
            cholesterol REAL NOT NULL DEFAULT 0, -- milligrams

            -- Metadata
            preference TEXT CHECK(preference IN ('liked', 'disliked', 'neutral')) DEFAULT 'neutral',
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_food_items_name ON food_items(name);
        CREATE INDEX idx_food_items_brand ON food_items(brand);

        -- ============================================
        -- RECIPES
        -- Collections of food items with quantities
        -- ============================================
        CREATE TABLE recipes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            servings_produced REAL NOT NULL DEFAULT 1.0,  -- "makes X servings"
            is_favorite INTEGER NOT NULL DEFAULT 0,       -- boolean

            -- Cached nutrition (per serving) - recalculated when ingredients change
            cached_calories REAL DEFAULT 0,
            cached_protein REAL DEFAULT 0,
            cached_carbs REAL DEFAULT 0,
            cached_fat REAL DEFAULT 0,
            cached_fiber REAL DEFAULT 0,
            cached_sodium REAL DEFAULT 0,
            cached_sugar REAL DEFAULT 0,
            cached_saturated_fat REAL DEFAULT 0,
            cached_cholesterol REAL DEFAULT 0,

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_recipes_name ON recipes(name);
        CREATE INDEX idx_recipes_favorite ON recipes(is_favorite);

        -- ============================================
        -- RECIPE INGREDIENTS
        -- Junction table: which food items in which recipes
        -- ============================================
        CREATE TABLE recipe_ingredients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
            food_item_id INTEGER NOT NULL REFERENCES food_items(id) ON DELETE RESTRICT,
            quantity REAL NOT NULL,              -- amount used
            unit TEXT NOT NULL,                  -- unit of quantity (may differ from food_item's serving_unit)

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),

            UNIQUE(recipe_id, food_item_id)      -- one entry per food item per recipe
        );

        CREATE INDEX idx_recipe_ingredients_recipe ON recipe_ingredients(recipe_id);
        CREATE INDEX idx_recipe_ingredients_food ON recipe_ingredients(food_item_id);

        -- ============================================
        -- DAYS
        -- Daily aggregation container
        -- ============================================
        CREATE TABLE days (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL UNIQUE,           -- ISO date: "2025-01-09"

            -- Cached daily totals - recalculated when meal entries change
            cached_calories REAL DEFAULT 0,
            cached_protein REAL DEFAULT 0,
            cached_carbs REAL DEFAULT 0,
            cached_fat REAL DEFAULT 0,
            cached_fiber REAL DEFAULT 0,
            cached_sodium REAL DEFAULT 0,
            cached_sugar REAL DEFAULT 0,
            cached_saturated_fat REAL DEFAULT 0,
            cached_cholesterol REAL DEFAULT 0,

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE UNIQUE INDEX idx_days_date ON days(date);

        -- ============================================
        -- MEAL ENTRIES
        -- What was actually consumed
        -- ============================================
        CREATE TABLE meal_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            day_id INTEGER NOT NULL REFERENCES days(id) ON DELETE CASCADE,
            meal_type TEXT NOT NULL CHECK(meal_type IN ('breakfast', 'lunch', 'dinner', 'snack', 'unspecified')),

            -- Source: either a recipe OR a direct food item (one must be set, not both)
            recipe_id INTEGER REFERENCES recipes(id) ON DELETE RESTRICT,
            food_item_id INTEGER REFERENCES food_items(id) ON DELETE RESTRICT,

            servings REAL NOT NULL DEFAULT 1.0,  -- how many servings consumed
            percent_eaten REAL NOT NULL DEFAULT 100.0, -- for partial consumption (0-100)

            -- Cached actual nutrition consumed - calculated from source × servings × percent
            cached_calories REAL DEFAULT 0,
            cached_protein REAL DEFAULT 0,
            cached_carbs REAL DEFAULT 0,
            cached_fat REAL DEFAULT 0,
            cached_fiber REAL DEFAULT 0,
            cached_sodium REAL DEFAULT 0,
            cached_sugar REAL DEFAULT 0,
            cached_saturated_fat REAL DEFAULT 0,
            cached_cholesterol REAL DEFAULT 0,

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),

            -- Constraint: must have exactly one source
            CHECK ((recipe_id IS NOT NULL AND food_item_id IS NULL) OR
                   (recipe_id IS NULL AND food_item_id IS NOT NULL))
        );

        CREATE INDEX idx_meal_entries_day ON meal_entries(day_id);
        CREATE INDEX idx_meal_entries_type ON meal_entries(meal_type);
        CREATE INDEX idx_meal_entries_recipe ON meal_entries(recipe_id);
        CREATE INDEX idx_meal_entries_food ON meal_entries(food_item_id);

        -- ============================================
        -- VITALS
        -- Health measurements
        -- ============================================
        CREATE TABLE vitals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vital_type TEXT NOT NULL CHECK(vital_type IN ('weight', 'blood_pressure', 'heart_rate', 'oxygen_saturation', 'glucose')),
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),

            -- Values (interpretation depends on vital_type)
            -- weight: value1 = weight, value2 = null
            -- blood_pressure: value1 = systolic, value2 = diastolic
            -- heart_rate: value1 = bpm, value2 = null
            -- oxygen_saturation: value1 = percentage, value2 = null
            -- glucose: value1 = mg/dL, value2 = null
            value1 REAL NOT NULL,
            value2 REAL,                         -- only used for blood_pressure
            unit TEXT NOT NULL,                  -- "lbs", "kg", "mmHg", "bpm", "%", "mg/dL"

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_vitals_type ON vitals(vital_type);
        CREATE INDEX idx_vitals_timestamp ON vitals(timestamp);
        "#,
    )?;

    Ok(())
}

/// Migration v2: Recipe components (recipes using other recipes)
fn migrate_v2(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- RECIPE COMPONENTS
        -- Allows recipes to use other recipes as ingredients
        -- ============================================
        CREATE TABLE recipe_components (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
            component_recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE RESTRICT,
            servings REAL NOT NULL DEFAULT 1.0,  -- how many servings of component recipe

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),

            -- Constraints
            UNIQUE(recipe_id, component_recipe_id),
            CHECK(recipe_id != component_recipe_id)  -- can't use itself as component
        );

        CREATE INDEX idx_recipe_components_recipe ON recipe_components(recipe_id);
        CREATE INDEX idx_recipe_components_component ON recipe_components(component_recipe_id);
        "#,
    )?;

    Ok(())
}

/// Migration v3: Medications tracking
fn migrate_v3(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- MEDICATIONS
        -- Tracks prescriptions, supplements, OTC, and other medications
        -- ============================================
        CREATE TABLE medications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,                   -- e.g., "Lisinopril", "Vitamin D3"

            -- Type of medication
            med_type TEXT NOT NULL CHECK(med_type IN (
                'prescription',
                'supplement',
                'otc',
                'natural',
                'compound',
                'medical_device',
                'other'
            )),

            -- Dosage information
            dosage_amount REAL NOT NULL,          -- e.g., 10.0
            dosage_unit TEXT NOT NULL CHECK(dosage_unit IN (
                'mg',
                'mcg',
                'g',
                'ml',
                'fl_oz',
                'pill',
                'tablet',
                'capsule',
                'spray',
                'drop',
                'patch',
                'injection',
                'unit',
                'iu',
                'puff',
                'other'
            )),

            -- Instructions and usage
            instructions TEXT,                    -- e.g., "Take 1 tablet daily with food"
            frequency TEXT,                       -- e.g., "twice daily", "PRN", "weekly"

            -- Prescription-specific fields
            prescribing_doctor TEXT,              -- Doctor's name (null for non-rx)
            prescribed_date TEXT,                 -- Date prescribed (ISO format)
            pharmacy TEXT,                        -- Pharmacy name
            rx_number TEXT,                       -- Prescription number
            refills_remaining INTEGER,            -- Number of refills left

            -- Status tracking
            is_active INTEGER NOT NULL DEFAULT 1, -- 1 = active, 0 = deprecated/inactive
            start_date TEXT,                      -- When started taking
            end_date TEXT,                        -- When stopped (if applicable)
            discontinue_reason TEXT,              -- Why discontinued

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_medications_name ON medications(name);
        CREATE INDEX idx_medications_type ON medications(med_type);
        CREATE INDEX idx_medications_active ON medications(is_active);
        "#,
    )?;

    Ok(())
}

/// Migration v4: Vital groups for linking related readings
fn migrate_v4(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- VITAL GROUPS
        -- Links related vital readings together
        -- (e.g., BP + HR taken at the same time)
        -- ============================================
        CREATE TABLE vital_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT,                     -- e.g., "BP & HR reading", "Post Exercise"
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_vital_groups_timestamp ON vital_groups(timestamp);

        -- Add group_id to vitals table (nullable for standalone readings)
        ALTER TABLE vitals ADD COLUMN group_id INTEGER REFERENCES vital_groups(id);

        CREATE INDEX idx_vitals_group ON vitals(group_id);
        "#,
    )?;

    Ok(())
}

/// Migration v5: Unit conversion support for food items
fn migrate_v5(conn: &Connection) -> DbResult<()> {
    // Add new columns for unit conversion
    conn.execute_batch(
        r#"
        -- ============================================
        -- UNIT CONVERSION SUPPORT
        -- Add fields for standardized unit handling
        -- ============================================

        -- Base unit type: weight (g), volume (ml), or count (each)
        ALTER TABLE food_items ADD COLUMN base_unit_type TEXT
            CHECK(base_unit_type IN ('weight', 'volume', 'count'));

        -- Grams per serving (for weight-based and count items with known weight)
        ALTER TABLE food_items ADD COLUMN grams_per_serving REAL;

        -- Milliliters per serving (for volume-based items)
        ALTER TABLE food_items ADD COLUMN ml_per_serving REAL;

        -- ============================================
        -- FOOD ITEM CONVERSIONS
        -- Custom unit conversions per food item
        -- (e.g., "scoop" = 31g for protein powder)
        -- ============================================
        CREATE TABLE food_item_conversions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            food_item_id INTEGER NOT NULL REFERENCES food_items(id) ON DELETE CASCADE,
            from_unit TEXT NOT NULL,           -- 'scoop', 'slice', 'piece', etc.
            to_grams REAL,                     -- how many grams this equals
            to_ml REAL,                        -- how many ml this equals
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(food_item_id, from_unit)
        );

        CREATE INDEX idx_food_conversions_food ON food_item_conversions(food_item_id);
        "#,
    )?;

    // Migrate existing food items to populate new fields
    migrate_existing_food_items(conn)?;

    Ok(())
}

/// Migrate existing food items to populate base_unit_type, grams_per_serving, ml_per_serving
fn migrate_existing_food_items(conn: &Connection) -> DbResult<()> {
    use crate::nutrition::{
        calculate_grams_per_serving, calculate_ml_per_serving, infer_base_unit_type,
    };

    // Fetch all existing food items
    let mut stmt = conn.prepare("SELECT id, serving_size, serving_unit FROM food_items")?;
    let items: Vec<(i64, f64, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Update each food item with inferred values
    let mut update_stmt = conn.prepare(
        "UPDATE food_items SET base_unit_type = ?1, grams_per_serving = ?2, ml_per_serving = ?3 WHERE id = ?4"
    )?;

    for (id, serving_size, serving_unit) in items {
        let base_type = infer_base_unit_type(&serving_unit);
        let grams = calculate_grams_per_serving(serving_size, &serving_unit);
        let ml = calculate_ml_per_serving(serving_size, &serving_unit);

        update_stmt.execute(rusqlite::params![
            base_type.to_db_str(),
            grams,
            ml,
            id
        ])?;
    }

    Ok(())
}

/// Migration v6: Exercise tracking
fn migrate_v6(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- EXERCISES
        -- Workout sessions (e.g., treadmill session)
        -- ============================================
        CREATE TABLE exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            day_id INTEGER NOT NULL REFERENCES days(id) ON DELETE CASCADE,
            exercise_type TEXT NOT NULL CHECK(exercise_type IN ('treadmill')),
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),

            -- Cached totals (recalculated when segments change)
            cached_duration_minutes REAL NOT NULL DEFAULT 0,
            cached_distance_miles REAL NOT NULL DEFAULT 0,
            cached_calories_burned REAL NOT NULL DEFAULT 0,

            -- Link to vital groups for PRE/POST readings
            pre_vital_group_id INTEGER REFERENCES vital_groups(id),
            post_vital_group_id INTEGER REFERENCES vital_groups(id),

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_exercises_day ON exercises(day_id);
        CREATE INDEX idx_exercises_type ON exercises(exercise_type);
        CREATE INDEX idx_exercises_timestamp ON exercises(timestamp);

        -- ============================================
        -- EXERCISE SEGMENTS
        -- Individual segments within a workout
        -- (e.g., 15 min at 2.3 mph, then 15 min at 2.5 mph)
        -- ============================================
        CREATE TABLE exercise_segments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            exercise_id INTEGER NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
            segment_order INTEGER NOT NULL DEFAULT 1,

            -- Treadmill metrics (2 of 3 required: duration, speed, distance)
            duration_minutes REAL,          -- time in minutes
            speed_mph REAL,                 -- speed in mph
            distance_miles REAL,            -- distance in miles
            incline_percent REAL NOT NULL DEFAULT 0,

            -- Calculation metadata
            calculated_field TEXT CHECK(calculated_field IN ('duration', 'speed', 'distance', 'none')),
            is_consistent INTEGER NOT NULL DEFAULT 1,  -- 1 if values match, 0 if inconsistent

            -- Calculated values
            calories_burned REAL NOT NULL DEFAULT 0,
            weight_used_lbs REAL,           -- weight used for calorie calculation

            -- Optional metrics
            avg_heart_rate REAL,

            -- Metadata
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_exercise_segments_exercise ON exercise_segments(exercise_id);
        CREATE INDEX idx_exercise_segments_order ON exercise_segments(exercise_id, segment_order);

        -- ============================================
        -- Add cached exercise calories to days
        -- ============================================
        ALTER TABLE days ADD COLUMN cached_calories_burned REAL NOT NULL DEFAULT 0;
        "#,
    )?;

    Ok(())
}

/// Migration v7: Patient info for reports
fn migrate_v7(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- PATIENT INFO
        -- Single-row table for report headers
        -- ============================================
        CREATE TABLE patient_info (
            id INTEGER PRIMARY KEY CHECK (id = 1),  -- Single row only
            name TEXT NOT NULL,
            dob TEXT NOT NULL,  -- ISO format YYYY-MM-DD
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )?;

    Ok(())
}

/// Get the current schema version
pub fn get_schema_version(conn: &Connection) -> DbResult<i32> {
    let version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(version)
}

/// Check if the database needs migration
pub fn needs_migration(conn: &Connection) -> DbResult<bool> {
    let current = get_schema_version(conn)?;
    Ok(current < SCHEMA_VERSION)
}
