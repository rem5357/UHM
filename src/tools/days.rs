//! Day and Meal Entry MCP Tools
//!
//! Tools for managing days and logging meals.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::models::{
    Day, DayUpdate, MealEntry, MealEntryCreate, MealEntryDetail, MealEntryUpdate,
    MealType, Nutrition, recalculate_day_nutrition,
    Exercise, ExerciseSegment, FoodItem,
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
    pub exercises: Vec<DayExerciseSummary>,
    pub nutrition_total: Nutrition,
    pub calories_burned: f64,
    pub net_calories: f64,
    pub notes: Option<String>,
}

/// Exercise summary for day detail
#[derive(Debug, Serialize)]
pub struct DayExerciseSummary {
    pub id: i64,
    pub exercise_type: String,
    pub timestamp: String,
    pub total_duration_minutes: f64,
    pub total_distance_miles: f64,
    pub calories_burned: f64,
    pub segment_count: usize,
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
    pub total_fiber: f64,
    pub total_sugar: f64,
    pub total_sodium: f64,
    pub total_saturated_fat: f64,
    pub total_cholesterol: f64,
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

/// Orphaned day summary (day with no meals)
#[derive(Debug, Serialize)]
pub struct OrphanedDaySummary {
    pub id: i64,
    pub date: String,
    pub notes: Option<String>,
}

/// Response for list_orphaned_days
#[derive(Debug, Serialize)]
pub struct ListOrphanedDaysResponse {
    pub days: Vec<OrphanedDaySummary>,
    pub count: usize,
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

/// Get a day with full details including meals and exercises
pub fn get_day(db: &Database, date: &str) -> Result<Option<DayDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let day = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to get day: {}", e))?;

