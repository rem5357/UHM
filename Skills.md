# UHM - Universal Health Manager

**Version:** 1.1.0 | **Build:** 132 | **Updated:** 2026-03-20

## Changelog

- **Build 132** (2026-03-20): Weight Watchers points integration — SmartPoints formula on food items, ZeroPoint food flagging, meal/day/exercise WW tracking, tier classification system.
- **Build 123** (2026-03-04): Fuzzy search tier — strsim Jaro-Winkler between LIKE and Haiku. `search_method` field on batch results.

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

### Phase 16: Duplicate Vitals Detection & Auto-Cleanup
- **Purpose**: Identify and clean up duplicate vital readings between exercise-linked and standalone entries
- **Problem Solved**: When entering BP during exercise tracking, readings may be entered both as part of exercise PRE/POST vital groups AND as standalone vitals, causing duplicates
- **New Tools**:
  - `find_duplicate_vitals` - Finds potential duplicates by matching:
    - Same vital type (BP with BP, HR with HR)
    - Same values (systolic/diastolic for BP, bpm for HR)
    - Timestamps within configurable window (default: 24 hours / 1440 minutes)
  - `delete_vitals_bulk` - Delete multiple vitals by ID for confirmed duplicate removal
- **Auto-Cleanup After Import**:
  - `import_omron_bp_csv` now automatically removes duplicate standalone vitals after import
  - Keeps exercise-linked vitals (which have PRE/POST context), deletes matching standalone duplicates
  - Response includes `duplicates_cleaned_bp` and `duplicates_cleaned_hr` counts
  - Uses 24-hour window - exact matches on same day are unlikely to be coincidental
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
  Or simply import via `import_omron_bp_csv` - cleanup happens automatically.
- **Files Modified**:
  - `src/tools/vitals.rs` - find_duplicate_vitals, delete_vitals_bulk, auto_cleanup_exercise_duplicates
  - `src/mcp/server.rs` - Tool registrations and params

### Phase 17: Full Field Updateability
- **Purpose**: Allow all fields in exercises and vitals to be updated, not just notes
- **Problem Solved**: Timestamps could not be corrected after entry; some fields were read-only
- **Exercise Updates** (`update_exercise`):
  - Now accepts `timestamp` parameter to fix incorrect exercise times
  - All fields: timestamp, pre_vital_group_id, post_vital_group_id, notes
- **Vital Updates** (`update_vital`):
  - Now accepts `timestamp` parameter to correct reading times
  - Now accepts `group_id` parameter to link/unlink from vital groups
  - All fields: timestamp, value1, value2, unit, group_id, notes
- **Files Modified**:
  - `src/models/exercise.rs` - Added timestamp to ExerciseUpdate struct
  - `src/models/vital.rs` - Added timestamp to VitalUpdate struct
  - `src/tools/exercise.rs` - Updated update_exercise function
  - `src/tools/vitals.rs` - Updated update_vital function
  - `src/mcp/server.rs` - Updated tool parameter structs

### Phase 18: Compendium MET Values
- **Purpose**: Use standardized MET values from the Compendium of Physical Activities
- **Problem Solved**: Original MET values were estimates; standardized values improve accuracy
- **MET Formula**: `calories = MET × weight_kg × duration_hours`
- **Updated MET Table** (by speed in mph):
  | Speed | MET | Activity Description |
  |-------|-----|---------------------|
  | <2.0  | 2.0 | Very slow walking |
  | 2.0-2.4 | 2.8 | Slow walking |
  | 2.5-2.9 | 3.0 | Leisurely walking |
  | 3.0-3.4 | 3.5 | Moderate walking |
  | 3.5-3.9 | 4.3 | Brisk walking |
  | 4.0-4.4 | 5.0 | Very brisk walking |
  | 4.5-4.9 | 7.0 | Jogging/running |
  | 5.0-5.4 | 8.3 | Running |
  | 5.5-5.9 | 9.0 | Running |
  | 6.0-6.9 | 9.8 | Running |
  | 7.0-7.9 | 10.5 | Running |
  | 8.0-8.9 | 11.5 | Fast running |
  | 9.0+  | 12.8 | Very fast running |
