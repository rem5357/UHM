//! UHM Status Tool
//!
//! Provides runtime status information about the UHM service.

use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::build_info::BuildInfo;

/// Meal logging instructions for AI assistants
pub const MEAL_INSTRUCTIONS: &str = r#"
# UHM Meal Logging Instructions (Recipe-Free Workflow)

This guide explains the streamlined meal logging workflow using direct food item logging.

## Overview

The recipe-free workflow is optimized for efficiency:
1. **Search** for food items using `search_food_items_batch` (one call for all foods)
2. **Log** all items using `log_meal_items_batch` (one call per meal)

This reduces tool calls by ~50% compared to the recipe-based workflow.

---

## Getting the Current Date

**IMPORTANT:** When logging meals for "today" or any relative date, use the UCM (Universal Calendar Manager) MCP server to get accurate dates.

**Tool:** `ucm_now`
- Returns the current date and time in ISO format
- Use the `date` field (YYYY-MM-DD) for meal logging

---

## Quick Start: Logging a Meal

### Step 1: Search for All Foods at Once

```
search_food_items_batch(
  queries: ["chicken breast", "rice", "broccoli"],
  fuzzy_match: true,  // Get AI suggestions for unmatched items
  limit_per_query: 5
)
```

Returns FULL nutrition data for all matches - no need for follow-up `get_food_item` calls.

### Step 2: Log All Items in One Call

```
log_meal_items_batch(
  date: "2026-01-13",
  meal_type: "dinner",
  items: [
    { food_item_id: 15, quantity: 150, unit: "g" },      // 150g chicken
    { food_item_id: 23, quantity: 200, unit: "g" },      // 200g rice
    { food_item_id: 45, quantity: 100, unit: "g" },      // 100g broccoli
    { food_item_id: 12, quantity: 2, unit: "count" }     // 2 eggs
  ]
)
```

Returns per-item nutrition and day totals.

---

## Unit Standards

### Food Items (stored in database)
| Category | serving_size | serving_unit |
|----------|--------------|--------------|
| Solids | 100 | g |
| Liquids | 100 | ml |
| Countables | 1 | count |

### Logging Units (what you pass to log_meal_items_batch)
| Unit | Use For | Example |
|------|---------|---------|
| `g` | Solids (meat, grains, vegetables) | 150g chicken |
| `ml` | Liquids (milk, oil, juice) | 240ml oat milk |
| `count` | Countable items (eggs, bananas) | 2 eggs |

### Unit Conversion Examples

| Food Item | Stored As | Logging 150g = |
|-----------|-----------|----------------|
| Chicken (100g = 165 cal) | per 100g | 165 × 1.5 = 248 cal |
| Rice (100g = 130 cal) | per 100g | 130 × 1.5 = 195 cal |
| Egg (1 count = 72 cal) | per 1 count | 72 × 2 = 144 cal (for 2 eggs) |

---

## The 80% Rule for Nutrition Ranges

When a source provides a range:
**Final Value = Low Value + (80% × Difference)**

Example: "500-600 calories" → 500 + (0.80 × 100) = **580 cal**

---

## Partial Consumption

Use `percent_eaten` for when you don't finish everything:

```
log_meal_items_batch(
  date: "2026-01-13",
  meal_type: "dinner",
  items: [
    { food_item_id: 15, quantity: 150, unit: "g", percent_eaten: 75 }  // ate 75%
  ]
)
```

---

## Adding New Food Items

If a food doesn't exist, add it first:

```
add_food_item(
  name: "Chicken Breast (grilled)",
  serving_size: 100,
  serving_unit: "g",
  calories: 165,
  protein: 31,
  carbs: 0,
  fat: 3.6,
  ...
)
```

**Converting Package Labels:**
- Package: 190 cal per 2 tbsp (32g)
- Per 100g: (190 / 32) × 100 = 594 cal

---

## Creating Compound Food Items (Reusable Combinations)

For items logged repeatedly (weekly or more), create a **compound food item** instead of logging individual components each time.

### When to Create
- DIYOO variations, protein coffees, smoothie bases
- Any combination with consistent ingredients you'll log multiple times

### Workflow

