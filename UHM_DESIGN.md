# Universal Health Manager (UHM) - Design Document

**Version**: 1.0.0-phase1  
**Author**: Robert (with Claude assistance)  
**Target**: Claude Code implementation  
**Location**: `D:\Projects\UHM`

---

## Overview

UHM is a Rust-based MCP (Model Context Protocol) server that provides persistent health and nutrition tracking via SQLite. It serves as the backend for logging meals, recipes, food items, and health vitals through Claude Desktop.

### Goals

1. **Reduce context consumption** - Move food/health logging from conversation memory to persistent storage
2. **Structured nutrition tracking** - Calculate and aggregate nutritional data automatically
3. **Flexible meal logging** - Support recipes, individual food items, and partial consumption
4. **Health vitals** - Track weight, blood pressure, heart rate, oxygen saturation, glucose

### Non-Goals (Phase 1)

- Web interface
- Mobile app
- Medication tracking (Phase 2)
- Lab results (Phase 2)
- Barcode scanning / external API integration

---

## Architecture

```
D:\Projects\UHM\
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs                 # Entry point, MCP server initialization
│   ├── db/
│   │   ├── mod.rs
│   │   ├── connection.rs       # SQLite connection pool/management
│   │   └── migrations.rs       # Schema creation and migrations
│   ├── models/
│   │   ├── mod.rs
│   │   ├── food_item.rs
│   │   ├── recipe.rs
│   │   ├── recipe_ingredient.rs
│   │   ├── meal_entry.rs
│   │   ├── day.rs
│   │   └── vital.rs
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── food_items.rs       # Food item CRUD operations
│   │   ├── recipes.rs          # Recipe management
│   │   ├── meals.rs            # Meal entry logging
│   │   ├── days.rs             # Day summaries and queries
│   │   └── vitals.rs           # Health vitals logging
│   ├── nutrition/
│   │   ├── mod.rs
│   │   ├── calculator.rs       # Nutrition aggregation logic
│   │   └── units.rs            # Unit conversion utilities
│   └── mcp/
│       ├── mod.rs
│       └── server.rs           # MCP protocol handler (stdio)
└── data/
    └── uhm.db                  # SQLite database (created on first run)
```

### Dependencies (Cargo.toml)

```toml
[package]
name = "uhm"
version = "0.1.0"
edition = "2021"

[dependencies]
# MCP
mcp-server = "0.2"              # Or latest MCP Rust SDK
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Database
rusqlite = { version = "0.31", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Database Schema

### Core Tables

```sql
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
```

---

## Unit Conversion

### Supported Units

```rust
pub enum VolumeUnit {
    Milliliter,    // ml (base)
    Liter,         // l
    FluidOunce,    // fl_oz
    Cup,           // cup
    Tablespoon,    // tbsp
    Teaspoon,      // tsp
    Pint,          // pint
    Quart,         // quart
    Gallon,        // gallon
}

pub enum WeightUnit {
    Gram,          // g (base)
    Kilogram,      // kg
    Milligram,     // mg
    Ounce,         // oz
    Pound,         // lb
}

pub enum CountUnit {
    Each,          // each
    Slice,         // slice
    Piece,         // piece
}
```

### Conversion Factors (to base unit)

```rust
// Volume → Milliliters
const ML_PER_L: f64 = 1000.0;
const ML_PER_FL_OZ: f64 = 29.5735;
const ML_PER_CUP: f64 = 236.588;
const ML_PER_TBSP: f64 = 14.7868;
const ML_PER_TSP: f64 = 4.92892;
const ML_PER_PINT: f64 = 473.176;
const ML_PER_QUART: f64 = 946.353;
const ML_PER_GALLON: f64 = 3785.41;