- **Incline Adjustment**: +0.1 MET per 1% grade (unchanged)
- **Files Modified**:
  - `src/models/exercise.rs` - Updated MET lookup table in calculate_met()

### Phase 19: Batch Weight Import
- **Purpose**: Enable bulk import of historical weight data without per-entry tool calls
- **Problem Solved**: Importing a year of weight data from logs/documents would require hundreds of individual `add_vital` calls
- **New Tools**:
  - `add_weights_batch` - Add multiple weight entries in a single call
    - Input: Array of `{date, value, unit?}` objects
    - Date formats: `YYYY-MM-DD`, `MM/DD/YYYY`, `MM-DD-YYYY`
    - Unit defaults to "lbs" if omitted
    - Skips duplicates (same date + value already exists)
    - Returns per-entry status: "added", "duplicate", or "error"
  - `import_weight_csv` - Import weights from CSV file
    - Format: `date,value,unit` (one per line, header optional)
    - Same date formats and duplicate handling as batch tool
- **Use Case**: Claude Desktop can extract weights from markdown logs/documents and send all entries in one `add_weights_batch` call
- **Example**:
  ```
  add_weights_batch(entries: [
    {date: "2025-01-15", value: 185.5},
    {date: "2025-01-16", value: 185.2, unit: "lbs"},
    {date: "1/17/2025", value: 184.8}
  ])
  ```
- **Files Modified**:
  - `src/tools/vitals.rs` - WeightEntry, add_weights_batch, import_weight_csv, parse_weight_date
  - `src/mcp/server.rs` - Tool registrations and parameter structs

### Phase 20: Cascading Day Nutrition Recalculation
- **Purpose**: Ensure `recalculate_day_nutrition` refreshes meal entry caches from their sources
- **Problem Solved**: When a recipe or food item is modified after meals are logged, the meal entry cache stays stale, so day totals remain wrong even after recalculation
- **Previous Behavior**: `recalculate_day_nutrition` only summed existing cached values from meal entries
- **New Behavior**: `recalculate_day_nutrition` now:
  1. For each meal entry on that day:
     - If source is recipe: fetches current `cached_nutrition` from recipe table
     - If source is food_item: fetches current `nutrition` from food_item table
     - Recalculates: `source_nutrition × servings × (percent_eaten / 100)`
     - Updates the meal entry's cached nutrition fields
  2. Sums all updated meal entries for day totals
  3. Returns as normal
- **Benefits**:
  - Recipe/food_item edits now propagate to historical meal entries automatically
  - No need to manually delete and re-log meals after source changes
  - Day totals always reflect current source nutrition values
- **Implementation**:
  - New helper function `refresh_meal_entry_nutrition()` fetches source and updates entry cache
  - `recalculate_day_nutrition()` calls this for each entry before summing
- **Files Modified**:
  - `src/models/meal_entry.rs` - refresh_meal_entry_nutrition, updated recalculate_day_nutrition

### Phase 21: Day Summary Report Generation
- **Purpose**: Generate comprehensive markdown report summarizing a day's meals, exercise, and nutrition status
- **New Tool**: `generate_day_summary`
  - Input: date (YYYY-MM-DD), optional output_path, optional include_ingredients (default true)
  - Output: Markdown file saved to Downloads folder + JSON summary stats
- **Report Sections**:
  1. **Header**: Formatted date, morning weight with change from yesterday, days to birthday (Oct 22, 2026)
  2. **Meal Sections**: For each meal entry:
     - Recipe/food name as header
     - Ingredient-level nutrition table (if include_ingredients=true)
     - Columns: Ingredient, Amount, Cal, Protein, Fat, Carbs, Fiber, Sodium
     - Meal total row
  3. **Exercise Sections**: For each exercise:
     - Metrics: Duration, Distance, Avg Speed, Calories Burned
     - Segment breakdown (if multiple segments)
     - Post-exercise BP/HR recovery table (if post_vital_group linked)
  4. **Day Summary**:
     - Meals breakdown table with gross totals
     - Net calories calculation (Gross - Exercise)
  5. **Status Check**: Target vs Actual with emoji status
     - Gross Calories: <2000
     - Net Calories: ≤1500
     - Protein: ≥140g
     - Sodium: <1800mg
  6. **Tier Classification**:
     - MEGA Win: net ≤1500 AND protein ≥140g
     - Super Win: gross <2000
     - Win: gross <3000
     - Over Budget: gross ≥3000