**Step 1: Calculate total nutrition from components**

Either search existing food items or look up nutrition data:
```
Morning Protein Coffee components:
- 240ml brewed coffee: 2 cal, 0g protein
- 31g ON Whey Vanilla: 120 cal, 24g protein, 50mg sodium
- 60ml oat milk: 28 cal, 0.5g protein, 25mg sodium
─────────────────────────────────────────────────
Total: 150 cal, 24.5g protein, 75mg sodium
```

**Step 2: Create compound food item**

```
add_food_item(
  name: "Morning Protein Coffee (ON Vanilla)",
  serving_size: 1,
  serving_unit: "count",
  calories: 150,
  protein: 24.5,
  carbs: 5,
  fat: 2.2,
  sodium: 75,
  ...
  notes: "RECIPE: 240ml coffee (2 cal) + 31g ON Whey (120 cal, 24g P) + 60ml oat milk (28 cal)"
)
```

**Step 3: Log as single item**

```
log_meal_items_batch(
  date: "2026-02-03",
  meal_type: "breakfast",
  items: [{ food_item_id: 201, quantity: 1, unit: "count" }]
)
```

### Notes Field Format

Include enough detail to recreate or modify later:
```
RECIPE: [qty] [ingredient] ([key nutrition]) + [qty] [ingredient] + ...
Total: [cal] cal, [protein]g protein.
[Optional prep notes]
```

### Existing Compound Food Items
| ID | Name | Cal | Protein |
|----|------|-----|---------|
| 199 | DIYOO Base Mix (ON Vanilla) | 290 | 25.6g |
| 200 | DIYOO Base Mix (Naked Whey) | 290 | 30.6g |

---

## Common Conversions Reference

| Item | 1 cup = | 1 tbsp = |
|------|---------|----------|
| Rolled oats | 80g | 5g |
| Flour | 120g | 7.5g |
| Rice (cooked) | 185g | 12g |
| Milk/water | 240ml | 15ml |
| Protein powder | 93g (~3 scoops) | 6g |

---

## Quick Reference

| Task | Tool |
|------|------|
| Search multiple foods | `search_food_items_batch` |
| Log meal items | `log_meal_items_batch` |
| Add new food item | `add_food_item` |
| View food item details | `get_food_item` |
| View day's meals | `get_day` |
| Get nutrition statistics | `list_days_stats` |
| Update meal entry | `update_meal_entry` |
| Delete meal entry | `delete_meal_entry` |

## Typical Workflow

1. User: "Log my lunch: 6oz chicken, cup of rice, salad"
2. Claude: `search_food_items_batch(["chicken breast", "rice cooked", "mixed salad"])`
3. Claude: Convert quantities (6oz = 170g, 1 cup rice = 185g)
4. Claude: `log_meal_items_batch(date, "lunch", [items with quantities])`
5. Done! One search + one log = 2 tool calls total.

## Notes

- Dates use ISO format: YYYY-MM-DD
- Days are created automatically when logging meals
- Nutrition calculations happen automatically based on quantity/unit
- Daily totals update automatically after logging
"#;

/// Medication management instructions for AI assistants
pub const MEDICATION_INSTRUCTIONS: &str = r#"
# UHM Medication Management Instructions

This guide explains how to manage medications using the Universal Health Manager (UHM) tools.

## Overview

The medication system tracks:
- **Prescriptions** - Rx medications from doctors
- **Supplements** - Vitamins, minerals, etc.
- **OTC** - Over-the-counter medications
- **Natural remedies** - Herbal, homeopathic
- **Compounds** - Compounded medications
- **Medical devices** - Inhalers, insulin pens, etc.
- **Other** - Anything else

## Key Concepts

### Active vs Deprecated Medications
- Medications have an `is_active` flag
- **Active** = currently taking
- **Deprecated** = no longer taking (but preserved for history)
- **Deprecate, don't delete** - preserves medication history

### Force Flag Requirement
- `update_medication` and `delete_medication` require `force=true`
- This prevents accidental modifications
- For dosage changes: **deprecate old + add new** (recommended)

## Step-by-Step Workflows

### Adding a New Prescription

