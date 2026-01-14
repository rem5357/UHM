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
1. **Food Items** - Base ingredients with nutritional data (stored per 100g/100ml/count)
2. **Recipes** (optional) - Collections of food items with quantities in grams/ml
3. **Meal Entry** - The actual logged consumption attached to a day

---

## Getting the Current Date

**IMPORTANT:** When logging meals for "today" or any relative date, use the UCM (Universal Calendar Manager) MCP server to get accurate dates.

**Tool:** `ucm_now`
- Returns the current date and time in ISO format
- Use the `date` field (YYYY-MM-DD) for meal logging

**Example workflow:**
1. User says "log my breakfast"
2. Call `ucm_now` to get current date → returns `{"date": "2026-01-13", "time": "08:30:00", ...}`
3. Use `"2026-01-13"` as the date parameter in `log_meal`

**Why use UCM?** LLMs have limitations with temporal reasoning. UCM provides accurate, real-time date calculations. Always use `ucm_now` rather than guessing or assuming the current date.

---

## The 80% Rule for Nutrition Ranges

When a nutrition source provides a **range** instead of a single value, use this formula:

**Final Value = Low Value + (80% × Difference)**

### Example
- Source says: "500-600 calories"
- Calculation: 500 + (0.80 × 100) = **580 calories**

### Why 80%?
- Manufacturers often underestimate actual values
- The high end may represent worst-case scenarios
- 80% provides a realistic estimate without over-counting

### Apply to ALL nutrition fields with ranges:
- Calories, protein, fat, carbs, fiber, sugar, sodium, etc.

---

## MANDATORY Unit Standards for Food Items

ALL food items MUST use one of these three standardized formats:

| Category | serving_size | serving_unit | Nutrition stored as |
|----------|--------------|--------------|---------------------|
| **Solids** | 100 | g | per 100 grams |
| **Liquids** | 100 | ml | per 100 milliliters |
| **Countables** | 1 | count | per 1 item |

**NO EXCEPTIONS.** This ensures consistent, accurate calculations.

### Unit Selection Decision Tree

1. **Is it countable?** (sushi pieces, eggs, cookies, whole fruits, pills)
   → `serving_size: 1, serving_unit: "count"`
   → Nutrition = per 1 item
   → ALWAYS use "count", never "piece", "each", "item", or "unit"

2. **Is it a solid/semi-solid?** (meat, cheese, vegetables, rice, powders, spreads)
   → `serving_size: 100, serving_unit: "g"`
   → Nutrition = per 100 grams

3. **Is it a liquid?** (milk, juice, oil, broth, sauces)
   → `serving_size: 100, serving_unit: "ml"`
   → Nutrition = per 100 milliliters

### Converting Package Nutrition to Standard Format

Most packages show nutrition "per serving" (e.g., "per 2 tbsp (32g)"). Convert to per-100g:

**Formula:** `(nutrition_value / package_grams) * 100`

**Example - Peanut Butter:**
- Package says: 190 cal per 2 tbsp (32g)
- Convert: (190 / 32) * 100 = 594 calories per 100g
- Store as: `serving_size: 100, serving_unit: "g", calories: 594`

**Example - Oat Milk:**
- Package says: 110 cal per 1 cup (240ml)
- Convert: (110 / 240) * 100 = 46 calories per 100ml
- Store as: `serving_size: 100, serving_unit: "ml", calories: 46`

### NEVER Use These Units in Food Items

- "handful", "portion", "serving"
- "piece", "each", "item" (use "count" instead)
- "tbsp", "cup", "scoop" (convert to grams, store per 100g)
- "small/medium/large" (put descriptor in name, use "count")

### Examples by Food Type

| Food Type | serving_size | serving_unit | Name Convention |
|-----------|--------------|--------------|-----------------|
| Chicken breast | 100 | g | "Chicken Breast (grilled)" |
| Rice (cooked) | 100 | g | "Jasmine Rice (cooked)" |
| Protein powder | 100 | g | "ON Whey - Vanilla" |
| Olive oil | 100 | ml | "Extra Virgin Olive Oil" |
| Oat milk | 100 | ml | "Oat Milk Barista" |
| Eggs | 1 | count | "Egg (large)" |
| Banana | 1 | count | "Banana (medium)" |
| Sushi | 1 | count | "Salmon Nigiri" |

