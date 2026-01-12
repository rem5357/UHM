//! Nutrition calculation module
//!
//! Handles nutrition aggregation and unit conversions.

pub mod converter;
pub mod units;

pub use converter::{
    calculate_grams_per_serving, calculate_ml_per_serving, calculate_nutrition_multiplier,
    infer_base_unit_type, parse_unit, to_grams, to_ml,
};
pub use units::{
    categorize_unit, grams_per_unit, ml_per_unit, BaseUnitType, ParsedUnit, UnitCategory,
};