```
add_medication(
  name: "Lisinopril",
  med_type: "prescription",
  dosage_amount: 10,
  dosage_unit: "mg",
  instructions: "Take 1 tablet daily in the morning",
  frequency: "once daily",
  prescribing_doctor: "Dr. Smith",
  prescribed_date: "2026-01-10",
  pharmacy: "CVS Pharmacy",
  rx_number: "RX12345678",
  refills_remaining: 5,
  start_date: "2026-01-10",
  notes: "For blood pressure management"
)
```

### Adding a Supplement

```
add_medication(
  name: "Vitamin D3",
  med_type: "supplement",
  dosage_amount: 5000,
  dosage_unit: "iu",
  instructions: "Take 1 capsule with food",
  frequency: "once daily",
  start_date: "2026-01-01"
)
```

### Adding an OTC Medication

```
add_medication(
  name: "Ibuprofen",
  med_type: "otc",
  dosage_amount: 200,
  dosage_unit: "mg",
  instructions: "Take 1-2 tablets as needed for pain",
  frequency: "PRN",
  notes: "Max 6 tablets per day"
)
```

### Changing a Dosage (Recommended Approach)

When a doctor changes your dosage, preserve history by:

1. **Deprecate the old medication:**
```
deprecate_medication(
  id: 5,
  end_date: "2026-01-15",
  reason: "Dosage increased by Dr. Smith"
)
```

2. **Add the new dosage:**
```
add_medication(
  name: "Lisinopril",
  med_type: "prescription",
  dosage_amount: 20,  // new dosage
  dosage_unit: "mg",
  instructions: "Take 1 tablet daily in the morning",
  frequency: "once daily",
  prescribing_doctor: "Dr. Smith",
  prescribed_date: "2026-01-15",
  pharmacy: "CVS Pharmacy",
  rx_number: "RX12345679",
  refills_remaining: 5,
  start_date: "2026-01-15",
  notes: "Increased from 10mg"
)
```

### Stopping a Medication

```
deprecate_medication(
  id: 3,
  end_date: "2026-01-20",
  reason: "Completed course of treatment"
)
```

### Restarting a Medication

```
reactivate_medication(id: 3)
```

### Generating a Medication List

```
export_medications_markdown(patient_name: "John Smith")
```

Returns a formatted markdown document with:
- Patient name and current date/time
- Medications grouped by type (prescriptions first)
- Full details: dosage, frequency, doctor, pharmacy, instructions

## Quick Reference

| Task | Tool |
|------|------|
| Add medication | `add_medication` |
| View medication details | `get_medication` |
| List active medications | `list_medications` |
| Search by name | `search_medications` |
| Stop taking (preserve history) | `deprecate_medication` |
| Restart taking | `reactivate_medication` |
| Update (requires force) | `update_medication` |
| Delete (requires force) | `delete_medication` |
| Generate med list doc | `export_medications_markdown` |

## Medication Types

| Type | Use For |
|------|---------|
| `prescription` | Doctor-prescribed Rx medications |
| `supplement` | Vitamins, minerals, fish oil, probiotics |
| `otc` | Over-the-counter (Tylenol, Advil, Benadryl) |
| `natural` | Herbal remedies, homeopathic |
| `compound` | Compounded medications from specialty pharmacy |
| `medical_device` | Inhalers, insulin pens, nebulizers |
| `other` | Anything else |

## Dosage Units

| Unit | Common Use |
|------|------------|
| `mg` | Most tablets/capsules |
| `mcg` | Thyroid meds, some vitamins |
| `g` | Large doses, powders |
| `ml` | Liquids |
| `fl_oz` | Liquid medications |
| `tablet` | Whole tablets |
| `capsule` | Capsules |
| `pill` | Generic pill form |
| `spray` | Nasal sprays |
| `puff` | Inhalers |
| `drop` | Eye/ear drops |
| `patch` | Transdermal patches |
| `injection` | Injectable medications |
| `iu` | International units (vitamins) |
| `unit` | Insulin units |

## Frequency Examples

- `once daily` - Take once per day
- `twice daily` - Take twice per day
- `three times daily` - Take three times per day
- `every 8 hours` - Take every 8 hours
- `weekly` - Take once per week
- `PRN` - As needed
- `at bedtime` - Take before sleep
- `with meals` - Take with food