- **Response Includes**:
  - `file_path`: Path to generated markdown file
  - `summary`: JSON object with date, weight, weight_change, gross_calories, net_calories, protein, sodium, exercise_calories, tier, protein_status
- **Files Modified**:
  - `src/tools/reports.rs` - generate_day_summary, DaySummaryResponse, DaySummary structs, helper functions
  - `src/mcp/server.rs` - GenerateDaySummaryParams, tool registration

### Phase 22: Recipe-Free Workflow (Direct Meal Logging)
- **Purpose**: Streamline meal logging by allowing direct food item logging without recipe intermediaries
- **Problem Solved**: Creating a recipe for every meal added unnecessary overhead. Users now log food items directly with quantities.
- **Database Schema** (Migration v8):
  - Added `quantity` and `unit` columns to `meal_entries` table
  - Allows direct food item logging with specific amounts (e.g., 150g chicken)
- **New Tools**:
  - `search_food_items_batch` - Search multiple food items in ONE call with full nutrition data
    - Input: Array of query strings, fuzzy_match boolean, limit_per_query
    - Returns complete nutrition for all matches (no follow-up get_food_item needed)
    - Fuzzy matching via Haiku API for unmatched queries
  - `log_meal_items_batch` - Log multiple food items to a meal in ONE call
    - Input: date, meal_type, array of items with food_item_id, quantity, unit, optional percent_eaten/notes
    - Calculates nutrition based on quantity/unit conversion
    - Returns per-item results and updated day totals
- **Recipe Tools Removed from MCP** (code preserved):
  - `create_recipe`, `update_recipe`, `delete_recipe`
  - `add_recipe_ingredient`, `add_recipe_ingredients_batch`
  - `update_recipe_ingredient`, `remove_recipe_ingredient`
  - `add_recipe_component`, `update_recipe_component`, `remove_recipe_component`
  - `recalculate_recipe_nutrition`, `list_unused_recipes`
  - `get_recipe`, `list_recipes` (historical access also removed)
- **Compound Food Items**: For frequently-used combinations (DIYOO, protein coffee), create a single food item with combined nutrition and document recipe in notes field
- **Workflow Reduction**: Typical meal logging reduced from 4+ tool calls to 2 (search_food_items_batch + log_meal_items_batch)
- **Files Modified**:
  - `src/db/migrations.rs` - Migration v8 for quantity/unit columns
  - `src/models/meal_entry.rs` - create_direct(), calculate_direct_log_multiplier()
  - `src/tools/food_items.rs` - search_food_items_batch, FoodItemFullSummary structs
  - `src/tools/days.rs` - log_meal_items_batch, BatchMealItem structs
  - `src/mcp/server.rs` - New tools, recipe tools commented out
  - `src/tools/status.rs` - Updated MEAL_INSTRUCTIONS with new workflow

### Phase 23: FTS5 Full-Text Search & Improved Fuzzy Matching
- **Purpose**: Fix search failures when queries don't exactly match food item names
- **Problem Solved**: SQL LIKE search with phrases like "burrito shell" found nothing because "shell" ≠ "tortilla". Brand + name combinations (e.g., "king oscar salmon") also failed.
- **Database Schema** (Migration v9):
  - Created `food_items_fts` FTS5 virtual table indexing `name` and `brand`
  - Triggers keep FTS in sync on INSERT/UPDATE/DELETE
- **FTS5 Search Features**:
  - Word tokenization: "burrito shell" → `"burrito* OR shell*"`
  - Prefix matching: "salm" matches "Salmon"
  - Word-order independence: "golden monk fruit" finds "Monk Fruit Sweetener Golden"
  - Brand + name combined: "king oscar salmon" finds "Atlantic Salmon in Olive Oil" (brand: King Oscar)
  - Relevance ranking via FTS5 rank score
