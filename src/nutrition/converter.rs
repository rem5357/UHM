//! Unit conversion functions
//!
//! Provides functions for parsing unit strings and converting between units.

use super::units::{
    categorize_unit, grams_per_unit, ml_per_unit, BaseUnitType, ParsedUnit, UnitCategory,
};

/// Parse a unit string, extracting any gram or ml annotation
///
/// Examples:
/// - "g" -> ParsedUnit { base_unit: "g", gram_weight: None, ... }
/// - "tbsp (20g)" -> ParsedUnit { base_unit: "tbsp", gram_weight: Some(20.0), ... }
/// - "cup (240ml)" -> ParsedUnit { base_unit: "cup", ml_amount: Some(240.0), ... }
/// - "slice (28g)" -> ParsedUnit { base_unit: "slice", gram_weight: Some(28.0), ... }
pub fn parse_unit(unit_str: &str) -> ParsedUnit {
    let trimmed = unit_str.trim();

    // Try to extract parenthetical annotation like "(20g)" or "(240ml)"
    if let Some(paren_start) = trimmed.find('(') {
        if let Some(paren_end) = trimmed.find(')') {
            let base_unit = trimmed[..paren_start].trim().to_lowercase();
            let annotation = &trimmed[paren_start + 1..paren_end];

            // Parse gram annotation
            let gram_weight = parse_gram_annotation(annotation);
            let ml_amount = parse_ml_annotation(annotation);

            let category = categorize_unit(&base_unit);

            return ParsedUnit {
                base_unit,
                gram_weight,
                ml_amount,
                category,
            };
        }
    }

    // No annotation - just parse the base unit
    let base_unit = trimmed.to_lowercase();
    let category = categorize_unit(&base_unit);

    ParsedUnit {
        base_unit,
        gram_weight: None,
        ml_amount: None,
        category,
    }
}

/// Parse a gram annotation like "20g" or "20 g" or "20 grams"
fn parse_gram_annotation(s: &str) -> Option<f64> {
    let lower = s.to_lowercase();
    let trimmed = lower.trim();

    // Try patterns: "20g", "20 g", "20grams", "20 grams"
    for suffix in &["g", "gram", "grams"] {
        if trimmed.ends_with(suffix) {
            let num_part = trimmed[..trimmed.len() - suffix.len()].trim();
            if let Ok(val) = num_part.parse::<f64>() {
                return Some(val);
            }
        }
    }

    None
}

/// Parse a ml annotation like "240ml" or "240 ml"
fn parse_ml_annotation(s: &str) -> Option<f64> {
    let lower = s.to_lowercase();
    let trimmed = lower.trim();

    // Try patterns: "240ml", "240 ml", "240milliliters"
    for suffix in &["ml", "milliliter", "milliliters", "millilitre", "millilitres"] {
        if trimmed.ends_with(suffix) {
            let num_part = trimmed[..trimmed.len() - suffix.len()].trim();
            if let Ok(val) = num_part.parse::<f64>() {
                return Some(val);
            }
        }
    }

    None
}

/// Convert a quantity in the given unit to grams
///
/// Returns None if conversion is not possible (e.g., volume to grams without density)
pub fn to_grams(quantity: f64, unit: &str) -> Option<f64> {
    let parsed = parse_unit(unit);

    // If the unit itself has a gram annotation, use that
    if let Some(grams_per) = parsed.gram_weight {
        return Some(quantity * grams_per);
    }

    // If it's a weight unit, convert directly
    if let Some(factor) = grams_per_unit(&parsed.base_unit) {
        return Some(quantity * factor);
    }

    // Cannot convert volume or count to grams without additional info
    None
}

/// Convert a quantity in the given unit to milliliters
///
/// Returns None if conversion is not possible
pub fn to_ml(quantity: f64, unit: &str) -> Option<f64> {
    let parsed = parse_unit(unit);

    // If the unit itself has an ml annotation, use that
    if let Some(ml_per) = parsed.ml_amount {
        return Some(quantity * ml_per);
    }

    // If it's a volume unit, convert directly
    if let Some(factor) = ml_per_unit(&parsed.base_unit) {
        return Some(quantity * factor);
    }

    // Cannot convert weight or count to ml without density
    None
}