## Best Practices

1. **Always include instructions** - Helps remember how to take
2. **Record prescribing doctor** - For prescriptions
3. **Track refills** - Know when to request refill
4. **Use start_date** - Know how long you've been taking it
5. **Deprecate, don't delete** - Preserve medication history
6. **New dosage = new entry** - Deprecate old, add new

## Listing Medications

**Active only (default):**
```
list_medications()
```

**All medications including inactive:**
```
list_medications(active_only: false)
```

**Filter by type:**
```
list_medications(med_type: "prescription")
list_medications(med_type: "supplement")
```

## Notes

- Dates use ISO format: YYYY-MM-DD
- `export_medications_markdown` only includes active medications
- Deprecated medications are hidden by default but preserved
- The force flag protects against accidental changes
"#;

/// Vital tracking instructions for AI assistants
pub const VITAL_INSTRUCTIONS: &str = r#"
# UHM Vital Sign Tracking Instructions

This guide explains how to track vital signs using the Universal Health Manager (UHM) tools.

## Overview

The vitals system tracks:
- **Weight** - Body weight in lbs or kg
- **Blood Pressure** - Systolic/diastolic in mmHg
- **Heart Rate** - Beats per minute (bpm)
- **Oxygen Saturation** - SpO2 percentage
- **Glucose** - Blood sugar in mg/dL

## Key Concepts

### Vital Groups
Vital groups link related readings taken at the same time. For example:
- Blood pressure + heart rate from the same measurement session
- Post-exercise readings (BP, HR, O2)
- Morning vitals check (weight, BP, HR)

Groups have a description (e.g., "BP & HR reading", "Post Exercise") and can contain any combination of vital types.

### Standalone Vitals
Vitals can be recorded without a group for quick, individual readings (e.g., a quick weight check).

## Step-by-Step Workflows

### Recording a Single Vital

```
add_vital(
  vital_type: "weight",
  value1: 185.5,
  unit: "lbs"
)
```

### Recording Related Vitals (BP + HR)

**Step 1: Create a group**
```
create_vital_group(
  description: "Morning BP & HR"
)
```
Returns the group ID.

**Step 2: Add vitals to the group**
```
add_vital(
  vital_type: "blood_pressure",
  value1: 120,           // systolic
  value2: 80,            // diastolic (required for BP)
  group_id: 1
)

add_vital(
  vital_type: "heart_rate",
  value1: 72,
  group_id: 1
)
```

**Step 3: View the group**
```
get_vital_group(id: 1)
```
Returns the group with all linked vitals.

### Linking Existing Vitals to a Group

If you've already recorded vitals separately:
```
assign_vital_to_group(vital_id: 5, group_id: 1)
assign_vital_to_group(vital_id: 6, group_id: 1)
```

### Removing a Vital from a Group

```
assign_vital_to_group(vital_id: 5, group_id: null)
```
The vital remains but is no longer linked to any group.

## Vital Types and Values

| Type | value1 | value2 | Default Unit |
|------|--------|--------|--------------|
| weight | Weight | - | lbs |
| blood_pressure | Systolic | Diastolic (required) | mmHg |
| heart_rate | BPM | - | bpm |
| oxygen_saturation | SpO2 % | - | % |
| glucose | mg/dL | - | mg/dL |

### Type Aliases
You can use these shortcuts when specifying vital_type:
- `bp` = blood_pressure
- `hr` or `pulse` = heart_rate
- `o2` or `spo2` = oxygen_saturation

## Quick Reference

| Task | Tool |
|------|------|
| Record a vital | `add_vital` |
| Get vital details | `get_vital` |
| Update a vital | `update_vital` |
| Delete a vital | `delete_vital` |
| List by type | `list_vitals_by_type` |
| List recent | `list_recent_vitals` |
| List by date range | `list_vitals_by_date_range` |
| Get latest of each type | `get_latest_vitals` |
| Get statistics by type | `list_vitals_stats` |
| **Batch import weights** | `add_weights_batch` |
| Import weights from CSV | `import_weight_csv` |
| Create group | `create_vital_group` |
| View group with vitals | `get_vital_group` |
| List groups | `list_vital_groups` |
| Update group | `update_vital_group` |
| Delete group | `delete_vital_group` |
| Link vital to group | `assign_vital_to_group` |

