//! Day and Meal Entry MCP Tools
//!
//! Tools for managing days and logging meals.

use serde::Serialize;

use crate::db::Database;
use crate::models::{
    Day, DayUpdate, MealEntry, MealEntryCreate, MealEntryDetail, MealEntryUpdate,
    MealType, Nutrition, recalculate_day_nutrition,
};

/// Response for get_or_create_day
#[derive(Debug, Serialize)]
pub struct GetOrCreateDayResponse {
    pub id: i64,
    pub date: String,
    pub created: bool,  // true if newly created, false if already existed
}

/// Day with meal entries for detailed view
#[derive(Debug, Serialize)]
pub struct DayDetail {
    pub id: i64,
    pub date: String,
    pub meals: DayMeals,
    pub nutrition_total: Nutrition,
    pub notes: Option<String>,
}

/// Meals organized by type
#[derive(Debug, Serialize)]
pub struct DayMeals {
    pub breakfast: Vec<MealEntryDetail>,
    pub lunch: Vec<MealEntryDetail>,
    pub dinner: Vec<MealEntryDetail>,
    pub snack: Vec<MealEntryDetail>,
    pub unspecified: Vec<MealEntryDetail>,
}

/// Day summary for listing
#[derive(Debug, Serialize)]
pub struct DaySummary {
    pub id: i64,
    pub date: String,
    pub total_calories: f64,
    pub total_protein: f64,
    pub total_carbs: f64,
    pub total_fat: f64,
    pub meal_count: usize,
}

/// Response for list_days
#[derive(Debug, Serialize)]
pub struct ListDaysResponse {
    pub days: Vec<DaySummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Response for log_meal
#[derive(Debug, Serialize)]
pub struct LogMealResponse {
    pub id: i64,
    pub day_id: i64,
    pub date: String,
    pub meal_type: String,
    pub source_type: String,
    pub source_name: String,
    pub servings: f64,
    pub percent_eaten: f64,
    pub nutrition: Nutrition,
}

/// Response for update_meal_entry
#[derive(Debug, Serialize)]
pub struct UpdateMealEntryResponse {
    pub id: i64,
    pub meal_type: String,
    pub servings: f64,
    pub percent_eaten: f64,
    pub nutrition: Nutrition,
    pub updated_at: String,
}

/// Response for recalculate_day_nutrition
#[derive(Debug, Serialize)]
pub struct RecalculateDayNutritionResponse {
    pub day_id: i64,
    pub date: String,
    pub nutrition: Nutrition,
}

// ============================================================================
// Day Tools
// ============================================================================

/// Get or create a day by date
pub fn get_or_create_day(db: &Database, date: &str) -> Result<GetOrCreateDayResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if day already exists
    let existing = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to check day: {}", e))?;

    match existing {
        Some(day) => Ok(GetOrCreateDayResponse {
            id: day.id,
            date: day.date,
            created: false,
        }),
        None => {
            let day = Day::get_or_create(&conn, date)
                .map_err(|e| format!("Failed to create day: {}", e))?;
            Ok(GetOrCreateDayResponse {
                id: day.id,
                date: day.date,
                created: true,
            })
        }
    }
}

/// Get a day with full details including meals
pub fn get_day(db: &Database, date: &str) -> Result<Option<DayDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let day = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to get day: {}", e))?;

    match day {
        Some(day) => {
            let entries = MealEntry::get_details_for_day(&conn, day.id)
                .map_err(|e| format!("Failed to get meal entries: {}", e))?;

            let mut meals = DayMeals {
                breakfast: Vec::new(),
                lunch: Vec::new(),
                dinner: Vec::new(),
                snack: Vec::new(),
                unspecified: Vec::new(),
            };

            for entry in entries {
                match entry.meal_type {
                    MealType::Breakfast => meals.breakfast.push(entry),
                    MealType::Lunch => meals.lunch.push(entry),
                    MealType::Dinner => meals.dinner.push(entry),
                    MealType::Snack => meals.snack.push(entry),
                    MealType::Unspecified => meals.unspecified.push(entry),
                }
            }

            Ok(Some(DayDetail {
                id: day.id,
                date: day.date,
                meals,
                nutrition_total: day.cached_nutrition,
                notes: day.notes,
            }))
        }
        None => Ok(None),
    }
}

/// List days with optional date range
pub fn list_days(
    db: &Database,
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<ListDaysResponse, String> {
    let limit = limit.min(200).max(1);
    let offset = offset.max(0);

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let days = Day::list(&conn, start_date, end_date, limit, offset)
        .map_err(|e| format!("Failed to list days: {}", e))?;

    let total = Day::count(&conn, start_date, end_date)
        .map_err(|e| format!("Failed to count days: {}", e))?;

    let mut summaries = Vec::new();
    for day in days {
        let entries = MealEntry::get_for_day(&conn, day.id)
            .map_err(|e| format!("Failed to get meal entries: {}", e))?;

        summaries.push(DaySummary {
            id: day.id,
            date: day.date,
            total_calories: day.cached_nutrition.calories,
            total_protein: day.cached_nutrition.protein,
            total_carbs: day.cached_nutrition.carbs,
            total_fat: day.cached_nutrition.fat,
            meal_count: entries.len(),
        });
    }

    Ok(ListDaysResponse {
        days: summaries,
        total,
        limit,
        offset,
    })
}

/// Update day notes
pub fn update_day(db: &Database, date: &str, notes: Option<String>) -> Result<Option<DayDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let day = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to get day: {}", e))?;

    match day {
        Some(day) => {
            Day::update(&conn, day.id, &DayUpdate { notes })
                .map_err(|e| format!("Failed to update day: {}", e))?;

            // Return full day detail after update
            drop(conn);
            get_day(db, date)
        }
        None => Ok(None),
    }
}