// Weight → Grams
const G_PER_KG: f64 = 1000.0;
const G_PER_MG: f64 = 0.001;
const G_PER_OZ: f64 = 28.3495;
const G_PER_LB: f64 = 453.592;
```

### Conversion Logic

When calculating nutrition for a recipe ingredient:

1. Get the food_item's `serving_size` and `serving_unit`
2. Get the recipe_ingredient's `quantity` and `unit`
3. Convert both to the same base unit (ml for volume, g for weight)
4. Calculate: `nutrition_multiplier = ingredient_quantity_base / serving_size_base`
5. Apply multiplier to all nutritional values

**Note**: Cross-type conversion (volume ↔ weight) is not supported without density data. If units are incompatible, return an error prompting the user to use matching unit types.

---

## MCP Tools Specification

### Tool Categories & Permissions

| Table | Create | Read | Update | Delete |
|-------|--------|------|--------|--------|
| food_items | ✅ | ✅ | ⚠️ if unused | ❌ |
| recipes | ✅ | ✅ | ⚠️ if not in meal_entries | ❌ |
| recipe_ingredients | ✅ | ✅ | ✅ | ✅ |
| meal_entries | ✅ | ✅ | ✅ | ✅ |
| days | ✅ auto | ✅ | ✅ notes only | ❌ |
| vitals | ✅ | ✅ | ✅ | ✅ |

### Food Item Tools

#### `add_food_item`

Creates a new food item with nutritional data.

**Parameters:**
```json
{
    "name": "string, required",
    "brand": "string, optional",
    "serving_size": "number, required",
    "serving_unit": "string, required (g, ml, each, etc.)",
    "calories": "number, required",
    "protein": "number, required",
    "carbs": "number, required",
    "fat": "number, required",
    "fiber": "number, default 0",
    "sodium": "number, default 0",
    "sugar": "number, default 0",
    "saturated_fat": "number, default 0",
    "cholesterol": "number, default 0",
    "preference": "string, enum: liked|disliked|neutral, default neutral",
    "notes": "string, optional"
}
```

**Returns:**
```json
{
    "id": 1,
    "name": "Coconut Aminos",
    "brand": "Coconut Secret",
    "created_at": "2025-01-09T14:30:00Z"
}
```

#### `search_food_items`

Search food items by name or brand.

**Parameters:**
```json
{
    "query": "string, required (searches name and brand)",
    "limit": "number, default 20, max 100"
}
```

**Returns:**
```json
{
    "items": [
        {
            "id": 1,
            "name": "Coconut Aminos",
            "brand": "Coconut Secret",
            "serving_size": 15,
            "serving_unit": "ml",
            "calories": 10,
            "preference": "liked"
        }
    ],
    "total": 1
}
```

#### `get_food_item`

Get full details for a food item including usage count.

**Parameters:**
```json
{
    "id": "number, required"
}
```

**Returns:**
```json
{
    "id": 1,
    "name": "Coconut Aminos",
    "brand": "Coconut Secret",
    "serving_size": 15,
    "serving_unit": "ml",
    "calories": 10,
    "protein": 0,
    "carbs": 2,
    "fat": 0,
    "fiber": 0,
    "sodium": 90,
    "sugar": 1,
    "saturated_fat": 0,
    "cholesterol": 0,
    "preference": "liked",
    "notes": "Soy sauce alternative",
    "created_at": "2025-01-09T14:30:00Z",
    "updated_at": "2025-01-09T14:30:00Z",
    "usage_count": 3,
    "used_in_recipes": ["Stir Fry", "Marinade"]
}
```

#### `update_food_item`

Update a food item (only if not used in any recipes).

**Parameters:**
```json
{
    "id": "number, required",
    "name": "string, optional",
    "brand": "string, optional",
    "serving_size": "number, optional",
    "serving_unit": "string, optional",
    "calories": "number, optional",
    "protein": "number, optional",
    "carbs": "number, optional",
    "fat": "number, optional",
    "fiber": "number, optional",
    "sodium": "number, optional",
    "sugar": "number, optional",
    "saturated_fat": "number, optional",
    "cholesterol": "number, optional",
    "preference": "string, optional",
    "notes": "string, optional"
}
```

**Returns:**
```json
{
    "success": true,
    "updated_at": "2025-01-09T15:00:00Z"
}
```

**Error (if in use):**
```json
{
    "error": "Cannot update food item: currently used in 3 recipes",
    "used_in": ["Stir Fry", "Marinade", "Buddha Bowl"]
}
```

#### `list_food_items`

List food items with optional filtering.

**Parameters:**
```json
{
    "preference": "string, optional (liked|disliked|neutral)",
    "sort_by": "string, default 'name' (name|created_at|calories)",
    "sort_order": "string, default 'asc' (asc|desc)",
    "limit": "number, default 50, max 200",
    "offset": "number, default 0"
}
```

---

### Recipe Tools

#### `create_recipe`

Create a new recipe (ingredients added separately).

**Parameters:**
```json
{
    "name": "string, required",
    "servings_produced": "number, default 1.0",
    "is_favorite": "boolean, default false",
    "notes": "string, optional"
}
```

**Returns:**
```json
{
    "id": 1,
    "name": "Morning Oatmeal",
    "created_at": "2025-01-09T14:30:00Z"
}
```

#### `get_recipe`

Get full recipe with ingredients and calculated nutrition.

**Parameters:**
```json
{
    "id": "number, required"
}
```

**Returns:**
```json
{
    "id": 1,
    "name": "Morning Oatmeal",
    "servings_produced": 2.0,
    "is_favorite": true,
    "ingredients": [
        {
            "id": 1,
            "food_item_id": 5,
            "food_item_name": "Rolled Oats",
            "quantity": 80,
            "unit": "g",
            "notes": null
        },
        {
            "id": 2,
            "food_item_id": 12,
            "food_item_name": "Whole Milk",
            "quantity": 240,
            "unit": "ml",
            "notes": null
        }
    ],
    "nutrition_per_serving": {
        "calories": 245,
        "protein": 8.5,
        "carbs": 35.2,
        "fat": 7.1,
        "fiber": 4.0,
        "sodium": 65,
        "sugar": 6.2,
        "saturated_fat": 2.8,
        "cholesterol": 12
    },
    "notes": "Add berries for extra fiber",
    "created_at": "2025-01-09T14:30:00Z",
    "updated_at": "2025-01-09T15:00:00Z",
    "times_logged": 15
}
```

#### `list_recipes`

List recipes with optional filtering.

**Parameters:**
```json
{
    "query": "string, optional (searches name)",
    "favorites_only": "boolean, default false",
    "sort_by": "string, default 'name' (name|created_at|times_logged)",
    "sort_order": "string, default 'asc'",
    "limit": "number, default 50",
    "offset": "number, default 0"
}
```

#### `update_recipe`

Update recipe metadata (only if not used in meal_entries).

**Parameters:**
```json
{
    "id": "number, required",
    "name": "string, optional",
    "servings_produced": "number, optional",
    "is_favorite": "boolean, optional",
    "notes": "string, optional"
}
```

#### `add_recipe_ingredient`

Add a food item to a recipe.

**Parameters:**
```json
{
    "recipe_id": "number, required",
    "food_item_id": "number, required",
    "quantity": "number, required",
    "unit": "string, required",
    "notes": "string, optional"
}
```

**Returns:**
```json
{
    "id": 1,
    "recipe_id": 1,
    "food_item_id": 5,
    "quantity": 80,
    "unit": "g"
}
```

**Side effect**: Triggers `recalculate_recipe_nutrition` automatically.

#### `update_recipe_ingredient`

Update an ingredient's quantity/unit.

**Parameters:**
```json
{
    "id": "number, required (recipe_ingredient id)",
    "quantity": "number, optional",
    "unit": "string, optional",
    "notes": "string, optional"
}
```

**Side effect**: Triggers `recalculate_recipe_nutrition` automatically.

#### `remove_recipe_ingredient`

Remove an ingredient from a recipe.

**Parameters:**
```json
{
    "id": "number, required (recipe_ingredient id)"
}
```

**Side effect**: Triggers `recalculate_recipe_nutrition` automatically.

#### `recalculate_recipe_nutrition`

Force recalculation of cached nutrition values. Called automatically when ingredients change, but can be invoked manually.

**Parameters:**
```json
{
    "recipe_id": "number, required"
}
```

**Returns:**
```json
{
    "recipe_id": 1,
    "nutrition_per_serving": {
        "calories": 245,
        "protein": 8.5,
        "carbs": 35.2,
        "fat": 7.1,
        "fiber": 4.0,
        "sodium": 65,
        "sugar": 6.2,
        "saturated_fat": 2.8,
        "cholesterol": 12
    }
}
```

---

### Meal Entry Tools

#### `log_meal`

Log a meal entry for a specific date.

**Parameters:**
```json
{
    "date": "string, required (ISO date: 2025-01-09)",
    "meal_type": "string, required (breakfast|lunch|dinner|snack|unspecified)",
    "recipe_id": "number, optional (either this OR food_item_id)",
    "food_item_id": "number, optional (either this OR recipe_id)",
    "servings": "number, default 1.0",
    "percent_eaten": "number, default 100 (0-100)",
    "notes": "string, optional"
}
```

**Side effects:**
- Auto-creates `days` record if not exists
- Calculates and caches nutrition for meal_entry
- Triggers `recalculate_day_nutrition` for the day

**Returns:**
```json
{
    "id": 1,
    "day_id": 5,
    "date": "2025-01-09",
    "meal_type": "breakfast",
    "source": "Morning Oatmeal (recipe)",
    "servings": 1.0,
    "percent_eaten": 100,
    "nutrition": {
        "calories": 245,
        "protein": 8.5,
        "carbs": 35.2,
        "fat": 7.1,
        "fiber": 4.0,
        "sodium": 65,
        "sugar": 6.2,
        "saturated_fat": 2.8,
        "cholesterol": 12
    }
}
```

#### `get_meal_entry`

Get details for a specific meal entry.

**Parameters:**
```json
{
    "id": "number, required"
}
```

#### `update_meal_entry`

Update a meal entry.

**Parameters:**
```json
{
    "id": "number, required",
    "meal_type": "string, optional",
    "servings": "number, optional",
    "percent_eaten": "number, optional",
    "notes": "string, optional"
}
```

**Note**: Cannot change the source (recipe_id/food_item_id). Delete and recreate if source change needed.

**Side effect**: Triggers `recalculate_day_nutrition`.

#### `delete_meal_entry`

Delete a meal entry.

**Parameters:**
```json
{
    "id": "number, required"
}
```

**Side effect**: Triggers `recalculate_day_nutrition`.

#### `list_meals_for_day`

Get all meal entries for a date.

**Parameters:**
```json
{
    "date": "string, required (ISO date)"
}
```

**Returns:**
```json
{
    "date": "2025-01-09",
    "meals": [
        {
            "id": 1,
            "meal_type": "breakfast",
            "source_type": "recipe",
            "source_name": "Morning Oatmeal",
            "servings": 1.0,
            "percent_eaten": 100,
            "nutrition": { ... }
        },
        {
            "id": 2,
            "meal_type": "lunch",
            "source_type": "food_item",
            "source_name": "Greek Yogurt",
            "servings": 1.5,
            "percent_eaten": 100,
            "nutrition": { ... }
        }
    ]
}
```

---

### Day Tools

#### `get_day_summary`

Get complete summary for a day including all meals and vitals.

**Parameters:**
```json
{
    "date": "string, required (ISO date)"
}
```

**Returns:**
```json
{
    "date": "2025-01-09",
    "meals": {
        "breakfast": [ ... ],
        "lunch": [ ... ],
        "dinner": [ ... ],
        "snack": [ ... ],
        "unspecified": [ ... ]
    },
    "nutrition_totals": {
        "calories": 1850,
        "protein": 95.2,
        "carbs": 180.5,
        "fat": 72.3,
        "fiber": 28.0,
        "sodium": 1950,
        "sugar": 45.2,
        "saturated_fat": 22.1,
        "cholesterol": 285
    },
    "vitals": [
        {
            "id": 1,
            "type": "weight",
            "timestamp": "2025-01-09T07:30:00Z",
            "value1": 185.4,
            "unit": "lbs"
        },
        {
            "id": 2,
            "type": "blood_pressure",
            "timestamp": "2025-01-09T07:32:00Z",
            "value1": 118,
            "value2": 78,
            "unit": "mmHg"
        }
    ],
    "notes": "Felt good today, energy levels high"
}
```

#### `get_date_range_summary`

Get daily nutrition totals for a date range.

**Parameters:**
```json
{
    "start_date": "string, required (ISO date)",
    "end_date": "string, required (ISO date)"
}
```

**Returns:**
```json
{
    "start_date": "2025-01-01",
    "end_date": "2025-01-09",
    "days": [
        {
            "date": "2025-01-01",
            "nutrition": { ... },
            "meal_count": 4
        },
        ...
    ],
    "averages": {
        "calories": 1920,
        "protein": 98.5,
        ...
    }
}
```

#### `update_day_notes`

Update notes for a specific day.

**Parameters:**
```json
{
    "date": "string, required (ISO date)",
    "notes": "string, required (can be empty string to clear)"
}
```

---

### Vital Tools

#### `log_vital`

Log a health vital measurement.

**Parameters:**
```json
{
    "vital_type": "string, required (weight|blood_pressure|heart_rate|oxygen_saturation|glucose)",
    "value1": "number, required",
    "value2": "number, optional (required for blood_pressure)",
    "unit": "string, required",
    "timestamp": "string, optional (ISO datetime, defaults to now)",
    "notes": "string, optional"
}
```

**Validation:**
- `blood_pressure`: requires both value1 (systolic) and value2 (diastolic)
- `weight`: unit must be "lbs" or "kg"
- `heart_rate`: unit must be "bpm"
- `oxygen_saturation`: unit must be "%"
- `glucose`: unit must be "mg/dL" or "mmol/L"

**Returns:**
```json
{
    "id": 1,
    "vital_type": "weight",
    "timestamp": "2025-01-09T07:30:00Z",
    "value1": 185.4,
    "value2": null,
    "unit": "lbs"
}
```

#### `get_vitals`

Query vital measurements with filtering.

**Parameters:**
```json
{
    "vital_type": "string, optional (filter by type)",
    "start_date": "string, optional (ISO date)",
    "end_date": "string, optional (ISO date)",
    "limit": "number, default 50",
    "offset": "number, default 0"
}
```

**Returns:**
```json
{
    "vitals": [
        {
            "id": 1,
            "vital_type": "weight",
            "timestamp": "2025-01-09T07:30:00Z",
            "value1": 185.4,
            "value2": null,
            "unit": "lbs",
            "notes": "Morning weigh-in"
        }
    ],
    "total": 45
}
```

#### `update_vital`

Update a vital measurement.

**Parameters:**
```json
{
    "id": "number, required",
    "value1": "number, optional",
    "value2": "number, optional",
    "unit": "string, optional",
    "timestamp": "string, optional",
    "notes": "string, optional"
}
```

#### `delete_vital`

Delete a vital measurement.

**Parameters:**
```json
{
    "id": "number, required"
}
```

---

### Utility Tools

#### `convert_unit`

Convert a value between units.

**Parameters:**
```json
{
    "value": "number, required",
    "from_unit": "string, required",
    "to_unit": "string, required"
}
```

**Returns:**
```json
{
    "original": 1,
    "original_unit": "cup",
    "converted": 236.588,
    "converted_unit": "ml"
}
```

**Error (incompatible types):**
```json
{
    "error": "Cannot convert between volume (cup) and weight (g) units"
}
```

---

## Nutrition Calculation Logic

### Recipe Nutrition Calculation

```rust
fn calculate_recipe_nutrition(recipe: &Recipe, ingredients: &[RecipeIngredient]) -> Nutrition {
    let mut total = Nutrition::default();
    
    for ingredient in ingredients {
        let food_item = get_food_item(ingredient.food_item_id);
        
        // Convert ingredient quantity to food_item's base unit
        let multiplier = convert_to_base_units(
            ingredient.quantity, 
            &ingredient.unit,
            food_item.serving_size,
            &food_item.serving_unit
        )?;
        
        // Add scaled nutrition
        total.calories += food_item.calories * multiplier;
        total.protein += food_item.protein * multiplier;
        // ... etc
    }
    
    // Divide by servings_produced to get per-serving values
    total / recipe.servings_produced
}
```

### Meal Entry Nutrition Calculation

```rust
fn calculate_meal_nutrition(entry: &MealEntry) -> Nutrition {
    let base_nutrition = if let Some(recipe_id) = entry.recipe_id {
        get_recipe_nutrition_per_serving(recipe_id)
    } else {
        get_food_item_nutrition(entry.food_item_id.unwrap())
    };
    
    // Apply servings and percent eaten
    base_nutrition * entry.servings * (entry.percent_eaten / 100.0)
}
```

### Day Nutrition Calculation

```rust
fn calculate_day_nutrition(day_id: i64) -> Nutrition {
    let entries = get_meal_entries_for_day(day_id);
    entries.iter()
        .map(|e| e.cached_nutrition())
        .sum()
}
```

---

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum UhmError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Not found: {entity} with id {id}")]
    NotFound { entity: String, id: i64 },
    
    #[error("Cannot modify {entity}: {reason}")]
    ModificationBlocked { entity: String, reason: String },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Unit conversion error: cannot convert {from} to {to}")]
    UnitConversion { from: String, to: String },
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}
```

