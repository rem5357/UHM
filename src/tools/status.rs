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
| Add ingredient to recipe | `add_recipe_ingredient` |
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