## Common Scenarios

### Quick Weight Check
```
add_vital(vital_type: "weight", value1: 182.3)
```

### Full Blood Pressure Reading (with HR)
1. `create_vital_group(description: "BP & HR")`
2. `add_vital(vital_type: "bp", value1: 118, value2: 76, group_id: 1)`
3. `add_vital(vital_type: "hr", value1: 68, group_id: 1)`

### Post-Exercise Vitals
1. `create_vital_group(description: "Post-Exercise", notes: "After 30 min run")`
2. `add_vital(vital_type: "bp", value1: 135, value2: 85, group_id: 2)`
3. `add_vital(vital_type: "hr", value1: 125, group_id: 2)`
4. `add_vital(vital_type: "o2", value1: 96, group_id: 2)`

### View Weight History
```
list_vitals_by_type(vital_type: "weight", limit: 30)
```

### Check Latest Readings
```
get_latest_vitals()
```
Returns the most recent reading for each vital type.

### Find Vitals in a Date Range
```
list_vitals_by_date_range(
  start_date: "2026-01-01",
  end_date: "2026-01-31",
  vital_type: "blood_pressure"  // optional filter
)
```

### Analyze Vital Trends with Statistics
Use `list_vitals_stats` to get comprehensive statistics for any vital type:
```
list_vitals_stats(
  vital_type: "blood_pressure",
  start_date: "2026-01-01",  // optional
  end_date: "2026-01-31"     // optional
)
```

**Statistics returned for all vital types:**
- **count** - Number of readings
- **average** - Mean value
- **median** - Middle value (less affected by outliers)
- **mode** - Most common value
- **standard_deviation** - Variability measure
- **min / max / range** - Lowest, highest, and spread
- **percentile_25 / percentile_75 / iqr** - Quartiles and interquartile range
- **coefficient_of_variation** - Relative variability (SD/mean × 100)
- **outliers** - Readings outside 1 standard deviation (with timestamp, value, z-score)

**Type-specific extras:**

| Vital Type | Additional Stats |
|------------|-----------------|
| **weight** | total_change, avg_change_per_reading |
| **blood_pressure** | Separate stats for systolic, diastolic, and pulse_pressure |
| **oxygen_saturation** | below_95_count, below_90_count (concerning readings) |
| **glucose** | low_count (<70), high_count (>180) |

**Example use cases:**
- "What's my average blood pressure this month?"
- "Which readings were unusually high or low?"
- "How much has my weight changed?"
- "How consistent is my heart rate?"

**Why use this?** Much faster than fetching raw data and calculating in Claude Desktop. A single tool call returns all statistics instantly.

### Batch Weight Import

For importing multiple historical weight entries (e.g., from logs or documents), use `add_weights_batch`:

```
add_weights_batch(entries: [
  {date: "2025-01-15", value: 185.5},
  {date: "2025-01-16", value: 185.2, unit: "lbs"},
  {date: "1/17/2025", value: 184.8}
])
```

**Features:**
- Accepts array of `{date, value, unit?}` objects
- Date formats: `YYYY-MM-DD`, `MM/DD/YYYY`, `MM-DD-YYYY`
- Unit defaults to "lbs" if omitted
- Skips duplicates (same date + value already exists)
- Returns per-entry status: "added", "duplicate", or "error"

**Use case:** User provides weight history in markdown/documents. Extract dates and values, then call `add_weights_batch` once with all entries instead of calling `add_vital` for each weight.

For CSV files, use `import_weight_csv(file_path: "path/to/weights.csv")` with format:
```
date,value,unit
2025-01-15,185.5,lbs
2025-01-16,185.2,lbs
```

## Notes

- Timestamps default to current time if not provided
- Blood pressure requires both value1 (systolic) and value2 (diastolic)
- Deleting a group unlinks vitals but doesn't delete them
- Use unit parameter to override default (e.g., "kg" instead of "lbs" for weight)
- Vitals can be added to a group at creation time or linked later
"#;

