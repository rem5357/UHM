# UHM - Universal Health Manager

## Project Overview

UHM is a health and nutrition tracking system built as an MCP (Model Context Protocol) server in Rust. It enables AI assistants like Claude to help users track their food intake, recipes, daily meals, and nutritional information through natural conversation.

## What We've Accomplished

### Phase 1: Foundation
- Created Rust project with MCP server using the `rmcp` crate
- Implemented auto-incrementing build number system
- Built `uhm_status` tool for querying service status (build info, uptime, memory usage, database info)
- Startup banner displaying version and build information

### Phase 2: Database
- SQLite database with r2d2 connection pooling
- WAL mode for better concurrent access
- Full schema with migrations support:
  - `food_items` - Base nutritional data
  - `recipes` - Recipe definitions with cached nutrition
  - `recipe_ingredients` - Junction table linking food items to recipes
  - `days` - Daily containers with cached nutrition totals
  - `meal_entries` - Logged food consumption
  - `vitals` - Health measurements with optional group linking
  - `vital_groups` - Groups related vital readings together

### Phase 3: Food Item Tools
- `add_food_item` - Create food items with full nutritional data
- `search_food_items` - Search by name or brand
- `get_food_item` - Get detailed food item info with recipe usage count
- `list_food_items` - List with filtering, sorting, pagination
- `update_food_item` - Update food item (auto-recalculates nutrition for recipes using it)
- `delete_food_item` - Delete unused food items (blocked if used in any recipe)

### Phase 4: Recipe Tools
- `create_recipe` - Create recipes (ingredients added separately)
- `get_recipe` - Get full recipe with ingredients and calculated nutrition
- `list_recipes` - List with search, favorites filter, sorting
- `update_recipe` - Update metadata (blocked if logged in meals)
- `delete_recipe` - Delete unused recipes (blocked if logged or used as component)
- `add_recipe_ingredient` - Add single food item to recipe
- `add_recipe_ingredients_batch` - **PREFERRED** Add multiple ingredients in ONE call (much faster)
- `update_recipe_ingredient` - Modify ingredient quantities
- `remove_recipe_ingredient` - Remove ingredients
- `recalculate_recipe_nutrition` - Force nutrition recalculation

### Phase 5: Day & Meal Entry Tools
- `get_or_create_day` - Get or create a day by date
- `get_day` - Full day view with meals organized by type
- `list_days` - List days with date range filtering
- `update_day` - Update day notes
- `log_meal` - Log food item or recipe consumption
- `get_meal_entry` - Get meal entry details
- `update_meal_entry` - Update servings, percent eaten, etc.
- `delete_meal_entry` - Remove meal entries
- `recalculate_day_nutrition` - Force daily totals recalculation

### Phase 6: AI Assistant Support
- `meal_instructions` - Returns step-by-step guide for logging meals (for AI assistants to reference)

### Phase 7: Recipe Components
- Recipes can now use other recipes as components (sub-recipes)
- `add_recipe_component` - Add a recipe as a component of another recipe
- `update_recipe_component` - Update component servings
- `remove_recipe_component` - Remove a component from a recipe
- Circular reference detection prevents infinite loops (A uses B uses A)
- Nutrition automatically cascades through component hierarchy
- `get_recipe` now returns both ingredients and components with full details

### Phase 8: Cleanup/Maintenance Tools
- `list_unused_food_items` - Find all food items not used in any recipe. Safe to delete with `delete_food_item`.
- `list_unused_recipes` - Find all recipes with zero uses (not logged in meals, not used as component in other recipes). Safe to delete with `delete_recipe`.
- `list_orphaned_days` - Find all days with no meal entries. Safe to delete with `delete_day`.
- `delete_day` - Delete a day by date. Only succeeds if the day has no meal entries.
- Efficient SQL queries to quickly identify cleanup candidates
- Workflow: Use list_unused_* / list_orphaned_* tools to find orphans, then delete with corresponding delete tools

### Phase 9: Medication Tracking
- **Types**: prescription, supplement, otc, natural, compound, medical_device, other
- **Dosage units**: mg, mcg, g, ml, fl_oz, pill, tablet, capsule, spray, drop, patch, injection, unit, iu, puff, other
- **Tools**:
  - `add_medication` - Add a new medication with full details
  - `get_medication` - Get full medication details
  - `list_medications` - List with optional filtering by active status and type
  - `search_medications` - Search by name
  - `update_medication` - Update (requires force=true)
  - `deprecate_medication` - Mark as inactive (preferred over deletion)
  - `reactivate_medication` - Restore a deprecated medication
  - `delete_medication` - Remove permanently (requires force=true)
  - `export_medications_markdown` - Generate formatted markdown document
