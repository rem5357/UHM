//! Data models
//!
//! Rust structs representing database entities.

mod day;
mod exercise;
mod food_item;
mod meal_entry;
mod medication;
mod nutrition;
mod recipe;
mod recipe_component;
mod recipe_ingredient;
mod vital;

pub use day::{Day, DayCreate, DayUpdate};
pub use food_item::{FoodItem, FoodItemCreate, FoodItemUpdate, Preference};
pub use meal_entry::{
    MealEntry, MealEntryCreate, MealEntryDetail, MealEntryUpdate, MealType,
    calculate_day_nutrition, recalculate_day_nutrition,
};
pub use medication::{
    Medication, MedicationCreate, MedicationUpdate, MedicationDeprecate,
    MedType, DosageUnit,
};
pub use nutrition::Nutrition;
pub use recipe::{Recipe, RecipeCreate, RecipeUpdate};
pub use recipe_component::{
    RecipeComponent, RecipeComponentCreate, RecipeComponentDetail, RecipeComponentUpdate,
    would_create_cycle,
};
pub use recipe_ingredient::{
    RecipeIngredient, RecipeIngredientCreate, RecipeIngredientDetail,
    RecipeIngredientUpdate, recalculate_recipe_nutrition,
    cascade_recalculate_from_food_item, CascadeRecalculateResult,
};
pub use vital::{
    Vital, VitalCreate, VitalGroup, VitalGroupCreate, VitalType, VitalUpdate,
};
pub use exercise::{
    Exercise, ExerciseCreate, ExerciseUpdate, ExerciseSegment,
    ExerciseSegmentCreate, ExerciseSegmentUpdate, ExerciseType, CalculatedField,
    recalculate_day_exercise_calories,
};