/// Exercise tracking instructions for AI assistants
pub const EXERCISE_INSTRUCTIONS: &str = r#"
# UHM Exercise Tracking Instructions

This guide explains how to track exercise using the Universal Health Manager (UHM) tools.

## Overview

The exercise system tracks **treadmill workouts** (expandable to other types in the future). Each exercise session:
- Belongs to a **Day** (same as meal logging)
- Contains one or more **Segments** (e.g., 15 min at 2.3 mph, then 15 min at 2.5 mph)
- Automatically calculates **calories burned** using current weight from vitals
- Can link to **PRE and POST vital groups** for recovery tracking

## Key Concepts

### Exercise Sessions
An exercise session represents a single workout (e.g., one trip to the gym or one treadmill session).

### Segments
Each session can have multiple segments with different settings:
- **Duration** (minutes)
- **Speed** (mph)
- **Distance** (miles)
- **Incline** (percent)

**Important:** You must provide at least 2 of 3 values (duration, speed, distance). The system calculates the third automatically.

### Automatic Calculations
- **Third value:** Given 2 of (duration, speed, distance), the 3rd is calculated
- **Consistency check:** If all 3 provided, system verifies they match (distance = speed × time)
- **Calories burned:** Uses MET formula with latest weight from vitals database
- **Exercise totals:** Cached totals (duration, distance, calories) update automatically when segments change
- **Day totals:** Day's `cached_calories_burned` updates automatically

### Calorie Calculation
Uses the MET (Metabolic Equivalent of Task) formula:
```
Calories = MET × weight_kg × duration_hours
```

MET values vary by speed:
| Speed (mph) | MET (approximate) |
|-------------|-------------------|
| 2.0 | 2.0 |
| 2.5 | 2.5 |
| 3.0 | 3.0 |
| 3.5 | 3.5 |
| 4.0 | 4.3 |
| 5.0 | 6.0 |
| 6.0 | 9.0 |

Incline adds ~0.1 MET per 1% grade.

## Step-by-Step Workflow

### 1. Create an Exercise Session

```
add_exercise(
  date: "2026-01-14",
  exercise_type: "treadmill",
  notes: "Morning workout"
)
```

Returns exercise ID to use for adding segments.

### 2. Add Segments

For each part of the workout with different settings:

```
add_exercise_segment(
  exercise_id: 1,
  duration_minutes: 15,
  speed_mph: 2.3,
  incline_percent: 0,
  notes: "Warmup"
)

add_exercise_segment(
  exercise_id: 1,
  duration_minutes: 15,
  speed_mph: 2.5,
  incline_percent: 2,
  avg_heart_rate: 83,
  notes: "Main workout"
)
```

Each segment returns:
- Calculated distance (if not provided)
- Calories burned for that segment
- Updated exercise totals

### 3. Link Vital Groups (Optional)

For tracking recovery, link PRE and POST vital readings:

**Before exercise:**
```
create_vital_group(description: "Pre-exercise BP/HR")
add_vital(vital_type: "bp", value1: 118, value2: 76, group_id: 1)
add_vital(vital_type: "hr", value1: 68, group_id: 1)
```

**After exercise:**
```
create_vital_group(description: "Post-exercise BP/HR")
add_vital(vital_type: "bp", value1: 135, value2: 85, group_id: 2)
add_vital(vital_type: "hr", value1: 95, group_id: 2)
```

**Link to exercise:**
```
update_exercise(
  id: 1,
  pre_vital_group_id: 1,
  post_vital_group_id: 2
)
```

### 4. View Exercise Details

```
get_exercise(id: 1)
```

Returns full details including:
- All segments with metrics
- Total duration, distance, calories
- Linked vital groups

## Quick Reference

| Task | Tool |
|------|------|
| Create exercise session | `add_exercise` |
| Add segment to exercise | `add_exercise_segment` |
| View exercise with segments | `get_exercise` |
| List exercises | `list_exercises` |
| List exercises for a day | `list_exercises_for_day` |
| Update exercise (notes, vital links) | `update_exercise` |
| Update segment | `update_exercise_segment` |
| Delete segment | `delete_exercise_segment` |
| Delete exercise | `delete_exercise` |
| Get exercise statistics | `list_exercise_stats` |