- **Search Hierarchy**:
  1. **FTS5 search** (fast, handles tokenization) - 80%+ of queries
  2. **LIKE fallback** (catches edge cases)
  3. **Haiku fuzzy suggestion** (semantic matching for synonyms like "shell" → "tortilla")
- **Improved Fuzzy Suggestion** (`get_fuzzy_suggestion`):
  - Now includes brand in food names sent to Haiku: "King Oscar - Atlantic Salmon..."
  - Improved prompt for better synonym/partial matching
  - Case-insensitive response validation
  - Contains-match fallback for partial responses
  - 10-second timeout for reliability
- **Benefits**:
  - FTS5 handles word tokenization without API calls
  - Haiku only called as final fallback (reduces cost/latency)
  - Brand searches now work naturally
- **Files Modified**:
  - `src/db/migrations.rs` - Migration v9 for FTS5
  - `src/models/food_item.rs` - FoodItem::search_fts()
  - `src/tools/food_items.rs` - Updated search_food_items_batch, fixed get_fuzzy_suggestion

### Phase 24: Exercise Performance PDF Report
- **Purpose**: Generate a 2-page PDF report covering exercise performance metrics and post-exercise BP recovery analysis
- **Architecture**: Python script generation — Rust collects data from SQLite, generates a Python script with data embedded as literals, shells out to Python (matplotlib + reportlab) for rendering
- **New Tool**: `generate_exercise_report`
  - Input: start_date, end_date, optional notes, optional output_path
  - Output: PDF saved to `C:\Users\rober\Downloads\Exercise_Report_{start}_to_{end}.pdf`
  - Returns: file_path, sessions, days_with_exercise, total_duration_minutes, total_distance_miles, total_calories_burned
- **Page 1 — Exercise Overview**:
  - Dark-theme charts using matplotlib
  - 3-panel overview: Duration (min), Distance (mi), Calories per session
  - Speed trend chart: Average speed per session
  - Adaptive chart mode: ≤31 days = bar charts, >31 days = smooth line charts
  - Date labels: "Mon DD" format, with AM/PM or session number for multiple-per-day
- **Page 2 — Post-Exercise BP Recovery**:
  - Spline-interpolated BP recovery curves (scipy, with numpy linear fallback)
  - Stats table with average early/late readings and recovery deltas
  - Clinical narrative summarizing BP recovery pattern
  - Only generated when post-exercise BP data exists
- **Post-Exercise BP Collection** (`collect_post_exercise_bp`):
  - Queries ALL vital groups with timestamps in 0–20 min window after exercise end
  - Also includes the exercise's linked `post_vital_group_id` if set
  - Handles split-group readings (1st BP in group N, 2nd BP in group N+1)
  - Filters BP readings to 0–20 min offset from exercise end time
  - Pairs first two BP readings as early/late, matches closest HR readings
- **Python Dependencies**: matplotlib, reportlab, numpy, scipy
- **Helper Functions**:
  - `build_exercise_report_python()` — generates complete Python script string
  - `collect_post_exercise_bp()` — unified BP pair extraction from any/all post-exercise vital groups
  - `find_closest_hr()` — matches HR reading closest in timestamp to a BP reading
  - `parse_timestamp()` — parses "YYYY-MM-DDTHH:MM:SS" to chrono NaiveDateTime
  - `month_abbrev()` — converts month number to 3-letter abbreviation
  - `escape_python_str()` — escapes strings for embedding in Python literals
- **Files Modified**:
  - `src/tools/reports.rs` — Full implementation (~900 lines): data collection, Python script generation, execution
  - `src/mcp/server.rs` — GenerateExerciseReportParams struct, tool registration

### Phase 25: Haiku Model Upgrade
- **Purpose**: Update fuzzy food search AI model from deprecated Haiku 3 to Haiku 4.5
- **Change**: Model ID `claude-3-haiku-20240307` → `claude-haiku-4-5-20251001`
- **Impact**: Improved semantic matching for fuzzy food name suggestions (e.g., "shell" → "tortilla")
- **Files Modified**:
  - `src/tools/food_items.rs` - Updated model ID in `get_fuzzy_suggestion`

