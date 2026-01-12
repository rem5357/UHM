//! Unit types and conversion constants
//!
//! Provides types for representing measurement units and standard conversion factors.

use serde::{Deserialize, Serialize};

/// Base unit type for a food item's canonical storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BaseUnitType {
    /// Stored per gram (solids, powders)
    Weight,
    /// Stored per milliliter (liquids)
    Volume,
    /// Stored per count/each (eggs, slices) - requires grams_per_serving
    Count,
}

impl BaseUnitType {
    /// Get the canonical unit string for this type
    pub fn canonical_unit(&self) -> &'static str {
        match self {
            BaseUnitType::Weight => "g",
            BaseUnitType::Volume => "ml",
            BaseUnitType::Count => "each",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "weight" => Some(BaseUnitType::Weight),
            "volume" => Some(BaseUnitType::Volume),
            "count" => Some(BaseUnitType::Count),
            _ => None,
        }
    }

    /// Convert to database string
    pub fn to_db_str(&self) -> &'static str {
        match self {
            BaseUnitType::Weight => "weight",
            BaseUnitType::Volume => "volume",
            BaseUnitType::Count => "count",
        }
    }
}

/// Category of a measurement unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitCategory {
    /// Weight/mass units (g, oz, lb, kg)
    Weight,
    /// Volume units (ml, tbsp, cup, etc.)
    Volume,
    /// Count/discrete units (each, piece, slice)
    Count,
    /// Custom unit requiring food-specific conversion (scoop, patty)
    Custom,
}

/// A parsed unit with optional gram weight annotation
#[derive(Debug, Clone)]
pub struct ParsedUnit {
    /// The base unit string (e.g., "tbsp" from "tbsp (20g)")
    pub base_unit: String,
    /// Gram weight if annotated (e.g., 20.0 from "(20g)")
    pub gram_weight: Option<f64>,
    /// Milliliter amount if annotated (e.g., 15.0 from "(15ml)")
    pub ml_amount: Option<f64>,
    /// The category of this unit
    pub category: UnitCategory,
}

// ============================================================================
// Volume Conversion Constants (to milliliters)
// ============================================================================

/// Milliliters per teaspoon
pub const ML_PER_TSP: f64 = 4.92892;
/// Milliliters per tablespoon
pub const ML_PER_TBSP: f64 = 14.7868;
/// Milliliters per fluid ounce
pub const ML_PER_FL_OZ: f64 = 29.5735;
/// Milliliters per cup (US)
pub const ML_PER_CUP: f64 = 236.588;
/// Milliliters per pint (US)
pub const ML_PER_PINT: f64 = 473.176;
/// Milliliters per quart (US)
pub const ML_PER_QUART: f64 = 946.353;
/// Milliliters per liter
pub const ML_PER_LITER: f64 = 1000.0;
/// Milliliters per gallon (US)
pub const ML_PER_GALLON: f64 = 3785.41;

// ============================================================================
// Weight Conversion Constants (to grams)
// ============================================================================

/// Grams per milligram
pub const G_PER_MG: f64 = 0.001;
/// Grams per kilogram
pub const G_PER_KG: f64 = 1000.0;
/// Grams per ounce
pub const G_PER_OZ: f64 = 28.3495;
/// Grams per pound
pub const G_PER_LB: f64 = 453.592;

// ============================================================================
// Unit Recognition
// ============================================================================

/// Get the conversion factor to grams for a weight unit
pub fn grams_per_unit(unit: &str) -> Option<f64> {
    let lower = unit.to_lowercase();
    let trimmed = lower.trim();

    match trimmed {
        "g" | "gram" | "grams" => Some(1.0),
        "mg" | "milligram" | "milligrams" => Some(G_PER_MG),
        "kg" | "kilogram" | "kilograms" => Some(G_PER_KG),
        "oz" | "ounce" | "ounces" => Some(G_PER_OZ),
        "lb" | "lbs" | "pound" | "pounds" => Some(G_PER_LB),
        _ => None,
    }
}

/// Get the conversion factor to milliliters for a volume unit
pub fn ml_per_unit(unit: &str) -> Option<f64> {
    let lower = unit.to_lowercase();
    let trimmed = lower.trim();

    match trimmed {
        "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => Some(1.0),
        "l" | "liter" | "liters" | "litre" | "litres" => Some(ML_PER_LITER),
        "tsp" | "teaspoon" | "teaspoons" => Some(ML_PER_TSP),
        "tbsp" | "tablespoon" | "tablespoons" => Some(ML_PER_TBSP),
        "fl oz" | "floz" | "fluid ounce" | "fluid ounces" => Some(ML_PER_FL_OZ),
        "cup" | "cups" => Some(ML_PER_CUP),
        "pint" | "pints" => Some(ML_PER_PINT),
        "quart" | "quarts" => Some(ML_PER_QUART),
        "gallon" | "gallons" => Some(ML_PER_GALLON),
        _ => None,
    }
}

/// Determine the category of a unit string
pub fn categorize_unit(unit: &str) -> UnitCategory {
    let lower = unit.to_lowercase();
    let trimmed = lower.trim();

    // Check weight units
    if grams_per_unit(trimmed).is_some() {
        return UnitCategory::Weight;
    }

    // Check volume units
    if ml_per_unit(trimmed).is_some() {
        return UnitCategory::Volume;
    }

    // Check count units
    match trimmed {
        "each" | "piece" | "pieces" | "item" | "items" | "count" | "unit" | "units" => {
            return UnitCategory::Count;
        }
        _ => {}
    }

    // Everything else is custom (scoop, slice, patty, etc.)
    UnitCategory::Custom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_weight_units() {
        assert_eq!(categorize_unit("g"), UnitCategory::Weight);
        assert_eq!(categorize_unit("gram"), UnitCategory::Weight);
        assert_eq!(categorize_unit("oz"), UnitCategory::Weight);
        assert_eq!(categorize_unit("lb"), UnitCategory::Weight);
        assert_eq!(categorize_unit("kg"), UnitCategory::Weight);
    }

    #[test]
    fn test_categorize_volume_units() {
        assert_eq!(categorize_unit("ml"), UnitCategory::Volume);
        assert_eq!(categorize_unit("tbsp"), UnitCategory::Volume);
        assert_eq!(categorize_unit("cup"), UnitCategory::Volume);
        assert_eq!(categorize_unit("tsp"), UnitCategory::Volume);
    }

    #[test]
    fn test_categorize_count_units() {
        assert_eq!(categorize_unit("each"), UnitCategory::Count);
        assert_eq!(categorize_unit("piece"), UnitCategory::Count);
    }

    #[test]
    fn test_categorize_custom_units() {
        assert_eq!(categorize_unit("scoop"), UnitCategory::Custom);
        assert_eq!(categorize_unit("slice"), UnitCategory::Custom);
        assert_eq!(categorize_unit("patty"), UnitCategory::Custom);
    }

    #[test]
    fn test_grams_per_unit() {
        assert_eq!(grams_per_unit("g"), Some(1.0));
        assert_eq!(grams_per_unit("oz"), Some(G_PER_OZ));
        assert_eq!(grams_per_unit("lb"), Some(G_PER_LB));
        assert_eq!(grams_per_unit("tbsp"), None);
    }

    #[test]
    fn test_ml_per_unit() {
        assert_eq!(ml_per_unit("ml"), Some(1.0));
        assert_eq!(ml_per_unit("tbsp"), Some(ML_PER_TBSP));
        assert_eq!(ml_per_unit("cup"), Some(ML_PER_CUP));
        assert_eq!(ml_per_unit("g"), None);
    }
}