- **Philosophy**: Medications should be deprecated, not deleted, to preserve history. For dosage changes, deprecate old and add new.
- **Export**: Generates markdown with patient name, date/time, medications grouped by type (prescriptions first), includes doctor, pharmacy, instructions
- **Prescription fields**: prescribing_doctor, prescribed_date, pharmacy, rx_number, refills_remaining
- **Tracking fields**: start_date, end_date, discontinue_reason, is_active

### Phase 10: Vitals Tracking
- **Vital Types**: weight, blood_pressure, heart_rate, oxygen_saturation, glucose
- **Units**: lbs/kg, mmHg, bpm, %, mg/dL
- **Vital Groups**: Link related readings together (e.g., BP + HR taken at same time)
- **Tools**:
  - `add_vital` - Record a vital reading with optional group association
  - `get_vital` - Get vital details
  - `update_vital` - Update values or notes
  - `delete_vital` - Remove a vital reading
  - `list_vitals_by_type` - List readings for a specific vital type
  - `list_recent_vitals` - List recent readings across all types
  - `list_vitals_by_date_range` - Query by date range with optional type filter
  - `get_latest_vitals` - Get most recent reading for each vital type
  - `create_vital_group` - Create a group to link related readings
  - `get_vital_group` - Get group with all linked vitals
  - `list_vital_groups` - List groups with vital summaries
  - `update_vital_group` - Update group description/notes
  - `delete_vital_group` - Delete group (unlinks vitals but doesn't delete them)
  - `assign_vital_to_group` - Link or unlink a vital to/from a group
  - `vital_instructions` - Instructions for using vital tracking tools
- **Use Cases**:
  - Quick standalone readings (e.g., weight check)
  - Grouped readings (e.g., BP + HR from same measurement session)
  - Post-exercise vitals (BP, HR, O2 grouped together)
  - Retroactive grouping (link existing vitals to a new group)

### Phase 11: Unit Management Module (UMM)
- **Purpose**: Comprehensive unit conversion system for accurate recipe nutrition calculations
- **Problem Solved**: Fixed bug where "8 tbsp" of a "2 tbsp (20g)" serving calculated 8x nutrition instead of 4x
- **New Database Fields** (Migration v5):
  - `base_unit_type` - weight, volume, or count
  - `grams_per_serving` - Total grams in one serving (for weight-based calculations)
  - `ml_per_serving` - Total ml in one serving (for volume-based calculations)
  - `food_item_conversions` table - Custom unit conversions per food item (scoop, slice, etc.)
- **Unit Categories**:
  - **Weight**: g, oz, lb, kg (converts to grams)
  - **Volume**: tbsp, tsp, cup, ml, fl oz (converts to ml)
  - **Count**: each, piece, slice (uses grams_per_serving)
  - **Custom**: scoop, patty (requires food_item_conversions entry)
- **Smart Unit Parsing**:
  - Parses compound units like "tbsp (20g)" to extract gram weight
  - Auto-infers base_unit_type and grams/ml values from serving_unit
- **Cascading Recalculation**:
  - When a food item is updated, ALL affected data is automatically recalculated:
    1. All recipes using that food item as an ingredient
    2. All parent recipes using those recipes as components (recursive)
    3. All days with meal entries for affected recipes
  - Response shows `recipes_recalculated` and `days_recalculated` counts
- **Files**:
  - `src/nutrition/units.rs` - Unit types, categories, conversion constants
  - `src/nutrition/converter.rs` - Unit parsing and conversion functions
  - `src/db/migrations.rs` - Migration v5 with auto-migration of existing data

### Phase 12: Batch Update Mode
- **Purpose**: Efficient bulk food item updates without performance degradation
- **Problem Solved**: Updating 50 food items caused 50 cascade recalculations of the same recipes, leading to slowdowns/crashes
- **New Tools**:
  - `start_batch_update` - Enter batch mode, defer cascade recalculations
  - `finish_batch_update` - Perform ONE combined cascade for all changed items
- **How It Works**:
  1. Call `start_batch_update()` before bulk updates
  2. Call `update_food_item()` normally - updates happen but cascade is deferred
  3. Call `finish_batch_update()` to perform combined cascade
- **Implementation**:
  - `BatchUpdateState` struct tracks active mode and changed food item IDs
  - `update_food_item` checks batch state, uses `update_food_item_no_cascade` if active
  - `batch_cascade_recalculate()` processes all changed items efficiently
  - Topological sort ensures recipes recalculated in correct dependency order
- **Recovery**: If crash during batch mode, food item updates are saved - just call `finish_batch_update()`
- **Files Modified**:
  - `src/mcp/server.rs` - Batch state, new tools
  - `src/tools/food_items.rs` - `update_food_item_no_cascade`, `batch_cascade_recalculate`
  - `src/tools/status.rs` - Batch update documentation in meal_instructions

### Phase 13: Batch Recipe Ingredients
- **Purpose**: Fast recipe creation by adding all ingredients in one tool call
- **Problem Solved**: Building an 8-ingredient recipe required 9 tool calls (1 create + 8 add_ingredient), with Claude Desktop thinking time between each call leading to ~2 minute build times
- **New Tool**:
  - `add_recipe_ingredients_batch` - Add multiple ingredients in ONE call
- **Benefits**:
  - Reduces tool calls from N+1 to 2 (create_recipe + add_recipe_ingredients_batch)
  - Only recalculates nutrition ONCE at the end (not after each ingredient)
  - Eliminates Claude Desktop thinking time overhead between ingredient additions
  - Returns detailed success/failure status for each ingredient
- **Usage**:
  ```
  add_recipe_ingredients_batch(
    recipe_id: 6,
    ingredients: [
      { food_item_id: 32, quantity: 320, unit: "g", notes: "4 cups" },
      { food_item_id: 29, quantity: 248, unit: "g", notes: "8 scoops" },
      ...
    ]
  )
  ```
- **Files Modified**:
  - `src/tools/recipes.rs` - `BatchIngredient`, `add_recipe_ingredients_batch`
  - `src/mcp/server.rs` - Tool registration
  - `src/tools/status.rs` - Updated meal_instructions

### Phase 14: Omron BP Import & Statistics Tools
- **Purpose**: Batch import blood pressure data + comprehensive statistics for all data types
- **Omron Import Tool**: `import_omron_bp_csv`
  - Input: Full file path to Omron CSV export
  - CSV Format: Date, Time, Systolic, Diastolic, Pulse, Symptoms, Consumed, TruRead, Notes
  - Creates vital groups linking BP + HR readings
  - Handles duplicates gracefully (skips if same timestamp + values exist)
- **Statistics Tools**:
  - `list_days_stats` - Nutrition statistics across logged days
  - `list_vitals_stats` - Vital sign statistics by type
- **Statistics Returned** (for each metric):
  - count, sum, average, median, mode
  - standard_deviation, variance
  - min, max, range
  - percentile_25, percentile_75, iqr
  - coefficient_of_variation
  - outliers (values outside 1 SD with z-scores)
- **Vital-Specific Stats**:
  - Weight: total_change, avg_change_per_reading
  - Blood Pressure: separate stats for systolic, diastolic, pulse_pressure
  - Oxygen Saturation: below_95_count, below_90_count
  - Glucose: low_count (<70), high_count (>180)

### Phase 15: Exercise Tracking
- **Purpose**: Track treadmill workouts with automatic calorie calculation
- **Database Schema** (Migration v6):
  - `exercises` - Workout sessions linked to days
  - `exercise_segments` - Individual segments with different settings
  - `days.cached_calories_burned` - Daily exercise calorie total
- **Exercise Tools**:
  - `add_exercise` - Create exercise session for a day
  - `get_exercise` - Get exercise with all segments
  - `list_exercises` - List with optional date range filter
  - `list_exercises_for_day` - List exercises for a specific day
  - `update_exercise` - Update notes, link vital groups
  - `delete_exercise` - Delete exercise and all segments
  - `add_exercise_segment` - Add segment (provide 2 of 3: duration, speed, distance)
  - `update_exercise_segment` - Update segment values
  - `delete_exercise_segment` - Delete a segment
  - `list_exercise_stats` - Exercise statistics (duration, distance, calories, speed, incline)
  - `exercise_instructions` - Instructions for AI assistants
- **Key Features**:
  - **Auto-calculation**: Given 2 of (duration, speed, distance), calculates 3rd
  - **Consistency check**: If all 3 provided, verifies they match (flags if inconsistent)
  - **Calorie calculation**: Uses MET formula with latest weight from vitals
  - **MET values**: Vary by speed (2.0 mph = 2.0 MET → 6.0 mph = 9.0 MET)
  - **Incline adjustment**: Adds ~0.1 MET per 1% grade
  - **Vital group linking**: Link PRE and POST exercise BP/HR for recovery tracking
- **Integration with Days**:
  - `get_day` now includes exercises and net_calories (consumed - burned)
  - Day totals update automatically when exercises change
- **Files Modified**:
  - `src/db/migrations.rs` - Migration v6
  - `src/models/exercise.rs` - Exercise and ExerciseSegment models
  - `src/tools/exercise.rs` - Exercise tool functions
  - `src/tools/days.rs` - Updated get_day to include exercises
  - `src/tools/status.rs` - EXERCISE_INSTRUCTIONS
  - `src/mcp/server.rs` - Tool registrations

### Phase 16: Duplicate Vitals Detection
- **Purpose**: Identify and clean up duplicate vital readings between exercise-linked and standalone entries
- **Problem Solved**: When entering BP during exercise tracking, readings may be entered both as part of exercise PRE/POST vital groups AND as standalone vitals, causing duplicates
- **New Tools**:
  - `find_duplicate_vitals` - Finds potential duplicates by matching:
    - Same vital type (BP with BP, HR with HR)
    - Same values (systolic/diastolic for BP, bpm for HR)
    - Timestamps within configurable window (default: 60 minutes)
  - `delete_vitals_bulk` - Delete multiple vitals by ID for confirmed duplicate removal
- **How It Works**:
  1. Queries all vitals in exercise-linked vital groups (PRE/POST)
  2. Queries all standalone vitals (not in exercise groups)
  3. Compares and reports pairs where values match within time window
  4. Returns exercise_vital (to keep) and standalone_vital (candidate for deletion)
- **Usage**:
  ```
  1. Call find_duplicate_vitals() to scan for duplicates
  2. Review pairs - exercise_vital is usually the one to keep
  3. Call delete_vitals_bulk([ids]) to remove confirmed duplicates
  ```
- **Files Modified**:
  - `src/tools/vitals.rs` - find_duplicate_vitals, delete_vitals_bulk functions
  - `src/mcp/server.rs` - Tool registrations and params

## Technology Stack

### Rust
- **Why Rust**: Memory safety, performance, excellent error handling, strong type system
- **Async Runtime**: Tokio for async I/O
- **Database**: rusqlite with r2d2 connection pooling
- **Serialization**: serde + serde_json
- **Schema Generation**: schemars for JSON Schema from Rust types

### MCP (Model Context Protocol)
- **Crate**: `rmcp` v0.8
- **Transport**: stdio (for Claude Desktop integration)
- **Tools**: Defined using `#[tool]` macro with automatic schema generation
- **Server Handler**: Implements `ServerHandler` trait for MCP protocol

### Build System
- **Cargo**: Standard Rust build tool
- **build.rs**: Custom build script for compile-time code generation

## Build Number System

The build number automatically increments **only when source files change**, not on every `cargo build`.

### How It Works

1. **build.rs** script runs before compilation
2. Uses `cargo:rerun-if-changed=src` directive - only triggers on src/ changes
3. Reads current build number from `build_number.txt`
4. Increments and writes back the new number
5. Sets environment variables for compile-time embedding:
   - `UHM_BUILD_NUMBER` - The incremented build number
   - `UHM_BUILD_TIMESTAMP` - ISO 8601 datetime of compilation

### Files Involved
- `build.rs` - Build script with increment logic
- `build_number.txt` - Persistent counter file
- `src/build_info.rs` - Compile-time constants using `env!()` macro

### Current Build
Build number starts at 0 and increments to 1 on first build. Each source change triggers a new build number.

## Lessons Learned

### rmcp Crate API
- `Error` type was deprecated in favor of `ErrorData`
- `ServerInfo` structure requires specific fields: `protocol_version`, `capabilities`, `server_info`, `instructions`
- `Implementation` struct needs `title`, `icons`, `website_url` fields (can be None/Some)
- Use `#[tool_router]` on impl block and `#[tool]` on individual methods

### sysinfo Crate
- API changed: `refresh_process()` became `refresh_processes()` with `ProcessesToUpdate` enum
- Must use `ProcessesToUpdate::Some(&[Pid::from_u32(pid)])` for single process refresh

### Rust Patterns
- Connection pooling with r2d2 simplifies database access
- Using `Box<dyn ToSql>` for dynamic SQL parameter building
- Cached nutrition values avoid expensive recalculations on every query
- Data integrity rules: block updates to items used in dependent records

### MCP Integration
- Claude Desktop config at `%APPDATA%\Claude\claude_desktop_config.json`
- Server runs as stdio subprocess
- Tools appear automatically in Claude's interface after restart

### File Path Cross-Platform Issues
- **Problem**: Claude Desktop may run in a different environment (WSL, Docker, or different temp directories) than the UHM MCP server (native Windows)
- **Symptoms**: File paths like `/tmp/file.csv` don't work because UHM runs on Windows and expects paths like `D:\path\file.csv`
- **Solution**: When using file import tools, ensure the file is placed in a Windows-accessible path that UHM can reach
- **Example**: Copy CSV to `D:\Projects\UHM\data\import.csv` before calling `import_omron_bp_csv`

### Duplicate Data Prevention
- **Problem**: When tracking exercise with PRE/POST vitals, the same readings can accidentally be entered both as exercise-linked groups AND standalone vitals
- **Solution**: Use `find_duplicate_vitals` to scan for matches and `delete_vitals_bulk` to clean up
- **Best Practice**: Going forward, announce exercise time explicitly so timestamps align properly

## Project Structure

```
D:\Projects\UHM\
├── Cargo.toml              # Dependencies and project config
├── build.rs                # Build number auto-increment
├── build_number.txt        # Persistent build counter
├── Skills.md               # This document
├── UHM_DESIGN.md           # Original design specification
├── src/
│   ├── main.rs             # Entry point, startup banner, MCP server init
│   ├── build_info.rs       # Compile-time constants
│   ├── db/
│   │   ├── mod.rs
│   │   ├── connection.rs   # Database pool management
│   │   └── migrations.rs   # Schema creation
│   ├── models/
│   │   ├── mod.rs
│   │   ├── nutrition.rs    # Shared Nutrition struct
│   │   ├── food_item.rs    # FoodItem CRUD
│   │   ├── recipe.rs       # Recipe CRUD
│   │   ├── recipe_ingredient.rs  # Recipe ingredients + nutrition calc
│   │   ├── recipe_component.rs   # Recipe components (sub-recipes)
│   │   ├── day.rs          # Day CRUD
│   │   ├── meal_entry.rs   # MealEntry CRUD + day nutrition calc
│   │   ├── medication.rs   # Medication CRUD
│   │   ├── vital.rs        # Vital and VitalGroup CRUD
│   │   └── exercise.rs     # Exercise and ExerciseSegment CRUD
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── status.rs       # uhm_status implementation + instructions
│   │   ├── food_items.rs   # Food item tool functions
│   │   ├── recipes.rs      # Recipe tool functions
│   │   ├── days.rs         # Day and meal entry tool functions + stats
│   │   ├── medications.rs  # Medication tool functions
│   │   ├── vitals.rs       # Vital tool functions + stats
│   │   └── exercise.rs     # Exercise tool functions + stats
│   └── mcp/
│       ├── mod.rs
│       └── server.rs       # MCP server, all tool definitions
└── data/
    └── uhm.db              # SQLite database (created on first run)
```

## Todo / Future Work

### To Resolve (Known Issues)
- [ ] **Omron CSV Import Path Issue**: `import_omron_bp_csv` fails when Claude Desktop passes non-Windows paths (e.g., `/tmp/file.csv`). UHM runs on Windows and can't access Linux-style paths. Potential fixes:
  - Add path validation/conversion in the import tool
  - Have the tool accept file content as base64 instead of file path
  - Document workaround: copy file to Windows-accessible path first
  - Investigate if Claude Desktop can be configured to use Windows temp paths

### Pending Implementation
- [ ] Unit conversion utilities (grams ↔ oz, ml ↔ cups, etc.)
- [ ] Nutrition goals and daily targets
- [ ] Weekly/monthly nutrition reports
- [ ] Food item import from external databases (USDA, etc.)

### Potential Enhancements
- [ ] Barcode scanning integration
- [ ] Meal planning and suggestions
- [ ] Recipe scaling (adjust servings)
- [ ] Ingredient substitution suggestions
- [ ] Export data to CSV/JSON
- [ ] Web dashboard for visualization

## Running UHM

### Development
```bash
cargo run
```

### Release Build
```bash
cargo build --release
```

### Claude Desktop Integration
Add to `%APPDATA%\Claude\claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "uhm": {
      "command": "D:\\Projects\\UHM\\target\\release\\uhm.exe",
      "args": []
    }
  }
}
```

Restart Claude Desktop to load the UHM server.
