//! Data models
//!
//! Rust structs representing database entities.

mod day;
mod food_item;
mod meal_entry;
mod nutrition;
mod recipe;
mod recipe_component;
mod recipe_ingredient;

pub use day::{Day, DayCreate, DayUpdate};
pub use food_item::{FoodItem, FoodItemCreate, FoodItemUpdate, Preference};
pub use meal_entry::{
    MealEntry, MealEntryCreate, MealEntryDetail, MealEntryUpdate, MealType,
    calculate_day_nutrition, recalculate_day_nutrition,
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
};