// ============================================================================
// Meal Entry Tools
// ============================================================================

/// Log a meal entry (food item or recipe)
pub fn log_meal(
    db: &Database,
    date: &str,
    meal_type: &str,
    recipe_id: Option<i64>,
    food_item_id: Option<i64>,
    servings: f64,
    percent_eaten: Option<f64>,
    notes: Option<String>,
) -> Result<LogMealResponse, String> {
    // Validate exactly one source is provided
    if recipe_id.is_none() && food_item_id.is_none() {
        return Err("Must provide either recipe_id or food_item_id".to_string());
    }
    if recipe_id.is_some() && food_item_id.is_some() {
        return Err("Provide only one of recipe_id or food_item_id, not both".to_string());
    }

    // Validate servings
    if servings <= 0.0 {
        return Err("Servings must be greater than 0".to_string());
    }

    // Validate percent_eaten if provided
    if let Some(pct) = percent_eaten {
        if pct < 0.0 || pct > 100.0 {
            return Err("percent_eaten must be between 0 and 100".to_string());
        }
    }

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Validate recipe exists if provided
    if let Some(rid) = recipe_id {
        let recipe = crate::models::Recipe::get_by_id(&conn, rid)
            .map_err(|e| format!("Database error checking recipe: {}", e))?;
        if recipe.is_none() {
            return Err(format!("Recipe not found with id: {}", rid));
        }
    }

    // Validate food item exists if provided
    if let Some(fid) = food_item_id {
        let food_item = crate::models::FoodItem::get_by_id(&conn, fid)
            .map_err(|e| format!("Database error checking food item: {}", e))?;
        if food_item.is_none() {
            return Err(format!("Food item not found with id: {}", fid));
        }
    }

    // Get or create the day
    let day = Day::get_or_create(&conn, date)
        .map_err(|e| format!("Failed to get/create day: {}", e))?;

    let meal_type_enum = MealType::from_str(meal_type);

    let data = MealEntryCreate {
        day_id: day.id,
        meal_type: meal_type_enum,
        recipe_id,
        food_item_id,
        servings,
        percent_eaten,
        notes,
    };

    let entry = MealEntry::create(&conn, &data)
        .map_err(|e| format!("Failed to log meal: {}", e))?;

    // Get source details
    let (source_type, source_name) = if let Some(recipe_id) = entry.recipe_id {
        let recipe = crate::models::Recipe::get_by_id(&conn, recipe_id)
            .map_err(|e| format!("Failed to get recipe: {}", e))?
            .ok_or_else(|| "Recipe not found".to_string())?;
        ("recipe".to_string(), recipe.name)
    } else if let Some(food_item_id) = entry.food_item_id {
        let food_item = crate::models::FoodItem::get_by_id(&conn, food_item_id)
            .map_err(|e| format!("Failed to get food item: {}", e))?
            .ok_or_else(|| "Food item not found".to_string())?;
        ("food_item".to_string(), food_item.name)
    } else {
        return Err("No source found".to_string());
    };

    Ok(LogMealResponse {
        id: entry.id,
        day_id: day.id,
        date: day.date,
        meal_type: entry.meal_type.as_str().to_string(),
        source_type,
        source_name,
        servings: entry.servings,
        percent_eaten: entry.percent_eaten,
        nutrition: entry.cached_nutrition,
    })
}

/// Get a meal entry by ID
pub fn get_meal_entry(db: &Database, id: i64) -> Result<Option<MealEntryDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    MealEntry::get_detail(&conn, id)
        .map_err(|e| format!("Failed to get meal entry: {}", e))
}

/// Update a meal entry
pub fn update_meal_entry(
    db: &Database,
    id: i64,
    meal_type: Option<&str>,
    servings: Option<f64>,
    percent_eaten: Option<f64>,
    notes: Option<String>,
) -> Result<Option<UpdateMealEntryResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let data = MealEntryUpdate {
        meal_type: meal_type.map(MealType::from_str),
        servings,
        percent_eaten,
        notes,
    };

    let updated = MealEntry::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update meal entry: {}", e))?;

    match updated {
        Some(entry) => Ok(Some(UpdateMealEntryResponse {
            id: entry.id,
            meal_type: entry.meal_type.as_str().to_string(),
            servings: entry.servings,
            percent_eaten: entry.percent_eaten,
            nutrition: entry.cached_nutrition,
            updated_at: entry.updated_at,
        })),
        None => Ok(None),
    }
}

/// Delete a meal entry
pub fn delete_meal_entry(db: &Database, id: i64) -> Result<bool, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    MealEntry::delete(&conn, id)
        .map_err(|e| format!("Failed to delete meal entry: {}", e))
}

/// Force recalculate day nutrition
pub fn recalculate_day_nutrition_tool(db: &Database, date: &str) -> Result<RecalculateDayNutritionResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let day = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to get day: {}", e))?
        .ok_or_else(|| format!("Day not found: {}", date))?;

    let nutrition = recalculate_day_nutrition(&conn, day.id)
        .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;

    Ok(RecalculateDayNutritionResponse {
        day_id: day.id,
        date: day.date,
        nutrition,
    })
}