---

## Nutrition Data Sources

### Primary Source: USDA FoodData Central

**URL:** https://fdc.nal.usda.gov/

**How to use:**
1. Search for the food item (e.g., "chicken breast raw", "rolled oats", "banana")
2. Look for **"Foundation Foods"** or **"SR Legacy"** data types - these are the most accurate analytical values
3. Avoid "Branded Foods" unless specifically looking up a branded product
4. Note the serving size and unit provided, then normalize to canonical units (per 100g for solids, per 100ml for liquids)

**Data types in order of reliability:**
1. **Foundation Foods** - Analytically derived, most accurate
2. **SR Legacy** (Standard Reference) - USDA's historical reference database
3. **FNDDS** - Food and Nutrient Database for Dietary Studies
4. **Branded Foods** - Manufacturer-submitted data (use only for specific branded products)

**Citation:** U.S. Department of Agriculture, Agricultural Research Service. FoodData Central, 2019. fdc.nal.usda.gov

### Secondary Source: Harvard T.H. Chan School of Public Health

**URL:** https://nutritionsource.hsph.harvard.edu/

**How to use:**
- Use for qualitative guidance about food categories (whole grains vs refined, healthy fats, etc.)
- Use for context on why certain foods are healthy/unhealthy
- **NOT a per-food nutritional database** - does not have specific calorie/macro values
- Good for understanding food categories when classifying items

**Useful pages:**
- Food Features: https://nutritionsource.hsph.harvard.edu/food-features/
- Healthy Eating Plate: https://nutritionsource.hsph.harvard.edu/healthy-eating-plate/

### For Branded Products

**Always prefer the actual product label over database lookups.** Label values are:
- Required by FDA to be accurate within regulatory tolerances
- Specific to the exact product formulation
- Include the manufacturer's stated serving size (helpful for conversions like "1 scoop = 31g")

### Workflow for Adding Food Items

| Scenario | Data Source | Action |
|----------|-------------|--------|
| **Branded product with label** | Product label | Use label values directly, convert to per-100g/100ml |
| **Generic food** (e.g., "chicken breast") | USDA FoodData Central | Use Foundation Foods or SR Legacy values |
| **Unknown conversion** (e.g., "grams per cup of flour") | USDA FDC | Check "Portion" data for gram weights of common measures |

**Example - Looking up "rolled oats":**
1. Go to fdc.nal.usda.gov
2. Search "rolled oats"
3. Select the **Foundation Foods** or **SR Legacy** entry
4. Find nutrition per 100g
5. Store in UHM with `serving_size: 100, serving_unit: "g"`

---

## CRITICAL: Recipe Ingredient Units

**This is where most errors occur.** The UHM system does NOT automatically convert volume measurements (cups, tbsp, scoops) to grams. **Claude must perform this conversion manually when adding recipe ingredients.**

### The Golden Rule

**Recipe ingredients must use grams (g) or milliliters (ml) to match the food item's base unit.**

When a user says "add 4 cups of oats", Claude must:
1. Know that 1 cup rolled oats ≈ 80g
2. Calculate: 4 cups × 80g = 320g
3. Store the ingredient as: `quantity: 320, unit: "g"`

### Common Conversion Reference

Claude should know (or look up) these conversions:

#### Dry Goods
| Item | 1 cup = | 1 tbsp = | 1 tsp = |
|------|---------|----------|---------|
| Rolled oats | 80g | 5g | 1.7g |
| Flour | 120g | 7.5g | 2.5g |
| Sugar | 200g | 12.5g | 4.2g |
| Protein powder (whey) | 93g (≈3 scoops) | 6g | 2g |
| Chia seeds | 160g | 10g | 3.3g |
| Ground flaxseed | 130g | 8g | 2.7g |
| PBfit/peanut powder | 64g | 8g | 2.7g |
| Cocoa powder | 85g | 5.3g | 1.8g |
| Freeze-dried fruit | 15-20g | 3g | 1g |
| Monk fruit sweetener | 192g | 12g | 4g |