## Common Scenarios

### Quick Single-Segment Workout
```
add_exercise(date: "2026-01-14", exercise_type: "treadmill")
add_exercise_segment(exercise_id: 1, duration_minutes: 30, speed_mph: 3.0)
```

### Multi-Segment Interval Training
```
add_exercise(date: "2026-01-14", exercise_type: "treadmill", notes: "Interval training")
add_exercise_segment(exercise_id: 1, duration_minutes: 5, speed_mph: 2.5, notes: "Warmup")
add_exercise_segment(exercise_id: 1, duration_minutes: 2, speed_mph: 4.0, incline_percent: 1)
add_exercise_segment(exercise_id: 1, duration_minutes: 3, speed_mph: 3.0)
add_exercise_segment(exercise_id: 1, duration_minutes: 2, speed_mph: 4.0, incline_percent: 1)
add_exercise_segment(exercise_id: 1, duration_minutes: 3, speed_mph: 3.0)
add_exercise_segment(exercise_id: 1, duration_minutes: 5, speed_mph: 2.5, notes: "Cooldown")
```

### Analyze Exercise Trends
```
list_exercise_stats(start_date: "2026-01-01", end_date: "2026-01-31")
```

Returns statistics for:
- Duration (total workout time)
- Distance (miles covered)
- Calories burned
- Speed (average across segments)
- Incline (average across segments)

Each includes: count, average, median, mode, SD, min, max, percentiles, outliers.

## Input Validation

### Segment Requirements
- **Must provide 2 of 3:** duration_minutes, speed_mph, distance_miles
- System calculates the missing value
- If all 3 provided, system checks consistency (1% tolerance)

### Consistency Flag
If values don't match (e.g., 30 min at 3 mph but distance says 2 miles instead of 1.5):
- `is_consistent: false` in response
- Values are still recorded as provided
- Claude should alert user to the inconsistency

## Integration with Days

Exercise calories appear in day reports:
- `get_day` includes `cached_calories_burned` field
- Net calories = `cached_calories` (consumed) - `cached_calories_burned` (exercise)
- Day totals update automatically when exercises change

## Weight Tracking

- System uses latest weight from vitals for calorie calculation
- If no weight recorded, defaults to 150 lbs
- Each segment records `weight_used_lbs` for reference
- Update weight vitals regularly for accurate calorie calculations

## Notes

- Dates use ISO format: YYYY-MM-DD
- Exercise timestamps default to current time if not provided
- Deleting an exercise deletes all its segments (cascade)
- Segment order is assigned automatically (1, 2, 3, ...)
- Vital groups can be created before or after exercise, then linked
"#;

/// Runtime status of the UHM service
#[derive(Debug, Clone, Serialize)]
pub struct UhmStatus {
    /// Build information
    pub build_number: u64,
    pub build_timestamp: &'static str,
    pub version: &'static str,

    /// Database information
    pub database_path: String,
    pub database_size_bytes: Option<u64>,

    /// Process information
    pub uptime_seconds: u64,
    pub process_id: u32,
    pub memory_usage_bytes: u64,
}

/// Status tracker for collecting runtime information
pub struct StatusTracker {
    start_time: Instant,
    database_path: PathBuf,
}

impl StatusTracker {
    /// Create a new status tracker
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            start_time: Instant::now(),
            database_path,
        }
    }

    /// Get the current status
    pub fn get_status(&self) -> UhmStatus {
        let build_info = BuildInfo::current();

        // Get database size if it exists
        let database_size_bytes = std::fs::metadata(&self.database_path)
            .ok()
            .map(|m| m.len());

        // Get process info
        let pid = std::process::id();
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));

        let memory_usage_bytes = sys
            .process(Pid::from_u32(pid))
            .map(|p| p.memory())
            .unwrap_or(0);

        UhmStatus {
            build_number: build_info.build_number,
            build_timestamp: build_info.build_timestamp,
            version: build_info.version,
            database_path: self.database_path.display().to_string(),
            database_size_bytes,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            process_id: pid,
            memory_usage_bytes,
        }
    }
}
