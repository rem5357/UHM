//! Database migrations
//!
//! Schema creation and migration logic.

use rusqlite::Connection;

use super::connection::DbResult;

/// Current schema version
const SCHEMA_VERSION: i32 = 12;

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

    if current_version < 8 {
        migrate_v8(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (8)", [])?;
    }

    if current_version < 9 {
        migrate_v9(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (9)", [])?;
    }

    if current_version < 10 {
        migrate_v10(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (10)", [])?;
    }

    if current_version < 11 {
        migrate_v11(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (11)", [])?;
    }

    if current_version < 12 {
        migrate_v12(conn)?;
        conn.execute("INSERT INTO schema_migrations (version) VALUES (12)", [])?;
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

/// Migration v8: Direct meal logging with quantity/unit
fn migrate_v8(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- DIRECT MEAL LOGGING
        -- Add quantity and unit columns for recipe-free workflow
        -- ============================================
        ALTER TABLE meal_entries ADD COLUMN quantity REAL;
        ALTER TABLE meal_entries ADD COLUMN unit TEXT;
        "#,
    )?;

    Ok(())
}

/// Migration v9: FTS5 full-text search for food items
fn migrate_v9(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- FTS5 FULL-TEXT SEARCH
        -- Enables tokenized search for food items
        -- Handles word-order independence and prefix matching
        -- ============================================

        -- Create FTS5 virtual table for food items
        CREATE VIRTUAL TABLE IF NOT EXISTS food_items_fts USING fts5(
            name,
            brand,
            content='food_items',
            content_rowid='id'
        );

        -- Populate from existing data
        INSERT INTO food_items_fts(rowid, name, brand)
        SELECT id, name, COALESCE(brand, '') FROM food_items;

        -- Trigger: keep FTS in sync on INSERT
        CREATE TRIGGER food_items_fts_ai AFTER INSERT ON food_items BEGIN
            INSERT INTO food_items_fts(rowid, name, brand)
            VALUES (new.id, new.name, COALESCE(new.brand, ''));
        END;

        -- Trigger: keep FTS in sync on DELETE
        CREATE TRIGGER food_items_fts_ad AFTER DELETE ON food_items BEGIN
            INSERT INTO food_items_fts(food_items_fts, rowid, name, brand)
            VALUES('delete', old.id, old.name, COALESCE(old.brand, ''));
        END;

        -- Trigger: keep FTS in sync on UPDATE
        CREATE TRIGGER food_items_fts_au AFTER UPDATE ON food_items BEGIN
            INSERT INTO food_items_fts(food_items_fts, rowid, name, brand)
            VALUES('delete', old.id, old.name, COALESCE(old.brand, ''));
            INSERT INTO food_items_fts(rowid, name, brand)
            VALUES (new.id, new.name, COALESCE(new.brand, ''));
        END;
        "#,
    )?;

    Ok(())
}

/// Migration v10: Source tracking columns + FTS5 rebuild with unified search_text
fn migrate_v10(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- SOURCE TRACKING
        -- Track provenance of food item nutritional data
        -- ============================================
        ALTER TABLE food_items ADD COLUMN source TEXT;
        ALTER TABLE food_items ADD COLUMN source_detail TEXT;

        -- ============================================
        -- FTS5 REBUILD: Unified search_text column
        -- Fixes cross-column queries like "Willa's oat milk"
        -- (brand="Willa's" + name="Oat Milk Barista")
        -- ============================================

        -- Drop existing triggers
        DROP TRIGGER IF EXISTS food_items_fts_ai;
        DROP TRIGGER IF EXISTS food_items_fts_ad;
        DROP TRIGGER IF EXISTS food_items_fts_au;

        -- Drop existing FTS table
        DROP TABLE IF EXISTS food_items_fts;

        -- Recreate with single search_text column
        CREATE VIRTUAL TABLE food_items_fts USING fts5(
            search_text,
            content='food_items',
            content_rowid='id'
        );

        -- Populate from existing data
        INSERT INTO food_items_fts(rowid, search_text)
        SELECT id, COALESCE(brand, '') || ' ' || name FROM food_items;

        -- Trigger: keep FTS in sync on INSERT
        CREATE TRIGGER food_items_fts_ai AFTER INSERT ON food_items BEGIN
            INSERT INTO food_items_fts(rowid, search_text)
            VALUES (new.id, COALESCE(new.brand, '') || ' ' || new.name);
        END;

        -- Trigger: keep FTS in sync on DELETE
        CREATE TRIGGER food_items_fts_ad AFTER DELETE ON food_items BEGIN
            INSERT INTO food_items_fts(food_items_fts, rowid, search_text)
            VALUES('delete', old.id, COALESCE(old.brand, '') || ' ' || old.name);
        END;

        -- Trigger: keep FTS in sync on UPDATE
        CREATE TRIGGER food_items_fts_au AFTER UPDATE ON food_items BEGIN
            INSERT INTO food_items_fts(food_items_fts, rowid, search_text)
            VALUES('delete', old.id, COALESCE(old.brand, '') || ' ' || old.name);
            INSERT INTO food_items_fts(rowid, search_text)
            VALUES (new.id, COALESCE(new.brand, '') || ' ' || new.name);
        END;
        "#,
    )?;

    Ok(())
}

/// Migration v11: Pill organizer fields for medications
fn migrate_v11(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- PILL ORGANIZER SUPPORT
        -- Add physical description and schedule slot
        -- for pill organizer report generation
        -- ============================================
        ALTER TABLE medications ADD COLUMN pill_description TEXT;
        ALTER TABLE medications ADD COLUMN schedule_slot TEXT;
        "#,
    )?;

    Ok(())
}

/// Migration v12: Weight Watchers points tracking
fn migrate_v12(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        -- ============================================
        -- WEIGHT WATCHERS POINTS SUPPORT
        -- Add WW points to food_items, meal_entries,
        -- exercises, and days tables
        -- ============================================

        -- Food items: WW points per serving and ZeroPoint flag
        ALTER TABLE food_items ADD COLUMN ww_points REAL;
        ALTER TABLE food_items ADD COLUMN ww_zero_point INTEGER NOT NULL DEFAULT 0;

        -- Meal entries: cached WW points for this entry
        ALTER TABLE meal_entries ADD COLUMN cached_ww_points REAL NOT NULL DEFAULT 0;

        -- Exercises: WW activity points earned
        ALTER TABLE exercises ADD COLUMN ww_activity_points REAL NOT NULL DEFAULT 0;

        -- Days: WW daily totals
        ALTER TABLE days ADD COLUMN cached_ww_points_gross REAL NOT NULL DEFAULT 0;
        ALTER TABLE days ADD COLUMN cached_ww_exercise_credit REAL NOT NULL DEFAULT 0;
        ALTER TABLE days ADD COLUMN cached_ww_points_net REAL NOT NULL DEFAULT 0;
        "#,
    )?;

    // Backfill WW points for all existing food items
    // Formula: points = (cal * 0.0305) + (sat_fat * 0.275) + (sugar * 0.12) - (protein * 0.098)
    // Rounded to nearest integer, floor at 0
    conn.execute_batch(
        r#"
        UPDATE food_items SET ww_points = MAX(0, ROUND(
            (calories * 0.0305) + (saturated_fat * 0.275) + (sugar * 0.12) - (protein * 0.098)
        ));
        "#,
    )?;

    // Flag ZeroPoint foods based on name patterns
    // Categories: skinless poultry, eggs, fish/shellfish, non-starchy vegetables,
    // potatoes, fruits, beans/legumes, non-fat yogurt, tofu
    conn.execute_batch(
        r#"
        UPDATE food_items SET ww_zero_point = 1, ww_points = 0
        WHERE
            -- Poultry (skinless/boneless/ground)
            (LOWER(name) LIKE '%chicken breast%' OR LOWER(name) LIKE '%turkey breast%'
             OR LOWER(name) LIKE '%skinless chicken%' OR LOWER(name) LIKE '%skinless turkey%'
             OR (LOWER(name) LIKE '%chicken%' AND LOWER(name) LIKE '%skinless%')
             OR (LOWER(name) LIKE '%chicken%' AND LOWER(name) LIKE '%boneless%' AND LOWER(name) NOT LIKE '%skin-on%')
             OR (LOWER(name) LIKE '%turkey%' AND LOWER(name) LIKE '%boneless%' AND LOWER(name) NOT LIKE '%skin-on%')
             OR LOWER(name) LIKE '%ground turkey%' OR LOWER(name) LIKE '%ground chicken%')
            -- Eggs
            OR (LOWER(name) LIKE '%egg%' AND LOWER(name) NOT LIKE '%eggplant%'
                AND LOWER(name) NOT LIKE '%egg roll%' AND LOWER(name) NOT LIKE '%egg noodle%')
            -- Fish and shellfish
            OR LOWER(name) LIKE '%salmon%' OR LOWER(name) LIKE '%tuna%'
            OR LOWER(name) LIKE '%tilapia%' OR LOWER(name) LIKE '%cod %'
            OR LOWER(name) LIKE '%shrimp%' OR LOWER(name) LIKE '%crab%'
            OR LOWER(name) LIKE '%lobster%' OR LOWER(name) LIKE '%scallop%'
            OR LOWER(name) LIKE '%halibut%' OR LOWER(name) LIKE '%trout%'
            OR LOWER(name) LIKE '%sardine%' OR LOWER(name) LIKE '%mahi%'
            OR LOWER(name) LIKE '%swordfish%' OR LOWER(name) LIKE '%bass %'
            -- Beans and legumes
            OR LOWER(name) LIKE '%black bean%' OR LOWER(name) LIKE '%kidney bean%'
            OR LOWER(name) LIKE '%pinto bean%' OR LOWER(name) LIKE '%chickpea%'
            OR LOWER(name) LIKE '%lentil%' OR LOWER(name) LIKE '%navy bean%'
            OR LOWER(name) LIKE '%garbanzo%' OR LOWER(name) LIKE '%cannellini%'
            -- Tofu
            OR LOWER(name) LIKE '%tofu%'
            -- Non-fat yogurt
            OR (LOWER(name) LIKE '%yogurt%' AND (LOWER(name) LIKE '%nonfat%'
                OR LOWER(name) LIKE '%non-fat%' OR LOWER(name) LIKE '%fat free%'
                OR LOWER(name) LIKE '%0% fat%' OR LOWER(name) LIKE '%fat-free%'))
            -- Fruits (common)
            OR LOWER(name) LIKE '%apple%' OR LOWER(name) LIKE '%banana%'
            OR LOWER(name) LIKE '%orange%' OR LOWER(name) LIKE '%strawberr%'
            OR LOWER(name) LIKE '%blueberr%' OR LOWER(name) LIKE '%raspberr%'
            OR LOWER(name) LIKE '%grape%' OR LOWER(name) LIKE '%watermelon%'
            OR LOWER(name) LIKE '%peach%' OR LOWER(name) LIKE '%pear %'
            OR LOWER(name) LIKE '%mango%' OR LOWER(name) LIKE '%pineapple%'
            OR LOWER(name) LIKE '%kiwi%' OR LOWER(name) LIKE '%plum%'
            OR LOWER(name) LIKE '%cherry%' OR LOWER(name) LIKE '%cantaloupe%'
            -- Vegetables (non-starchy + potatoes which WW includes)
            OR LOWER(name) LIKE '%broccoli%' OR LOWER(name) LIKE '%spinach%'
            OR LOWER(name) LIKE '%kale%' OR LOWER(name) LIKE '%lettuce%'
            OR LOWER(name) LIKE '%carrot%' OR LOWER(name) LIKE '%tomato%'
            OR LOWER(name) LIKE '%cucumber%' OR LOWER(name) LIKE '%pepper%'
            OR LOWER(name) LIKE '%onion%' OR LOWER(name) LIKE '%mushroom%'
            OR LOWER(name) LIKE '%zucchini%' OR LOWER(name) LIKE '%celery%'
            OR LOWER(name) LIKE '%cauliflower%' OR LOWER(name) LIKE '%asparagus%'
            OR LOWER(name) LIKE '%green bean%' OR LOWER(name) LIKE '%potato%'
            OR LOWER(name) LIKE '%sweet potato%' OR LOWER(name) LIKE '%corn %'
            OR LOWER(name) LIKE '%peas%' OR LOWER(name) LIKE '%cabbage%'
            -- Frozen vegetable blends
            OR LOWER(name) LIKE '%normandy%' OR LOWER(name) LIKE '%steamfresh%'
            OR LOWER(name) LIKE '%blend vegetables%' OR LOWER(name) LIKE '%vegetables%blend%'
            OR LOWER(name) LIKE '%mixed vegetables%' OR LOWER(name) LIKE '%stir fry vegetables%'
            -- Broths and stocks (near-zero calorie)
            OR LOWER(name) LIKE '%chicken broth%' OR LOWER(name) LIKE '%beef broth%'
            OR LOWER(name) LIKE '%cooking stock%'
        ;
        "#,
    )?;

    // Backfill meal entry WW points for March 2026 forward
    // For each meal entry: if food item is zero_point → 0, else scale ww_points by servings * percent/100
    conn.execute(
        r#"
        UPDATE meal_entries SET cached_ww_points = CASE
            WHEN food_item_id IS NOT NULL AND EXISTS (
                SELECT 1 FROM food_items fi WHERE fi.id = meal_entries.food_item_id AND fi.ww_zero_point = 1
            ) THEN 0
            WHEN food_item_id IS NOT NULL THEN
                COALESCE((SELECT fi.ww_points FROM food_items fi WHERE fi.id = meal_entries.food_item_id), 0)
                * servings * (percent_eaten / 100.0)
            WHEN recipe_id IS NOT NULL THEN
                -- For recipes, sum the WW points of ingredients (approximation: use cached calories formula)
                MAX(0, ROUND(
                    (cached_calories * 0.0305) + (cached_saturated_fat * 0.275)
                    + (cached_sugar * 0.12) - (cached_protein * 0.098)
                ))
            ELSE 0
        END
        WHERE day_id IN (SELECT id FROM days WHERE date >= '2026-03-01')
        "#,
        [],
    )?;

    // Backfill day WW totals for March 2026 forward
    conn.execute(
        r#"
        UPDATE days SET
            cached_ww_points_gross = COALESCE((
                SELECT SUM(cached_ww_points) FROM meal_entries WHERE meal_entries.day_id = days.id
            ), 0),
            cached_ww_exercise_credit = COALESCE((
                SELECT SUM(ww_activity_points) FROM exercises WHERE exercises.day_id = days.id
            ), 0)
        WHERE date >= '2026-03-01'
        "#,
        [],
    )?;

    conn.execute(
        r#"
        UPDATE days SET cached_ww_points_net = cached_ww_points_gross - cached_ww_exercise_credit
        WHERE date >= '2026-03-01'
        "#,
        [],
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