/// Calculate the nutrition multiplier for a recipe ingredient
///
/// This is the core function that fixes the unit mismatch bug.
///
/// # Arguments
/// * `quantity` - The amount of ingredient used (e.g., 8.0)
/// * `ingredient_unit` - The unit of the ingredient (e.g., "tbsp")
/// * `serving_size` - The food item's serving size (e.g., 2.0)
/// * `serving_unit` - The food item's serving unit (e.g., "tbsp (20g)")
/// * `grams_per_serving` - Total grams in one serving (e.g., 40.0 for 2 tbsp × 20g)
/// * `ml_per_serving` - Total ml in one serving (for liquids)
///
/// # Returns
/// The multiplier to apply to the food's per-serving nutrition
pub fn calculate_nutrition_multiplier(
    quantity: f64,
    ingredient_unit: &str,
    serving_size: f64,
    serving_unit: &str,
    grams_per_serving: Option<f64>,
    ml_per_serving: Option<f64>,
) -> f64 {
    let ingredient_lower = ingredient_unit.to_lowercase();
    let ingredient_trimmed = ingredient_lower.trim();

    // Case 1: Ingredient is specified in "servings" - quantity IS the multiplier
    if ingredient_trimmed == "serving" || ingredient_trimmed == "servings" {
        return quantity;
    }

    // Parse both units
    let ingredient_parsed = parse_unit(ingredient_unit);
    let food_parsed = parse_unit(serving_unit);

    // Case 2: Base units match exactly (e.g., "tbsp" matches "tbsp" from "tbsp (20g)")
    if ingredient_parsed.base_unit == food_parsed.base_unit {
        return quantity / serving_size;
    }

    // Case 3: Both are weight units - convert to grams and compare
    if ingredient_parsed.category == UnitCategory::Weight {
        if let Some(food_grams) = grams_per_serving {
            if let Some(ingredient_grams) = to_grams(quantity, ingredient_unit) {
                return ingredient_grams / food_grams;
            }
        }
    }

    // Case 4: Ingredient is in grams, food has grams_per_serving
    if ingredient_trimmed == "g" || ingredient_trimmed == "gram" || ingredient_trimmed == "grams" {
        if let Some(food_grams) = grams_per_serving {
            return quantity / food_grams;
        }
    }

    // Case 5: Both are volume units - convert to ml and compare
    if ingredient_parsed.category == UnitCategory::Volume {
        if let Some(food_ml) = ml_per_serving {
            if let Some(ingredient_ml) = to_ml(quantity, ingredient_unit) {
                return ingredient_ml / food_ml;
            }
        }
        // If food doesn't have ml_per_serving but both units are volume,
        // try to convert both to ml
        if food_parsed.category == UnitCategory::Volume {
            if let (Some(ingredient_ml), Some(food_ml_per_unit)) = (
                to_ml(quantity, ingredient_unit),
                ml_per_unit(&food_parsed.base_unit),
            ) {
                let food_ml = serving_size * food_ml_per_unit;
                return ingredient_ml / food_ml;
            }
        }
    }

    // Case 6: Ingredient is in ml, food has ml_per_serving
    if ingredient_trimmed == "ml"
        || ingredient_trimmed == "milliliter"
        || ingredient_trimmed == "milliliters"
    {
        if let Some(food_ml) = ml_per_serving {
            return quantity / food_ml;
        }
    }

    // Fallback: treat quantity as servings (with warning logged)
    tracing::warn!(
        "Unit conversion fallback: '{}' vs '{}'. Treating {} as servings.",
        ingredient_unit,
        serving_unit,
        quantity
    );
    quantity
}

/// Infer the base unit type from a serving unit string
pub fn infer_base_unit_type(serving_unit: &str) -> BaseUnitType {
    let parsed = parse_unit(serving_unit);

    // If there's a gram annotation, it's weight-based
    if parsed.gram_weight.is_some() {
        return BaseUnitType::Weight;
    }

    // If there's an ml annotation, it's volume-based
    if parsed.ml_amount.is_some() {
        return BaseUnitType::Volume;
    }

    // Otherwise, infer from the unit category
    match parsed.category {
        UnitCategory::Weight => BaseUnitType::Weight,
        UnitCategory::Volume => BaseUnitType::Volume,
        UnitCategory::Count => BaseUnitType::Count,
        UnitCategory::Custom => BaseUnitType::Weight, // Default custom units to weight
    }
}

/// Calculate grams_per_serving from serving_size and serving_unit
pub fn calculate_grams_per_serving(serving_size: f64, serving_unit: &str) -> Option<f64> {
    let parsed = parse_unit(serving_unit);

    // If there's a gram annotation, multiply by serving_size
    if let Some(grams_per) = parsed.gram_weight {
        return Some(serving_size * grams_per);
    }

    // If it's a weight unit, convert directly
    if let Some(factor) = grams_per_unit(&parsed.base_unit) {
        return Some(serving_size * factor);
    }

    None
}

