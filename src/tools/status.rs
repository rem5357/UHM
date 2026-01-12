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
# UHM Meal Logging Instructions

This guide explains how to log meals using the Universal Health Manager (UHM) tools.

## Overview

To log a meal, you need:
1. **Food Items** - Base ingredients with nutritional data
2. **Recipes** (optional) - Collections of food items
3. **Meal Entry** - The actual logged consumption attached to a day

## Step-by-Step Workflow

### Step 1: Check for Existing Food Items

Before adding new food items, search for existing ones:
```
search_food_items(query: "chicken breast")
```

### Step 2: Add Food Items (if needed)

If the food item doesn't exist, create it with nutritional info per serving:
```
add_food_item(
  name: "Chicken Breast",
  brand: null,  // or brand name for packaged foods
  serving_size: 100,
  serving_unit: "g",
  calories: 165,
  protein: 31,
  carbs: 0,
  fat: 3.6,
  fiber: 0,
  sodium: 74,
  sugar: 0,
  saturated_fat: 1,
  cholesterol: 85
)
```

**Tips:**
- Use consistent units (g for weight, ml for liquids)
- Nutrition values are PER SERVING
- You can add preference: "liked", "disliked", or "neutral"

### Step 3: Create a Recipe (for multi-ingredient meals)

If the meal has multiple ingredients, create a recipe:
```
create_recipe(
  name: "Grilled Chicken Salad",
  servings_produced: 2,  // how many servings this recipe makes
  is_favorite: false,
  notes: "Light lunch option"
)
```

This returns the recipe ID.

### Step 4: Add Ingredients to Recipe

For each ingredient, add it to the recipe:
```
add_recipe_ingredient(
  recipe_id: 1,
  food_item_id: 5,  // the chicken breast
  quantity: 200,     // amount used in entire recipe
  unit: "g"
)
```

**Important:**
- Quantity is for the ENTIRE recipe, not per serving
- Nutrition is automatically calculated and cached
- Each food item can only be added once per recipe (use update to change quantity)

### Step 4b: Add Component Recipes (optional)

Recipes can use other recipes as components (sub-recipes):
```
add_recipe_component(
  recipe_id: 1,          // parent recipe
  component_recipe_id: 5, // the sub-recipe to include
  servings: 2            // how many servings of the component to use
)
```

**Example use case:** A "Burrito Bowl" recipe might include:
- "Cilantro Lime Rice" recipe (2 servings)
- "Black Beans" recipe (1 serving)
- Plus individual food items like chicken, salsa, cheese

**Important:**
- Circular references are automatically prevented (A cannot use B if B uses A)
- Component nutrition is automatically included in parent recipe calculation
- Use `update_recipe_component` to change servings
- Use `remove_recipe_component` to remove a component

**Unit handling:**
- Use `unit: "serving"` when quantity represents number of servings (most common)
  - e.g., `quantity: 1.0, unit: "serving"` = 1 serving of the food item
  - e.g., `quantity: 0.5, unit: "serving"` = half a serving
- Use matching units (g, ml, etc.) when specifying raw amounts
  - e.g., `quantity: 200, unit: "g"` with a food item that has `serving_size: 100, serving_unit: "g"`
  - This calculates: 200g / 100g per serving = 2 servings worth of nutrition

### Step 5: Log the Meal

Now log the consumption to a specific day:

**Option A: Log a recipe**
```
log_meal(
  date: "2026-01-10",
  meal_type: "lunch",  // breakfast, lunch, dinner, snack, unspecified
  recipe_id: 1,
  servings: 1,         // how many servings you ate
  percent_eaten: 100   // optional, default 100
)
```

**Option B: Log a single food item directly**
```
log_meal(
  date: "2026-01-10",
  meal_type: "snack",
  food_item_id: 3,
  servings: 1.5
)
```

### Step 6: View Daily Summary

Check the day's nutrition totals:
```
get_day(date: "2026-01-10")
```

This returns all meals organized by type (breakfast/lunch/dinner/snack) with nutrition totals.

## Quick Reference

| Task | Tool |
|------|------|
| Find food items | `search_food_items` |
| Add new food item | `add_food_item` |
| View food item details | `get_food_item` |
| Create recipe | `create_recipe` |
| Delete unused recipe | `delete_recipe` |
| Add ingredient to recipe | `add_recipe_ingredient` |
| Add sub-recipe to recipe | `add_recipe_component` |
| View recipe with nutrition | `get_recipe` |
| Log meal to day | `log_meal` |
| View day's meals | `get_day` |
| List recent days | `list_days` |
| Update meal entry | `update_meal_entry` |
| Delete meal entry | `delete_meal_entry` |

## Common Scenarios

### Logging a simple snack
1. `search_food_items("apple")` - Check if exists
2. `add_food_item(...)` - Add if needed
3. `log_meal(date, "snack", food_item_id, servings)` - Log it

### Logging a homemade recipe
1. Search/add all ingredients as food items
2. `create_recipe(...)` - Create the recipe
3. `add_recipe_ingredient(...)` - Add each ingredient
4. `get_recipe(id)` - Verify nutrition looks correct
5. `log_meal(date, meal_type, recipe_id, servings)` - Log it

### Using recipe components (sub-recipes)
Example: Creating a "Burrito Bowl" that uses a "Rice" sub-recipe:
1. Create the rice recipe first with its ingredients
2. Create the burrito bowl recipe
3. `add_recipe_component(recipe_id: burrito_bowl_id, component_recipe_id: rice_id, servings: 1)`
4. Add other ingredients directly to the burrito bowl
5. `get_recipe(burrito_bowl_id)` - Will show both ingredients and components with combined nutrition

### Partial consumption
Use `percent_eaten` when you didn't finish:
```
log_meal(date, "dinner", recipe_id: 5, servings: 1, percent_eaten: 75)
```

### Updating a logged meal
```
update_meal_entry(id: 12, servings: 2)  // ate more than initially logged
```

## Updating Food Items

Food items can be updated at any time, even when used in recipes:
```
update_food_item(id: 5, calories: 170, protein: 32)
```

When you update a food item:
- All recipes using that item automatically have their nutrition recalculated
- The response includes `recipes_updated` showing which recipe IDs were affected
- This is useful for correcting nutritional data you entered incorrectly

## Notes

- Dates use ISO format: YYYY-MM-DD
- Days are created automatically when you log the first meal
- Recipe nutrition is cached and updates automatically when:
  - Ingredients are added/removed/updated
  - A food item used in the recipe is updated
- Daily nutrition totals are cached and update when meals change
- Recipes logged in meals cannot be modified (preserves historical accuracy)
- Recipes can only be deleted if:
  - Never logged in any meal entries (times_logged == 0)
  - Not used as a component in any other recipe
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

## Notes

- Timestamps default to current time if not provided
- Blood pressure requires both value1 (systolic) and value2 (diastolic)
- Deleting a group unlinks vitals but doesn't delete them
- Use unit parameter to override default (e.g., "kg" instead of "lbs" for weight)
- Vitals can be added to a group at creation time or linked later
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