#### Liquids
| Item | 1 cup = | 1 tbsp = | 1 tsp = |
|------|---------|----------|---------|
| Water/milk/juice | 240ml | 15ml | 5ml |
| Oil | 240ml | 15ml | 5ml |
| Honey/syrup | 340g | 21g | 7g |

#### Protein Powder Scoops
| Brand | 1 scoop = |
|-------|-----------|
| ON Gold Standard | 31g |
| Naked Whey ISO | 16g (their "2 scoops" serving = 32g) |
| Generic whey | ~30g |

### Recipe Ingredient Workflow

**WRONG approach:**
```
add_recipe_ingredient(
  recipe_id: 1,
  food_item_id: 32,  // Rolled Oats (per 100g)
  quantity: 4,
  unit: "cup"        // ❌ System won't convert this properly!
)
```

**CORRECT approach:**
```
add_recipe_ingredient(
  recipe_id: 1,
  food_item_id: 32,  // Rolled Oats (per 100g)
  quantity: 320,     // 4 cups × 80g/cup = 320g
  unit: "g",         // ✅ Matches food item's base unit
  notes: "4 cups × 80g/cup"  // Document the conversion
)
```

### How the System Calculates Nutrition

When the ingredient uses matching units (g for solids, ml for liquids):

```
nutrition_multiplier = ingredient_quantity / food_item_serving_size
                     = 320g / 100g
                     = 3.2

ingredient_calories = food_item_calories × nutrition_multiplier
                    = 375 cal × 3.2
                    = 1,200 cal
```

### Best Practice: Document Conversions in Notes

Always include the original measurement in the notes field:

```
add_recipe_ingredient(
  recipe_id: 6,
  food_item_id: 29,
  quantity: 248,
  unit: "g",
  notes: "8 scoops × 31g/scoop"  // ✅ Future reference
)
```

This helps when:
- Reviewing recipes later
- Debugging calculation errors
- Adjusting recipes (e.g., "I want to use 6 scoops instead")

---

## Step-by-Step Workflow

### Step 1: Check for Existing Food Items

```
search_food_items(query: "chicken breast")
```

### Step 2: Add Food Items (if needed)

**For solids (per 100g):**
```
add_food_item(
  name: "Chicken Breast (grilled)",
  serving_size: 100,
  serving_unit: "g",
  calories: 165,
  protein: 31,
  carbs: 0,
  fat: 3.6,
  fiber: 0,
  sodium: 74,
  ...
)
```

**For liquids (per 100ml):**
```
add_food_item(
  name: "Oat Milk Barista",
  serving_size: 100,
  serving_unit: "ml",
  calories: 46,
  protein: 0.8,
  ...
)
```

**For countables (per 1 item):**
```
add_food_item(
  name: "Egg (large)",
  serving_size: 1,
  serving_unit: "count",
  calories: 72,
  protein: 6.3,
  ...
)
```

### Step 3: Create a Recipe

```
create_recipe(
  name: "Overnight Oats Base Mix",
  servings_produced: 10,
  is_favorite: true,
  notes: "Makes ~6 cups dry mix. ½ cup = 1 serving."
)
```

### Step 4: Add Ingredients (WITH GRAM CONVERSIONS)

**IMPORTANT: Use `add_recipe_ingredients_batch` for efficiency!**

This batch tool adds ALL ingredients in ONE call, which is much faster than calling `add_recipe_ingredient` multiple times (reduces from N tool calls to 1, and only recalculates nutrition once).

For each ingredient, **convert to grams/ml first**, then add all at once:

```
add_recipe_ingredients_batch(
  recipe_id: 6,
  ingredients: [
    {
      food_item_id: 32,      // Rolled oats
      quantity: 320,         // 4 cups × 80g/cup = 320g
      unit: "g",
      notes: "4 cups × 80g/cup"
    },
    {
      food_item_id: 29,      // ON Whey Protein
      quantity: 248,         // 8 scoops × 31g/scoop = 248g
      unit: "g",
      notes: "8 scoops × 31g/scoop"
    },
    {
      food_item_id: 35,      // Oat milk
      quantity: 240,         // 1 cup = 240ml
      unit: "ml",
      notes: "1 cup = 240ml"
    },
    {
      food_item_id: 53,      // Banana (count item)
      quantity: 0.5,
      unit: "count",
      notes: "½ banana added in morning"
    }
  ]
)
```

The response shows success/failure for each ingredient and the final recipe nutrition.

**Alternative: Single ingredient add** (use only when adding ONE ingredient):
```
add_recipe_ingredient(
  recipe_id: 6,
  food_item_id: 32,
  quantity: 320,
  unit: "g",
  notes: "4 cups × 80g/cup"
)
```

### Step 5: Add Component Recipes (optional)

Recipes can include other recipes as sub-components:

```
add_recipe_component(
  recipe_id: 8,           // DIYOO - Blueberry Donut
  component_recipe_id: 6, // DIYOO Base Mix
  servings: 1,            // 1 serving of base mix
  notes: null
)
```

The system correctly pulls the per-serving nutrition from the component recipe.

### Step 6: Verify Recipe Nutrition

```
get_recipe(id: 8)
```

Check that the `nutrition_per_serving` values are reasonable. If they seem way off (e.g., 1500 calories for overnight oats), the ingredients likely have unit conversion errors.

### Step 7: Log the Meal

```
log_meal(
  date: "2026-01-13",
  meal_type: "breakfast",
  recipe_id: 8,
  servings: 1,
  percent_eaten: 100
)
```

---

## Fixing Existing Recipe Ingredients

If you find a recipe with wrong units (e.g., "4 cups" instead of "320g"):

```
update_recipe_ingredient(
  id: 34,           // ingredient ID
  quantity: 320,    // corrected gram amount
  unit: "g",        // correct unit
  notes: "4 cups × 80g/cup"
)
```

Then recalculate the recipe:

```
recalculate_recipe_nutrition(recipe_id: 6)
```

---

## Quick Conversion Checklist

When adding a recipe ingredient, ask yourself:

1. **What's the food item's base unit?**
   - Check `serving_unit` (should be "g", "ml", or "count")

2. **What unit is the user giving me?**
   - If cups, tbsp, scoops → convert to grams
   - If fl oz, cups (liquid) → convert to ml
   - If "half", "2 pieces" → use decimal with count

3. **Do I know the conversion factor?**
   - Check the reference table above
   - If unknown, search for "[ingredient] grams per cup" or check package

4. **Store in matching units with notes:**
   ```
   quantity: [converted_amount],
   unit: "[g/ml/count]",
   notes: "[original_amount] × [conversion_factor]"
   ```

---

## Common Mistakes to Avoid

### ❌ Mistake 1: Using volume units for solid food items
```
quantity: 4, unit: "cup"  // Won't calculate correctly!
```
**Fix:** Convert to grams first

### ❌ Mistake 2: Using "serving" as a unit for food items
```
quantity: 1, unit: "serving"  // Ambiguous!
```
**Fix:** For count items, use `unit: "count"`. For weight items, use `unit: "g"` with the gram amount.

### ❌ Mistake 3: Forgetting that food items are per 100g/100ml
If you add `quantity: 1, unit: "g"` for something stored per 100g, you're only getting 1% of the nutrition!

**Example:** Oat milk is 46 cal per 100ml
- `quantity: 1, unit: "ml"` → 0.46 cal (wrong!)
- `quantity: 240, unit: "ml"` → 110 cal (correct for 1 cup)

### ❌ Mistake 4: Not documenting conversions
Six months later, you won't remember if "248g" was 8 scoops or something else.

**Fix:** Always use the notes field: `notes: "8 scoops × 31g/scoop"`

---

## Summary