### MCP Error Response Format

```json
{
    "error": {
        "code": "MODIFICATION_BLOCKED",
        "message": "Cannot update food item: currently used in 3 recipes",
        "details": {
            "entity": "food_item",
            "id": 5,
            "used_in": ["Stir Fry", "Marinade", "Buddha Bowl"]
        }
    }
}
```

---

## MCP Server Configuration

### Claude Desktop Config Entry

```json
{
    "mcpServers": {
        "uhm": {
            "command": "D:\\Projects\\UHM\\target\\release\\uhm.exe",
            "args": [],
            "env": {
                "UHM_DATABASE_PATH": "D:\\Projects\\UHM\\data\\uhm.db",
                "RUST_LOG": "info"
            }
        }
    }
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `UHM_DATABASE_PATH` | `./data/uhm.db` | Path to SQLite database |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

---

## Testing Strategy

### Unit Tests

- Nutrition calculation accuracy
- Unit conversion correctness
- Validation logic for all inputs

### Integration Tests

- Full CRUD cycles for each entity
- Cascade behavior (day auto-creation, nutrition recalculation)
- Permission enforcement (blocked modifications)

### Test Data

Include a `seed_test_data()` function that populates the database with sample data for testing:
- 10-15 common food items
- 3-5 recipes with varied complexity
- 1 week of meal entries
- Sample vitals

---

## Future Considerations (Phase 2+)

1. **Medications module** - Track medications, dosages, schedules, and adherence
2. **Lab results** - Store and trend lab values over time
3. **Goals/Targets** - Set daily nutrition goals, track progress
4. **Export** - CSV/JSON export of data
5. **External API integration** - USDA FoodData Central, nutrition APIs
6. **Web interface** - Optional HTTP transport for web access
7. **Sync** - Cross-device sync via PostgreSQL or similar

---

## Implementation Notes for Claude Code

1. **Start with schema** - Create the database and run migrations first
2. **Models next** - Define Rust structs matching the tables
3. **Tools one at a time** - Implement and test each tool before moving to the next
4. **Nutrition calculation is critical** - Get this right, with comprehensive unit tests
5. **Use transactions** - Wrap multi-step operations (like meal logging with day recalculation) in transactions
6. **Logging** - Use `tracing` for structured logging throughout
7. **Keep it simple** - Phase 1 doesn't need to be perfect, just functional

### Recommended Implementation Order

1. Database connection and migrations
2. FoodItem model and tools (add, search, get, list)
3. Recipe model and tools (create, get, list)
4. RecipeIngredient model and tools (add, update, remove)
5. Nutrition calculation engine
6. Day model (auto-creation logic)
7. MealEntry model and tools (log, get, update, delete)
8. Day recalculation and summary tools
9. Vital model and tools
10. MCP server integration and testing

---

## Appendix: Sample Data

### Sample Food Items

```sql
INSERT INTO food_items (name, brand, serving_size, serving_unit, calories, protein, carbs, fat, fiber, sodium, sugar, saturated_fat, cholesterol, preference) VALUES
('Rolled Oats', 'Quaker', 40, 'g', 150, 5, 27, 3, 4, 0, 1, 0.5, 0, 'liked'),
('Whole Milk', 'Generic', 240, 'ml', 150, 8, 12, 8, 0, 105, 12, 5, 35, 'neutral'),
('Chicken Breast', 'Generic', 100, 'g', 165, 31, 0, 3.6, 0, 74, 0, 1, 85, 'liked'),
('Brown Rice', 'Generic', 195, 'g', 216, 5, 45, 1.8, 3.5, 10, 0, 0.4, 0, 'neutral'),
('Olive Oil', 'Kirkland', 15, 'ml', 120, 0, 0, 14, 0, 0, 0, 2, 0, 'liked'),
('Broccoli', 'Generic', 91, 'g', 31, 2.5, 6, 0.4, 2.4, 30, 1.5, 0.1, 0, 'liked'),
('Egg', 'Generic', 50, 'g', 72, 6, 0.4, 5, 0, 71, 0.2, 1.6, 186, 'liked'),
('Greek Yogurt', 'Fage', 170, 'g', 100, 18, 6, 0, 0, 65, 6, 0, 10, 'liked'),
('Banana', 'Generic', 118, 'g', 105, 1.3, 27, 0.4, 3.1, 1, 14, 0.1, 0, 'liked'),
('Almonds', 'Generic', 28, 'g', 164, 6, 6, 14, 3.5, 0, 1, 1.1, 0, 'liked');
```

### Sample Recipe

```sql
-- Create recipe
INSERT INTO recipes (name, servings_produced, is_favorite, notes) VALUES
('Morning Oatmeal', 1, 1, 'Quick and filling breakfast');

-- Add ingredients (assuming recipe_id = 1)
INSERT INTO recipe_ingredients (recipe_id, food_item_id, quantity, unit) VALUES
(1, 1, 40, 'g'),   -- 40g rolled oats
(1, 2, 120, 'ml'), -- 120ml whole milk
(1, 9, 0.5, 'each'); -- half a banana
```

---

*End of Design Document*