    match day {
        Some(day) => {
            // Get meal entries
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

            // Get exercises for this day
            let exercises_raw = Exercise::list_for_day(&conn, day.id)
                .map_err(|e| format!("Failed to get exercises: {}", e))?;

            let mut exercises = Vec::new();
            for e in exercises_raw {
                let segments = ExerciseSegment::list_for_exercise(&conn, e.id)
                    .map_err(|e| format!("Failed to get exercise segments: {}", e))?;

                exercises.push(DayExerciseSummary {
                    id: e.id,
                    exercise_type: e.exercise_type.display_name().to_string(),
                    timestamp: e.timestamp,
                    total_duration_minutes: e.cached_duration_minutes,
                    total_distance_miles: e.cached_distance_miles,
                    calories_burned: e.cached_calories_burned,
                    segment_count: segments.len(),
                    notes: e.notes,
                });
            }

            // Calculate calories burned from cached value
            let calories_burned: f64 = conn.query_row(
                "SELECT COALESCE(cached_calories_burned, 0) FROM days WHERE id = ?1",
                [day.id],
                |row| row.get(0),
            ).unwrap_or(0.0);

            let net_calories = day.cached_nutrition.calories - calories_burned;

            Ok(Some(DayDetail {
                id: day.id,
                date: day.date,
                meals,
                exercises,
                nutrition_total: day.cached_nutrition,
                calories_burned,
                net_calories,
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
            total_fiber: day.cached_nutrition.fiber,
            total_sugar: day.cached_nutrition.sugar,
            total_sodium: day.cached_nutrition.sodium,
            total_saturated_fat: day.cached_nutrition.saturated_fat,
            total_cholesterol: day.cached_nutrition.cholesterol,
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

/// List days with no meal entries (orphaned days safe to delete)
pub fn list_orphaned_days(db: &Database) -> Result<ListOrphanedDaysResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Find days that have no meal_entries
    let mut stmt = conn.prepare(
        r#"
        SELECT d.id, d.date, d.notes
        FROM days d
        WHERE NOT EXISTS (
            SELECT 1 FROM meal_entries me WHERE me.day_id = d.id
        )
        ORDER BY d.date DESC
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let days: Vec<OrphanedDaySummary> = stmt
        .query_map([], |row| {
            Ok(OrphanedDaySummary {
                id: row.get("id")?,
                date: row.get("date")?,
                notes: row.get("notes")?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    let count = days.len();

    Ok(ListOrphanedDaysResponse { days, count })
}

/// Response for delete_day
#[derive(Debug, Serialize)]
pub struct DeleteDayResponse {
    pub deleted: bool,
    pub date: String,
    pub message: String,
}

/// Delete a day by date (only if it has no meal entries)
pub fn delete_day(db: &Database, date: &str) -> Result<DeleteDayResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // First, find the day
    let day = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to get day: {}", e))?;

    let day = match day {
        Some(d) => d,
        None => {
            return Ok(DeleteDayResponse {
                deleted: false,
                date: date.to_string(),
                message: format!("Day not found: {}", date),
            });
        }
    };

    // Check if day has any meal entries
    let meal_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM meal_entries WHERE day_id = ?1",
            [day.id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count meal entries: {}", e))?;

    if meal_count > 0 {
        return Ok(DeleteDayResponse {
            deleted: false,
            date: date.to_string(),
            message: format!(
                "Cannot delete day {} - it has {} meal entries. Delete the meal entries first.",
                date, meal_count
            ),
        });
    }

    // Safe to delete
    Day::delete(&conn, day.id)
        .map_err(|e| format!("Failed to delete day: {}", e))?;

    Ok(DeleteDayResponse {
        deleted: true,
        date: date.to_string(),
        message: format!("Day {} deleted successfully", date),
    })
}

// ============================================================================
// Batch Meal Logging
// ============================================================================

/// Input for a single meal item in batch logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMealItem {
    /// Food item ID
    pub food_item_id: i64,
    /// Quantity in g/ml/count
    pub quantity: f64,
    /// Unit: g, ml, or count
    pub unit: String,
    /// Percentage eaten (0-100, default 100)
    #[serde(default = "default_percent")]
    pub percent_eaten: f64,
    /// Optional notes
    pub notes: Option<String>,
}

fn default_percent() -> f64 { 100.0 }

/// Result for a single item in batch logging
#[derive(Debug, Serialize)]
pub struct BatchMealItemResult {
    pub food_item_id: i64,
    pub food_item_name: String,
    pub meal_entry_id: i64,
    pub quantity: f64,
    pub unit: String,
    pub nutrition: Nutrition,
    pub success: bool,
    pub error: Option<String>,
}

/// Response for log_meal_items_batch
#[derive(Debug, Serialize)]
pub struct LogMealItemsBatchResponse {
    pub date: String,
    pub day_id: i64,
    pub meal_type: String,
    pub items: Vec<BatchMealItemResult>,
    pub items_logged: usize,
    pub items_failed: usize,
    pub meal_total: Nutrition,
    pub day_total: Nutrition,
}

/// Log multiple food items to a meal in one call
///
/// This is the primary tool for the recipe-free workflow:
/// 1. Takes a date, meal type, and list of items with quantities
/// 2. Creates meal_entries with quantity/unit fields populated
/// 3. Calculates nutrition using unit conversion logic
/// 4. Returns per-item results and day totals
pub fn log_meal_items_batch(
    db: &Database,
    date: &str,
    meal_type: &str,
    items: Vec<BatchMealItem>,
) -> Result<LogMealItemsBatchResponse, String> {
    use crate::models::{Day, MealEntry, MealType, FoodItem};

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get or create the day
    let day = Day::get_or_create(&conn, date)
        .map_err(|e| format!("Failed to get/create day: {}", e))?;

    let meal_type_enum = MealType::from_str(meal_type);

    let mut results = Vec::new();
    let mut meal_total = Nutrition::zero();
    let mut items_logged = 0;
    let mut items_failed = 0;

    for item in items {
        // Validate percent_eaten
        let percent_eaten = if item.percent_eaten < 0.0 || item.percent_eaten > 100.0 {
            results.push(BatchMealItemResult {
                food_item_id: item.food_item_id,
                food_item_name: "Unknown".to_string(),
                meal_entry_id: 0,
                quantity: item.quantity,
                unit: item.unit.clone(),
                nutrition: Nutrition::zero(),
                success: false,
                error: Some("percent_eaten must be between 0 and 100".to_string()),
            });
            items_failed += 1;
            continue;
        } else {
            item.percent_eaten
        };

        // Get the food item to get its name
        let food_item = match FoodItem::get_by_id(&conn, item.food_item_id) {
            Ok(Some(fi)) => fi,
            Ok(None) => {
                results.push(BatchMealItemResult {
                    food_item_id: item.food_item_id,
                    food_item_name: "Unknown".to_string(),
                    meal_entry_id: 0,
                    quantity: item.quantity,
                    unit: item.unit.clone(),
                    nutrition: Nutrition::zero(),
                    success: false,
                    error: Some(format!("Food item not found: {}", item.food_item_id)),
                });
                items_failed += 1;
                continue;
            }
            Err(e) => {
                results.push(BatchMealItemResult {
                    food_item_id: item.food_item_id,
                    food_item_name: "Unknown".to_string(),
                    meal_entry_id: 0,
                    quantity: item.quantity,
                    unit: item.unit.clone(),
                    nutrition: Nutrition::zero(),
                    success: false,
                    error: Some(format!("Database error: {}", e)),
                });
                items_failed += 1;
                continue;
            }
        };

        // Create the meal entry using direct quantity/unit
        match MealEntry::create_direct(
            &conn,
            day.id,
            meal_type_enum.clone(),
            item.food_item_id,
            item.quantity,
            &item.unit,
            Some(percent_eaten),
            item.notes,
        ) {
            Ok(entry) => {
                meal_total = meal_total + entry.cached_nutrition.clone();
                results.push(BatchMealItemResult {
                    food_item_id: item.food_item_id,
                    food_item_name: food_item.name,
                    meal_entry_id: entry.id,
                    quantity: item.quantity,
                    unit: item.unit,
                    nutrition: entry.cached_nutrition,
                    success: true,
                    error: None,
                });
                items_logged += 1;
            }
            Err(e) => {
                results.push(BatchMealItemResult {
                    food_item_id: item.food_item_id,
                    food_item_name: food_item.name,
                    meal_entry_id: 0,
                    quantity: item.quantity,
                    unit: item.unit,
                    nutrition: Nutrition::zero(),
                    success: false,
                    error: Some(format!("Failed to create entry: {}", e)),
                });
                items_failed += 1;
            }
        }
    }

    // Get updated day totals
    let updated_day = Day::get_by_id(&conn, day.id)
        .map_err(|e| format!("Failed to get updated day: {}", e))?
        .ok_or_else(|| "Day not found after logging".to_string())?;

    Ok(LogMealItemsBatchResponse {
        date: date.to_string(),
        day_id: day.id,
        meal_type: meal_type_enum.as_str().to_string(),
        items: results,
        items_logged,
        items_failed,
        meal_total,
        day_total: updated_day.cached_nutrition,
    })
}

// ============================================================================
// Day Statistics
// ============================================================================

/// An outlier reading (value outside standard deviation bounds)
#[derive(Debug, Serialize)]
pub struct Outlier {
    pub date: String,
    pub value: f64,
    /// How many standard deviations from mean (positive = above, negative = below)
    pub z_score: f64,
}

/// Statistics for a single nutritional value
#[derive(Debug, Serialize)]
pub struct NutritionStats {
    pub count: i64,
    pub sum: f64,
    pub average: f64,
    pub median: f64,
    pub mode: Option<f64>,
    pub standard_deviation: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub range: f64,
    /// 25th percentile (Q1)
    pub percentile_25: f64,
    /// 75th percentile (Q3)
    pub percentile_75: f64,
    /// Interquartile range (Q3 - Q1)
    pub iqr: f64,
    /// Coefficient of variation (SD/mean * 100) - relative variability
    pub coefficient_of_variation: f64,
    /// Days with values outside 1 standard deviation
    pub outliers: Vec<Outlier>,
}

/// Response for list_days_stats
#[derive(Debug, Serialize)]
pub struct ListDaysStatsResponse {
    pub days_analyzed: i64,
    pub date_range: Option<DateRange>,
    pub calories: NutritionStats,
    pub protein: NutritionStats,
    pub carbs: NutritionStats,
    pub fat: NutritionStats,
    pub fiber: NutritionStats,
    pub sugar: NutritionStats,
    pub sodium: NutritionStats,
    pub saturated_fat: NutritionStats,
    pub cholesterol: NutritionStats,
}

/// Date range for stats
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

/// Internal: day value for stats calculation
struct DayValue {
    date: String,
    value: f64,
}

/// Calculate statistics for a list of day values
fn calculate_stats(values: &[DayValue]) -> NutritionStats {
    if values.is_empty() {
        return NutritionStats {
            count: 0,
            sum: 0.0,
            average: 0.0,
            median: 0.0,
            mode: None,
            standard_deviation: 0.0,
            variance: 0.0,
            min: 0.0,
            max: 0.0,
            range: 0.0,
            percentile_25: 0.0,
            percentile_75: 0.0,
            iqr: 0.0,
            coefficient_of_variation: 0.0,
            outliers: Vec::new(),
        };
    }

    let count = values.len() as i64;
    let mut sorted_values: Vec<f64> = values.iter().map(|v| v.value).collect();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Sum and average
    let sum: f64 = sorted_values.iter().sum();
    let average = sum / count as f64;

    // Min, max, range
    let min = sorted_values[0];
    let max = sorted_values[sorted_values.len() - 1];
    let range = max - min;

    // Median
    let median = if count % 2 == 0 {
        let mid = count as usize / 2;
        (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
    } else {
        sorted_values[count as usize / 2]
    };

    // Mode (most frequent value, rounded to 1 decimal for grouping)
    let mode = {
        let mut freq: HashMap<i64, i64> = HashMap::new();
        for v in &sorted_values {
            let key = (v * 10.0).round() as i64; // Round to 1 decimal
            *freq.entry(key).or_insert(0) += 1;
        }
        let max_freq = freq.values().max().copied().unwrap_or(0);
        if max_freq > 1 {
            // Only report mode if something appears more than once
            freq.iter()
                .find(|(_, &f)| f == max_freq)
                .map(|(&k, _)| k as f64 / 10.0)
        } else {
            None
        }
    };

    // Variance and standard deviation
    let variance = if count > 1 {
        let sum_sq_diff: f64 = sorted_values.iter().map(|v| (v - average).powi(2)).sum();
        sum_sq_diff / (count - 1) as f64 // Sample variance
    } else {
        0.0
    };
    let standard_deviation = variance.sqrt();

    // Percentiles (using linear interpolation)
    let percentile_25 = percentile(&sorted_values, 25.0);
    let percentile_75 = percentile(&sorted_values, 75.0);
    let iqr = percentile_75 - percentile_25;

    // Coefficient of variation
    let coefficient_of_variation = if average != 0.0 {
        (standard_deviation / average) * 100.0
    } else {
        0.0
    };

    // Outliers (outside 1 standard deviation)
    let outliers: Vec<Outlier> = if standard_deviation > 0.0 {
        values
            .iter()
            .filter_map(|dv| {
                let z_score = (dv.value - average) / standard_deviation;
                if z_score.abs() > 1.0 {
                    Some(Outlier {
                        date: dv.date.clone(),
                        value: dv.value,
                        z_score: (z_score * 100.0).round() / 100.0, // Round to 2 decimals
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    NutritionStats {
        count,
        sum: (sum * 100.0).round() / 100.0,
        average: (average * 100.0).round() / 100.0,
        median: (median * 100.0).round() / 100.0,
        mode: mode.map(|m| (m * 100.0).round() / 100.0),
        standard_deviation: (standard_deviation * 100.0).round() / 100.0,
        variance: (variance * 100.0).round() / 100.0,
        min: (min * 100.0).round() / 100.0,
        max: (max * 100.0).round() / 100.0,
        range: (range * 100.0).round() / 100.0,
        percentile_25: (percentile_25 * 100.0).round() / 100.0,
        percentile_75: (percentile_75 * 100.0).round() / 100.0,
        iqr: (iqr * 100.0).round() / 100.0,
        coefficient_of_variation: (coefficient_of_variation * 100.0).round() / 100.0,
        outliers,
    }
}

/// Calculate percentile using linear interpolation
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len();
    let rank = (p / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let weight = rank - lower as f64;

    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

/// Get comprehensive statistics for days' nutrition data
pub fn list_days_stats(
    db: &Database,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<ListDaysStatsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get all days in range with their cached nutrition
    let days = Day::list(&conn, start_date, end_date, 10000, 0)
        .map_err(|e| format!("Failed to list days: {}", e))?;

    if days.is_empty() {
        return Ok(ListDaysStatsResponse {
            days_analyzed: 0,
            date_range: None,
            calories: calculate_stats(&[]),
            protein: calculate_stats(&[]),
            carbs: calculate_stats(&[]),
            fat: calculate_stats(&[]),
            fiber: calculate_stats(&[]),
            sugar: calculate_stats(&[]),
            sodium: calculate_stats(&[]),
            saturated_fat: calculate_stats(&[]),
            cholesterol: calculate_stats(&[]),
        });
    }

    // Collect values for each nutrient
    let mut calories: Vec<DayValue> = Vec::new();
    let mut protein: Vec<DayValue> = Vec::new();
    let mut carbs: Vec<DayValue> = Vec::new();
    let mut fat: Vec<DayValue> = Vec::new();
    let mut fiber: Vec<DayValue> = Vec::new();
    let mut sugar: Vec<DayValue> = Vec::new();
    let mut sodium: Vec<DayValue> = Vec::new();
    let mut saturated_fat: Vec<DayValue> = Vec::new();
    let mut cholesterol: Vec<DayValue> = Vec::new();

    let mut min_date: Option<String> = None;
    let mut max_date: Option<String> = None;

    for day in &days {
        let n = &day.cached_nutrition;

        // Only include days that have actual meals (non-zero calories)
        // This filters out empty days
        if n.calories > 0.0 {
            calories.push(DayValue { date: day.date.clone(), value: n.calories });
            protein.push(DayValue { date: day.date.clone(), value: n.protein });
            carbs.push(DayValue { date: day.date.clone(), value: n.carbs });
            fat.push(DayValue { date: day.date.clone(), value: n.fat });
            fiber.push(DayValue { date: day.date.clone(), value: n.fiber });
            sugar.push(DayValue { date: day.date.clone(), value: n.sugar });
            sodium.push(DayValue { date: day.date.clone(), value: n.sodium });
            saturated_fat.push(DayValue { date: day.date.clone(), value: n.saturated_fat });
            cholesterol.push(DayValue { date: day.date.clone(), value: n.cholesterol });

            // Track date range
            if min_date.is_none() || day.date < *min_date.as_ref().unwrap() {
                min_date = Some(day.date.clone());
            }
            if max_date.is_none() || day.date > *max_date.as_ref().unwrap() {
                max_date = Some(day.date.clone());
            }
        }
    }

    let date_range = match (min_date, max_date) {
        (Some(start), Some(end)) => Some(DateRange { start, end }),
        _ => None,
    };

    Ok(ListDaysStatsResponse {
        days_analyzed: calories.len() as i64,
        date_range,
        calories: calculate_stats(&calories),
        protein: calculate_stats(&protein),
        carbs: calculate_stats(&carbs),
        fat: calculate_stats(&fat),
        fiber: calculate_stats(&fiber),
        sugar: calculate_stats(&sugar),
        sodium: calculate_stats(&sodium),
        saturated_fat: calculate_stats(&saturated_fat),
        cholesterol: calculate_stats(&cholesterol),
    })
}