1. **Food items** → Always per 100g, 100ml, or 1 count
2. **Recipe ingredients** → Always in grams or ml (Claude converts from cups/tbsp/scoops)
3. **Document conversions** → Use notes field for future reference
4. **Verify results** → Check that `nutrition_per_serving` is reasonable after adding ingredients

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
| Get nutrition statistics | `list_days_stats` |
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

### Analyzing nutrition trends
Use `list_days_stats` to get comprehensive statistics across all logged days:
```
list_days_stats(start_date: "2026-01-01", end_date: "2026-01-31")
```

Returns for each nutrient (calories, protein, carbs, fat, fiber, sugar, sodium, saturated_fat, cholesterol):
- **count** - Number of days with data
- **sum** - Total across all days
- **average** - Mean daily intake
- **median** - Middle value (less affected by outliers)
- **mode** - Most common value
- **standard_deviation** - Variability measure
- **min / max / range** - Lowest, highest, and spread
- **percentile_25 / percentile_75 / iqr** - Quartiles and interquartile range
- **coefficient_of_variation** - Relative variability (SD/mean × 100)
- **outliers** - Days outside 1 standard deviation (with date, value, z-score)

**Why use this?** Much faster than fetching raw data and calculating in Claude Desktop. A single tool call returns all statistics instantly.

**Example use cases:**
- "What's my average daily calorie intake this month?"
- "Which days had unusually high sodium?"
- "How consistent is my protein intake?"

## Updating Food Items (Cascading Recalculation)

Food items can be updated at any time, even when used in recipes:
```
update_food_item(id: 5, calories: 170, protein: 32)
```

When you update a food item, **cascading recalculation** automatically updates all affected data:

1. **Direct recipes** - All recipes using this food item as an ingredient
2. **Parent recipes** - All recipes using affected recipes as components (recursive)
3. **Meal entries** - All logged meals using affected recipes
4. **Daily totals** - All days containing affected meal entries

**Response includes:**
- `recipes_recalculated` - Number of recipes that were updated
- `days_recalculated` - Number of days that were updated

**Example response:**
```json
{
  "success": true,
  "updated_at": "2026-01-12T10:30:00",
  "recipes_recalculated": 5,
  "days_recalculated": 12
}
```

This feature is particularly useful for:
- Correcting nutritional data you entered incorrectly
- Updating serving sizes or units
- All historical data automatically reflects corrections

---

## Batch Updates (For Updating Many Food Items)

When updating many food items at once (e.g., standardizing all units to 100g/100ml), use batch mode to avoid performance issues.

### Why Use Batch Mode?

Without batch mode, each `update_food_item` triggers a full cascade recalculation:
- Find all recipes using that item
- Recalculate each recipe
- Find all meals using those recipes
- Recalculate all affected days

If you update 50 food items that all affect the same 10 recipes, those 10 recipes get recalculated **50 times**!

With batch mode, the cascade happens **once** at the end with all affected recipes/days combined.

### Batch Update Workflow

**Step 1: Start batch mode**
```
start_batch_update()
```

**Step 2: Update food items normally**
```
update_food_item(id: 1, serving_size: 100, serving_unit: "g", ...)
update_food_item(id: 2, serving_size: 100, serving_unit: "g", ...)
update_food_item(id: 3, serving_size: 100, serving_unit: "ml", ...)
... (as many as needed)
```

During batch mode, updates happen immediately but cascade recalculation is deferred. You'll see `cascade_deferred: true` in responses.

**Step 3: Finish batch mode**
```
finish_batch_update()
```

This performs ONE combined cascade for all changed food items and returns:
```json
{
  "success": true,
  "message": "Batch update completed successfully",
  "food_items_processed": 50,
  "recipes_recalculated": 10,
  "days_recalculated": 25
}
```

### When to Use Batch Mode

- Standardizing units across all food items
- Correcting nutrition data for multiple items
- Any operation touching more than 5-10 food items

### Important Notes

- If Claude Desktop crashes during batch mode, the food item updates are already saved
- Just call `finish_batch_update()` to complete the cascade
- Calling `start_batch_update()` when already in batch mode is safe (returns current state)

---

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
| Get statistics by type | `list_vitals_stats` |
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