/// Calculate ml_per_serving from serving_size and serving_unit
pub fn calculate_ml_per_serving(serving_size: f64, serving_unit: &str) -> Option<f64> {
    let parsed = parse_unit(serving_unit);

    // If there's an ml annotation, multiply by serving_size
    if let Some(ml_per) = parsed.ml_amount {
        return Some(serving_size * ml_per);
    }

    // If it's a volume unit, convert directly
    if let Some(factor) = ml_per_unit(&parsed.base_unit) {
        return Some(serving_size * factor);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unit_simple() {
        let parsed = parse_unit("g");
        assert_eq!(parsed.base_unit, "g");
        assert_eq!(parsed.gram_weight, None);
        assert_eq!(parsed.category, UnitCategory::Weight);
    }

    #[test]
    fn test_parse_unit_with_gram_annotation() {
        let parsed = parse_unit("tbsp (20g)");
        assert_eq!(parsed.base_unit, "tbsp");
        assert_eq!(parsed.gram_weight, Some(20.0));
        assert_eq!(parsed.category, UnitCategory::Volume);
    }

    #[test]
    fn test_parse_unit_with_ml_annotation() {
        let parsed = parse_unit("cup (240ml)");
        assert_eq!(parsed.base_unit, "cup");
        assert_eq!(parsed.ml_amount, Some(240.0));
        assert_eq!(parsed.category, UnitCategory::Volume);
    }

    #[test]
    fn test_parse_unit_slice_with_grams() {
        let parsed = parse_unit("slice (28g)");
        assert_eq!(parsed.base_unit, "slice");
        assert_eq!(parsed.gram_weight, Some(28.0));
        assert_eq!(parsed.category, UnitCategory::Custom);
    }

    #[test]
    fn test_multiplier_matching_units() {
        // 8 tbsp of food with serving_size=2 tbsp = 4 servings
        let mult =
            calculate_nutrition_multiplier(8.0, "tbsp", 2.0, "tbsp (20g)", Some(40.0), None);
        assert!((mult - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_multiplier_grams_to_grams() {
        // 200g of food with 100g serving = 2 servings
        let mult = calculate_nutrition_multiplier(200.0, "g", 100.0, "g", Some(100.0), None);
        assert!((mult - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_multiplier_grams_to_tbsp_food() {
        // 80g of food with serving = 2 tbsp (20g each) = 40g per serving
        // 80g / 40g = 2 servings
        let mult = calculate_nutrition_multiplier(80.0, "g", 2.0, "tbsp (20g)", Some(40.0), None);
        assert!((mult - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_multiplier_servings_unit() {
        // 3 servings = multiplier of 3
        let mult = calculate_nutrition_multiplier(3.0, "serving", 1.0, "cup", None, Some(236.588));
        assert!((mult - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_multiplier_volume_units() {
        // 2 cups of food with 1 cup serving = 2 servings
        let mult = calculate_nutrition_multiplier(2.0, "cup", 1.0, "cup", None, Some(236.588));
        assert!((mult - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_grams_per_serving() {
        // 2 tbsp at 20g each = 40g
        assert_eq!(calculate_grams_per_serving(2.0, "tbsp (20g)"), Some(40.0));

        // 100g serving = 100g
        assert_eq!(calculate_grams_per_serving(100.0, "g"), Some(100.0));

        // 1 cup (no gram annotation) = None
        assert_eq!(calculate_grams_per_serving(1.0, "cup"), None);
    }

    #[test]
    fn test_calculate_ml_per_serving() {
        // 1 cup = ~236.588 ml
        let ml = calculate_ml_per_serving(1.0, "cup").unwrap();
        assert!((ml - 236.588).abs() < 0.01);

        // 2 tbsp = ~29.57 ml
        let ml = calculate_ml_per_serving(2.0, "tbsp").unwrap();
        assert!((ml - 29.5736).abs() < 0.01);
    }

    #[test]
    fn test_infer_base_unit_type() {
        assert_eq!(infer_base_unit_type("g"), BaseUnitType::Weight);
        assert_eq!(infer_base_unit_type("tbsp (20g)"), BaseUnitType::Weight);
        assert_eq!(infer_base_unit_type("cup"), BaseUnitType::Volume);
        assert_eq!(infer_base_unit_type("each"), BaseUnitType::Count);
        assert_eq!(infer_base_unit_type("scoop"), BaseUnitType::Weight);
    }
}
