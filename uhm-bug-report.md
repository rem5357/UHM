# UHM Bug Report: Recipe Nutrition Calculation Inflation

**Date:** 2026-01-11  
**Reporter:** Robert (via Claude Opus)  
**Severity:** High - Incorrect nutrition totals affect meal tracking accuracy  
**Build:** UHM Build 10+

---

## Summary

Recipe nutrition calculations are inflated by approximately 37% compared to manual summation of ingredient values. The stored food item values are correct, ingredient quantities are correct, but the computed `nutrition_per_serving` in recipes returns significantly higher numbers.

---

## Test Case: Recipe ID 5

**Recipe:** Kefir DIYOO v1 - Strawberry Banana Chocolate  
**Servings Produced:** 1.0

### Ingredients (from `get_recipe` response)

| Ingredient ID | Food Item ID | Name | Quantity | Unit |
|---------------|--------------|------|----------|------|
| 30 | 37 | DIYOO Base Mix | 1.0 | serving |
| 31 | 36 | Lowfat Kefir - Strawberry Banana | 1.0 | serving |
| 32 | 35 | Barista Blend Oat Milk - Vanilla | 0.5 | serving |
| 33 | 30 | Gold Standard Whey - Double Rich Chocolate | 1.0 | serving |

### Food Item Values (verified via `get_food_item`)

| Food Item ID | Name | Cal | Protein | Sodium |
|--------------|------|-----|---------|--------|
| 37 | DIYOO Base Mix (½ cup) | 288 | 26g | 108mg |
| 36 | Lifeway Kefir (1 cup) | 180 | 10g | 100mg |
| 35 | Califia Oat Milk (8 fl oz) | 110 | 2g | 100mg |
| 30 | ON Chocolate Whey (1 scoop) | 120 | 24g | 130mg |

### Expected Calculation

```
DIYOO Base:     1.0  × (288 cal, 26g protein, 108mg sodium)
Kefir:          1.0  × (180 cal, 10g protein, 100mg sodium)
Oat Milk:       0.5  × (110 cal,  2g protein, 100mg sodium) = (55 cal, 1g, 50mg)
Chocolate Whey: 1.0  × (120 cal, 24g protein, 130mg sodium)
─────────────────────────────────────────────────────────────
EXPECTED TOTAL:       643 cal,  61g protein, 388mg sodium
```

### Actual UHM Response

From `get_recipe` for Recipe ID 5:

```json
"nutrition_per_serving": {
  "calories": 882.875,
  "protein": 86.125,
  "sodium": 452.25
}
```

### Discrepancy

| Metric | Expected | UHM Shows | Difference | % Inflation |
|--------|----------|-----------|------------|-------------|
| Calories | 643 | 883 | +240 | +37% |
| Protein | 61g | 86g | +25g | +41% |
| Sodium | 388mg | 452mg | +64mg | +16% |

---

## Hypothesis

Possible causes to investigate:

1. **Double-counting an ingredient** - One ingredient may be added twice in the calculation loop
2. **Serving size multiplier applied incorrectly** - The 0.5 serving of oat milk might be treated as 0.5 × full serving size (8 fl oz) then multiplied again
3. **Base mix being expanded** - The DIYOO Base Mix (ID 37) is a standalone food item, but the system might be recursively calculating its components AND the stored values
4. **Quantity/unit mismatch** - The `unit` field in recipe ingredients ("serving") might be interpreted differently than the food item's `serving_unit`

---

## Related Data for Investigation

### Recipe ID 5 Full Response

```json
{
  "id": 5,
  "name": "Kefir DIYOO v1 - Strawberry Banana Chocolate",
  "servings_produced": 1.0,
  "is_favorite": true,
  "ingredients": [
    {"id": 30, "food_item_id": 37, "quantity": 1.0, "unit": "serving"},
    {"id": 31, "food_item_id": 36, "quantity": 1.0, "unit": "serving"},
    {"id": 32, "food_item_id": 35, "quantity": 0.5, "unit": "serving"},
    {"id": 33, "food_item_id": 30, "quantity": 1.0, "unit": "serving"}
  ],
  "nutrition_per_serving": {
    "calories": 882.875,
    "protein": 86.125,
    "carbs": 79.9375,
    "fat": 19.8125,
    "fiber": 14.0625,
    "sodium": 452.25,
    "sugar": 13.3125,
    "saturated_fat": 0.53125,
    "cholesterol": 55.0
  }
}
```

### Food Item ID 37 (DIYOO Base Mix)

```json
{
  "id": 37,
  "name": "DIYOO Base Mix",
  "brand": "Homemade",
  "serving_size": 0.5,
  "serving_unit": "cup",
  "calories": 288.0,
  "protein": 26.0,
  "sodium": 108.0
}
```

### Food Item ID 35 (Califia Oat Milk)

```json
{
  "id": 35,
  "name": "Barista Blend Oat Milk - Vanilla",
  "brand": "Califia Farms",
  "serving_size": 8.0,
  "serving_unit": "fl oz",
  "calories": 110.0,
  "protein": 2.0,
  "sodium": 100.0
}
```

---

## Steps to Reproduce

1. Call `get_recipe` with `id: 5`
2. Call `get_food_item` for each ingredient's `food_item_id` (37, 36, 35, 30)
3. Manually calculate: `sum(quantity × food_item_nutrition)` for each nutrient
4. Compare manual sum to `nutrition_per_serving` in recipe response

---

## Impact

- Meal logging shows incorrect daily totals
- User cannot trust UHM for nutrition tracking
- Workaround: Add notes to meal entries with manually calculated values (currently in use)

---

## Suggested Fix

Review the nutrition calculation function in the recipe module. Specifically check:

1. The loop that sums ingredient contributions
2. How `quantity` and `unit` interact with food item `serving_size` and `serving_unit`
3. Whether any multiplication is happening twice
4. Edge case: food items where `serving_size` ≠ 1.0 (like oat milk at 8 fl oz or base mix at 0.5 cup)
