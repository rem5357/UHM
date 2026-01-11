//! Database migrations
//!
//! Schema creation and migration logic.

use rusqlite::Connection;

use super::connection::DbResult;

/// Current schema version
const SCHEMA_VERSION: i32 = 1;

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