### Phase 26: Medication List PDF Report
- **Purpose**: Generate a professional PDF medication list grouped by type, filling the last gap in UHM's report toolset
- **New Tool**: `generate_medications_report`
  - Input: patient_name, optional output_path, active_only (default true), include_notes (default true)
  - Output: PDF saved to `C:\Users\rober\Downloads\Medication_List_{name}_{date}.pdf`
  - Returns: success, file_path, medication_count, message
- **PDF Layout**:
  - Portrait letter-size (215.9mm x 279.4mm), auto-paginating
  - Header on every page: "Medication List" (blue), patient name, DOB, generated date, active/all filter status
  - Medications grouped by MedType, sorted by sort_order() (Prescription > Supplement > OTC > Natural > Compound > Medical Device > Other)
  - Per medication: name (12pt bold), dosage + frequency, instructions, prescription details (doctor, pharmacy, Rx#, refills), discontinue reason for inactive meds
  - Notes displayed conditionally based on include_notes parameter
  - Light dividers between medications, blue underlined section headers
  - Footer: disclaimer text + page number on every page
- **Response Struct**: `GenerateMedicationsReportResponse` with success, file_path, medication_count, message
- **Color**: `COLOR_MED_TITLE` = (0, 112, 192) -- blue for medication headers
- **Files Modified**:
  - `src/tools/reports.rs` - generate_medications_report(), GenerateMedicationsReportResponse, COLOR_MED_TITLE, format_dosage_amount()
  - `src/mcp/server.rs` - GenerateMedicationsReportParams, tool handler, updated server instructions

### Phase 28: FTS5 Search Fix, Verified Food Pipeline, Source Tracking & Audit
- **Purpose**: Fix cross-column FTS5 search, add food item provenance tracking, verified creation pipeline, and audit tool
- **Problems Solved**:
  1. **FTS5 cross-column bug**: "Willa's oat milk" failed because FTS5 indexed `name` and `brand` as separate columns — query couldn't match across both
  2. **The Bacon Incident**: Food items from AI estimates had no source tracking — raw bacon sodium (1720mg/100g) stored as cooked, inflating sodium by ~1000mg/day for 12+ days undetected
- **Database Schema** (Migration v10):
  - Added `source TEXT` and `source_detail TEXT` columns to `food_items` (nullable, NULL = legacy)
  - Rebuilt FTS5 with single unified `search_text` column: `COALESCE(brand, '') || ' ' || name`
  - Recreated FTS5 sync triggers using concatenated search_text
- **FTS5 Search Fix**:
  - Changed FTS5 query from OR semantics to implicit AND (space-separated prefix terms)
  - Added concatenated LIKE fallback: `COALESCE(brand, '') || ' ' || name LIKE ?`
  - Cross-column queries like "Willa's oat milk" now work correctly
- **Source Tracking**:
  - `source` field: "label_photo", "usda", "estimate", or NULL (legacy)
  - `source_detail` field: free-text provenance (USDA FDC ID, estimation notes, etc.)
  - Fully backward-compatible — existing `add_food_item` calls work without source fields
- **New Tool: `audit_food_items`**:
  - Filters: "all", "estimates_only", "no_source", "high_usage"
  - Returns items with usage counts (from meal_entries + recipe_ingredients)
  - Summary stats: total, estimates count, no_source count, label_verified, usda_verified
  - Sortable by usage_count, name, or calories
- **New Tool: `add_food_item_verified`**:
  - Tiered source resolution: label photo (Sonnet vision) → USDA FoodData Central → AI estimate with 80% rule
  - Automatic unit normalization: solids→per 100g, liquids→per 100ml
  - Local sanity checks (no expensive API calls):
    - Macro math: |actual - (P×4 + C×4 + F×9)| > 15% flags review
    - Sodium: >1000mg/100g flags review (raw vs cooked detection)
    - Cross-reference: >50% deviation from similar DB items flags review
  - Returns "created" with food_item_id or "review_needed" with flags and suggestions
- **New Module: `ai_client.rs`** — Shared Anthropic API client (Sonnet/Haiku) with text + image support
- **New Module: `usda_client.rs`** — USDA FoodData Central API client (optional, requires USDA_API_KEY env var)
- **New Module: `verified.rs`** — Complete verified food item pipeline
- **Refactored**: `get_fuzzy_suggestion` now uses shared `AnthropicClient` instead of inline HTTP code
- **Files Modified**:
  - `src/db/migrations.rs` - Migration v10
  - `src/models/food_item.rs` - source fields, FTS5 AND semantics, concatenated LIKE
  - `src/tools/food_items.rs` - source fields, audit_food_items, refactored fuzzy suggestion
  - `src/tools/mod.rs` - New module declarations
  - `src/mcp/server.rs` - source fields on params, add_food_item_verified + audit_food_items handlers
  - `Cargo.toml` - Added urlencoding dependency
- **New Files**:
  - `src/tools/ai_client.rs` - Shared Anthropic API client
  - `src/tools/usda_client.rs` - USDA FoodData Central client
  - `src/tools/verified.rs` - Verified food item pipeline

### Phase 27: BP Time-of-Day Chart Page
- **Purpose**: Visualize circadian BP patterns by appending a time-of-day analysis page to the existing BP report
- **Problem Solved**: Feb 2026 data showed clear pattern (post-exercise afternoon avg ~112/60 vs evening ~148/77) that wasn't visible in daily trend charts
- **No API Changes**: Same `generate_bp_report` signature — Page 3 auto-appended when data spans 2+ time windows
- **Page 3 Layout** (Landscape):
  - Header: "Blood Pressure by Time of Day" with patient name, reading count, window count
  - Line chart (plotters): red systolic + blue diastolic lines with data point markers
  - 8 three-hour time buckets (12a-3a through 9p-12a), bucketed by `hour / 3`
  - Reference lines: 120 mmHg (orange, normal SYS ceiling), 140 mmHg (red, Stage 1 HTN), 80 mmHg (blue, normal DIA ceiling)
  - Gaps in chart lines for empty time windows
  - Reference line legend (color-coded)
  - Stats table: 2 columns x 4 rows, color-coded values (green < 120, amber 120-139, red >= 140 for systolic; green < 80, amber 80-89, red >= 90 for diastolic), em-dash for empty windows
- **New Functions**:
  - `aggregate_time_of_day_bp()` — buckets vitals into 8 windows, returns per-bucket averages and counts
  - `generate_bp_time_of_day_chart()` — plotters BitMapBackend chart with reference lines, connected line segments, legend
  - `TimeBucketStats` struct — label, systolic_avg, diastolic_avg, count per window
- **Files Modified**:
  - `src/tools/reports.rs` - New functions + Page 3 insertion in generate_bp_report()

### Phase 29: Weight Watchers Points Integration
- **Purpose**: Add WW SmartPoints as a supplemental food quality metric alongside existing calorie/macro tracking
- **Database Schema** (Migration v12):
  - `food_items`: `ww_points` (f64, nullable), `ww_zero_point` (bool, default false)
  - `meal_entries`: `cached_ww_points` (f64, default 0)
  - `exercises`: `ww_activity_points` (f64, default 0)
  - `days`: `cached_ww_points_gross`, `cached_ww_exercise_credit`, `cached_ww_points_net` (all f64, default 0)
- **SmartPoints Formula**: `points = (cal × 0.0305) + (sat_fat × 0.275) + (sugar × 0.12) − (protein × 0.098)`, rounded to nearest integer, floor at 0
- **ZeroPoint Foods**: Auto-detected by name pattern on creation. Categories: skinless/boneless poultry, eggs, fish/shellfish, non-starchy vegetables, potatoes, fruits, beans/legumes, non-fat yogurt, tofu, frozen vegetable blends, broths/stocks. When `ww_zero_point = true`, item always scores 0 regardless of formula.
- **Meal Entry WW**: If source food is ZeroPoint → 0 pts. Otherwise, `food_item.ww_points × servings × (percent_eaten / 100)`. Recipe-based entries use formula on cached nutrition.
- **Exercise Credit**: `weight_lbs × duration_min × 0.00047`, capped at 6 points per session. Stored as `ww_activity_points` on exercises table.
- **Day Aggregation**: `cached_ww_points_gross` = sum of meal entry WW points. `cached_ww_exercise_credit` = sum of exercise WW activity points. `cached_ww_points_net` = gross − exercise credit.
- **Tier System** (daily budget: 35 pts):
  | Threshold | Tier |
  |-----------|------|
  | ≤25 pts net + ≥140g protein | MEGA Win |
  | ≤30 pts net | Super Win |
  | ≤35 pts net | Win |
  | 36-40 pts net | Watch Zone |
  | >40 pts net | Red Alert |
- **Reporting**: WW points appear as first fields in `get_day` and `list_days` output. `DayDetail` includes `ww_points_net`, `ww_points_gross`, `ww_exercise_credit`, `ww_budget` (35), `ww_tier`. `DaySummary` includes `ww_points_net`, `ww_tier`. `MealEntryDetail` includes `ww_points`. `DayExerciseSummary` includes `ww_activity_points`.
- **Auto-calculation**: WW points auto-calculated on food item create/update. Recalculated when nutrition fields change.
- **Backfill**: Migration v12 calculates WW points for all existing food items, flags ZeroPoint foods by name pattern, and recalculates March 2026 meal entries and day totals.
- **Files Modified**:
  - `src/db/migrations.rs` - Migration v12 with schema + backfill
  - `src/models/food_item.rs` - `ww_points`, `ww_zero_point`, `calculate_ww_points()`, auto-calc on create/update
  - `src/models/meal_entry.rs` - `cached_ww_points`, threaded through create/create_direct/refresh/recalculate
  - `src/models/exercise.rs` - `ww_activity_points` in recalculate_totals and recalculate_day_exercise_calories
  - `src/tools/days.rs` - WW fields on DayDetail, DaySummary, DayExerciseSummary, ww_tier() helper
  - `src/mcp/server.rs` - `ww_zero_point` on FoodItemCreate/Update param structs
  - `src/tools/verified.rs` - `ww_zero_point` on verified create path

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
- **Auto-cleanup**: Import tools now automatically clean up duplicates - keeps exercise-linked (with context), removes standalone duplicates

### Standard References for Health Data
- **MET Values**: Always use the Compendium of Physical Activities for standardized MET values
- **Why it matters**: Self-estimated values can drift from standards; using authoritative sources ensures consistency
- **Resource**: Compendium of Physical Activities (sites.google.com/site/compendiumofphysicalactivities/)

### Model Field Updateability
- **Principle**: All fields in a model should be updateable, not just "notes"
- **Reasoning**: Users make mistakes - wrong timestamps, incorrect values, missing links
- **Implementation**: Include all relevant fields in Update structs, even if rarely changed
- **Example**: Adding `timestamp` to ExerciseUpdate and VitalUpdate allows correcting entry times

### SQLite FTS5 Full-Text Search
- **When to Use**: When users search with multiple words that may not appear consecutively
- **Key Feature**: Word tokenization and prefix matching (e.g., "burrito shell" → "burrito* OR shell*")
- **Setup**: Create virtual table with `USING fts5(column1, column2, content='source_table', content_rowid='id')`
- **Sync Triggers**: AFTER INSERT/UPDATE/DELETE triggers keep FTS index in sync with source table
- **Query Syntax**: Use MATCH with OR for multi-word queries, add * for prefix matching
- **Ranking**: ORDER BY rank gives relevance-sorted results
- **Limitation**: FTS5 doesn't handle synonyms - use AI fallback for semantic matching

### Search Strategy Hierarchy
- **Principle**: Layer search methods from fast/cheap to slow/expensive
- **Pattern**: FTS5 (instant, no API) → LIKE fallback → AI fuzzy matching (API call)
- **Why**: Most queries succeed with FTS5, reducing unnecessary API calls
- **Implementation**: Try each method in order, only proceed to next if no results

### Fuzzy Matching with AI
- **Include Context**: Send brand + name to AI, not just name (e.g., "King Oscar - Atlantic Salmon")
- **Validation**: Use case-insensitive matching when comparing AI response to actual items
- **Fallbacks**: Try exact match → name-only match → contains match
- **Timeouts**: Set reasonable timeout (10s) to avoid hanging on API issues
- **Graceful Degradation**: Return None if AI unavailable rather than failing the entire search

### Recipe-Free Architecture
- **Principle**: Prefer direct data entry over intermediate abstractions when the abstraction adds overhead without value
- **Example**: Recipes added unnecessary steps for simple meals - now users log food items directly with quantities
- **Compound Items**: For reusable combinations, create a single food item with combined nutrition + recipe in notes
- **Tool Consolidation**: Batch operations (search_food_items_batch, log_meal_items_batch) reduce round-trips

### Python Script Generation Pattern (for Complex Visualizations)
- **When to Use**: When Rust-native charting (plotters + printpdf) is insufficient for complex visualizations (dark themes, spline interpolation, fill-between areas)
- **Pattern**: Rust collects data → builds Python script string with data as literals → writes to temp file → shells out to `python` → captures output → cleans up
- **Benefits**: Leverages matplotlib's full charting power without adding a Python dependency at compile time
- **Data Embedding**: Embed data as Python list/dict literals (no file I/O needed in the script)
- **String Escaping**: Always escape backslashes and quotes when embedding strings in Python literals
- **Error Handling**: Capture both stdout and stderr from Python process; return meaningful error if Python not found
- **Cleanup**: Temp script + chart PNGs cleaned up by the Python script itself before exit
- **Dependencies**: Ensure Python packages are installed (`pip install matplotlib reportlab numpy scipy`)
- **Gotcha on Windows**: pip may fail with TLS cert errors; use `--trusted-host pypi.org --trusted-host pypi.python.org --trusted-host files.pythonhosted.org` flags

### Post-Exercise Vital Group Split Pattern
- **Problem**: Post-exercise BP readings may be split across multiple vital groups (1st reading in group N at ~4 min, 2nd reading in group N+1 at ~12 min), but exercise only links to one group via `post_vital_group_id`
- **Solution**: Don't rely solely on the linked group. Query ALL vital groups with timestamps in the post-exercise window (0–20 min after exercise end), collect ALL vitals from ALL matching groups, then sort and pair.
- **Key Insight**: Calculate exercise end time = `timestamp + cached_duration_minutes`, then filter vitals to the window [end, end+20min]
- **Edge Case**: Include a -1 min tolerance to catch readings taken just before the official end time
- **Pairing Logic**: Sort BP readings by minute offset, take first two as early/late pair

### Adaptive Chart Mode Selection
- **Principle**: Chart rendering style should adapt to the data density
- **Pattern**: Use bar charts for small date ranges (≤31 days) where individual sessions are visible, switch to smooth line charts for larger ranges (>31 days) to show trends
- **Avoid Middle Tiers**: A 3-tier system (bars/line_marks/line_only) adds complexity without clear user value; 2 tiers (bars/lines) is simpler and sufficient

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
│   │   ├── food_items.rs   # Food item tool functions + audit
│   │   ├── recipes.rs      # Recipe tool functions
│   │   ├── days.rs         # Day and meal entry tool functions + stats
│   │   ├── medications.rs  # Medication tool functions
│   │   ├── vitals.rs       # Vital tool functions + stats
│   │   ├── exercise.rs     # Exercise tool functions + stats
│   │   ├── reports.rs      # PDF/markdown report generation (BP, HR, Weight, Exercise, Day Summary, Medications)
│   │   ├── ai_client.rs    # Shared Anthropic API client (Sonnet/Haiku)
│   │   ├── usda_client.rs  # USDA FoodData Central API client
│   │   └── verified.rs     # Verified food item creation pipeline
│   ├── nutrition/
│   │   ├── mod.rs
│   │   ├── units.rs        # Unit types, categories, conversion constants
│   │   └── converter.rs    # Unit parsing and conversion functions
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
- [x] Food item import from external databases (USDA) — Phase 28: USDA FoodData Central integrated into verified pipeline

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
