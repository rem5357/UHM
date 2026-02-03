//! Meal Entry model
//!
//! Represents food consumed, either from a recipe or direct food item.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use super::{Day, FoodItem, Nutrition, Recipe};

/// Meal type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
    Unspecified,
}

impl MealType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MealType::Breakfast => "breakfast",
            MealType::Lunch => "lunch",
            MealType::Dinner => "dinner",
            MealType::Snack => "snack",
            MealType::Unspecified => "unspecified",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "breakfast" => MealType::Breakfast,
            "lunch" => MealType::Lunch,
            "dinner" => MealType::Dinner,
            "snack" => MealType::Snack,
            _ => MealType::Unspecified,
        }
    }
}

/// A meal entry representing consumed food
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealEntry {
    pub id: i64,
    pub day_id: i64,
    pub meal_type: MealType,
    pub recipe_id: Option<i64>,
    pub food_item_id: Option<i64>,
    pub servings: f64,
    pub percent_eaten: f64,
    /// Direct quantity (e.g., 150 for 150g) - used with quantity/unit workflow
    pub quantity: Option<f64>,
    /// Unit for direct quantity (g, ml, count)
    pub unit: Option<String>,
    pub cached_nutrition: Nutrition,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Meal entry with source details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealEntryDetail {
    pub id: i64,
    pub day_id: i64,
    pub date: String,
    pub meal_type: MealType,
    pub source_type: String,  // "recipe" or "food_item"
    pub source_id: i64,
    pub source_name: String,
    pub servings: f64,
    pub percent_eaten: f64,
    pub nutrition: Nutrition,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Data for creating a meal entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealEntryCreate {
    pub day_id: i64,
    pub meal_type: MealType,
    pub recipe_id: Option<i64>,
    pub food_item_id: Option<i64>,
    pub servings: f64,
    pub percent_eaten: Option<f64>,  // defaults to 100.0
    pub notes: Option<String>,
}

/// Data for updating a meal entry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MealEntryUpdate {
    pub meal_type: Option<MealType>,
    pub servings: Option<f64>,
    pub percent_eaten: Option<f64>,
    pub notes: Option<String>,
}

impl MealEntry {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let meal_type_str: String = row.get("meal_type")?;
        Ok(Self {
            id: row.get("id")?,
            day_id: row.get("day_id")?,
            meal_type: MealType::from_str(&meal_type_str),
            recipe_id: row.get("recipe_id")?,
            food_item_id: row.get("food_item_id")?,
            servings: row.get("servings")?,
            percent_eaten: row.get("percent_eaten")?,
            quantity: row.get("quantity")?,
            unit: row.get("unit")?,
            cached_nutrition: Nutrition {
                calories: row.get("cached_calories")?,
                protein: row.get("cached_protein")?,
                carbs: row.get("cached_carbs")?,
                fat: row.get("cached_fat")?,
                fiber: row.get("cached_fiber")?,
                sodium: row.get("cached_sodium")?,
                sugar: row.get("cached_sugar")?,
                saturated_fat: row.get("cached_saturated_fat")?,
                cholesterol: row.get("cached_cholesterol")?,
            },
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Create a new meal entry
    pub fn create(conn: &Connection, data: &MealEntryCreate) -> DbResult<Self> {
        // Validate that exactly one source is provided
        if data.recipe_id.is_none() && data.food_item_id.is_none() {
            return Err(crate::db::DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                "Either recipe_id or food_item_id must be provided".to_string()
            )));
        }
        if data.recipe_id.is_some() && data.food_item_id.is_some() {
            return Err(crate::db::DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                "Only one of recipe_id or food_item_id can be provided".to_string()
            )));
        }

        let percent_eaten = data.percent_eaten.unwrap_or(100.0);

        // Calculate nutrition based on source
        let base_nutrition = if let Some(recipe_id) = data.recipe_id {
            let recipe = Recipe::get_by_id(conn, recipe_id)?
                .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
            recipe.cached_nutrition
        } else if let Some(food_item_id) = data.food_item_id {
            let food_item = FoodItem::get_by_id(conn, food_item_id)?
                .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
            food_item.nutrition
        } else {
            Nutrition::zero()
        };

        // Scale by servings and percent eaten
        let nutrition = base_nutrition.scale(data.servings * (percent_eaten / 100.0));

        conn.execute(
            r#"
            INSERT INTO meal_entries (
                day_id, meal_type, recipe_id, food_item_id, servings, percent_eaten,
                cached_calories, cached_protein, cached_carbs, cached_fat,
                cached_fiber, cached_sodium, cached_sugar, cached_saturated_fat,
                cached_cholesterol, notes
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            params![
                data.day_id,
                data.meal_type.as_str(),
                data.recipe_id,
                data.food_item_id,
                data.servings,
                percent_eaten,
                nutrition.calories,
                nutrition.protein,
                nutrition.carbs,
                nutrition.fat,
                nutrition.fiber,
                nutrition.sodium,
                nutrition.sugar,
                nutrition.saturated_fat,
                nutrition.cholesterol,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        let entry = Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })?;

        // Recalculate day nutrition
        recalculate_day_nutrition(conn, data.day_id)?;

        Ok(entry)
    }

    /// Create a meal entry using direct quantity/unit (recipe-free workflow)
    ///
    /// This method allows logging food items directly with a quantity in g/ml/count,
    /// calculating nutrition based on the food item's per-serving values.
    pub fn create_direct(
        conn: &Connection,
        day_id: i64,
        meal_type: MealType,
        food_item_id: i64,
        quantity: f64,
        unit: &str,
        percent_eaten: Option<f64>,
        notes: Option<String>,
    ) -> DbResult<Self> {
        let percent_eaten = percent_eaten.unwrap_or(100.0);

        // Get the food item
        let food_item = FoodItem::get_by_id(conn, food_item_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        // Calculate the nutrition multiplier based on quantity and unit
        let multiplier = calculate_direct_log_multiplier(quantity, unit, &food_item);

        // Scale nutrition by multiplier and percent eaten
        let nutrition = food_item.nutrition.scale(multiplier * (percent_eaten / 100.0));

        // Use servings=multiplier for backwards compatibility with existing queries
        let servings = multiplier;

        conn.execute(
            r#"
            INSERT INTO meal_entries (
                day_id, meal_type, recipe_id, food_item_id, servings, percent_eaten,
                quantity, unit,
                cached_calories, cached_protein, cached_carbs, cached_fat,
                cached_fiber, cached_sodium, cached_sugar, cached_saturated_fat,
                cached_cholesterol, notes
            )
            VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            "#,
            params![
                day_id,
                meal_type.as_str(),
                food_item_id,
                servings,
                percent_eaten,
                quantity,
                unit,
                nutrition.calories,
                nutrition.protein,
                nutrition.carbs,
                nutrition.fat,
                nutrition.fiber,
                nutrition.sodium,
                nutrition.sugar,
                nutrition.saturated_fat,
                nutrition.cholesterol,
                notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        let entry = Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })?;

        // Recalculate day nutrition
        recalculate_day_nutrition(conn, day_id)?;

        Ok(entry)
    }

    /// Get a meal entry by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM meal_entries WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get detailed meal entry with source info
    pub fn get_detail(conn: &Connection, id: i64) -> DbResult<Option<MealEntryDetail>> {
        let entry = Self::get_by_id(conn, id)?;

        match entry {
            Some(entry) => {
                let day = Day::get_by_id(conn, entry.day_id)?
                    .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

                let (source_type, source_id, source_name) = if let Some(recipe_id) = entry.recipe_id {
                    let recipe = Recipe::get_by_id(conn, recipe_id)?
                        .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
                    ("recipe".to_string(), recipe_id, recipe.name)
                } else if let Some(food_item_id) = entry.food_item_id {
                    let food_item = FoodItem::get_by_id(conn, food_item_id)?
                        .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
                    ("food_item".to_string(), food_item_id, food_item.name)
                } else {
                    return Ok(None);
                };

                Ok(Some(MealEntryDetail {
                    id: entry.id,
                    day_id: entry.day_id,
                    date: day.date,
                    meal_type: entry.meal_type,
                    source_type,
                    source_id,
                    source_name,
                    servings: entry.servings,
                    percent_eaten: entry.percent_eaten,
                    nutrition: entry.cached_nutrition,
                    notes: entry.notes,
                    created_at: entry.created_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all meal entries for a day
    pub fn get_for_day(conn: &Connection, day_id: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM meal_entries WHERE day_id = ?1 ORDER BY meal_type, id"
        )?;

        let entries = stmt
            .query_map([day_id], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get detailed meal entries for a day
    pub fn get_details_for_day(conn: &Connection, day_id: i64) -> DbResult<Vec<MealEntryDetail>> {
        let entries = Self::get_for_day(conn, day_id)?;
        let day = Day::get_by_id(conn, day_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        let mut details = Vec::new();
        for entry in entries {
            let (source_type, source_id, source_name) = if let Some(recipe_id) = entry.recipe_id {
                let recipe = Recipe::get_by_id(conn, recipe_id)?
                    .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
                ("recipe".to_string(), recipe_id, recipe.name)
            } else if let Some(food_item_id) = entry.food_item_id {
                let food_item = FoodItem::get_by_id(conn, food_item_id)?
                    .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
                ("food_item".to_string(), food_item_id, food_item.name)
            } else {
                continue;
            };

            details.push(MealEntryDetail {
                id: entry.id,
                day_id: entry.day_id,
                date: day.date.clone(),
                meal_type: entry.meal_type,
                source_type,
                source_id,
                source_name,
                servings: entry.servings,
                percent_eaten: entry.percent_eaten,
                nutrition: entry.cached_nutrition,
                notes: entry.notes,
                created_at: entry.created_at,
            });
        }

        Ok(details)
    }

    /// Update a meal entry
    pub fn update(conn: &Connection, id: i64, data: &MealEntryUpdate) -> DbResult<Option<Self>> {
        let entry = Self::get_by_id(conn, id)?;

        if entry.is_none() {
            return Ok(None);
        }
        let entry = entry.unwrap();

        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref meal_type) = data.meal_type {
            updates.push(format!("meal_type = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(meal_type.as_str().to_string()));
        }
        if let Some(servings) = data.servings {
            updates.push(format!("servings = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(servings));
        }
        if let Some(percent_eaten) = data.percent_eaten {
            updates.push(format!("percent_eaten = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(percent_eaten));
        }
        if let Some(ref notes) = data.notes {
            updates.push(format!("notes = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(notes.clone()));
        }

        if updates.is_empty() {
            return Ok(Some(entry));
        }

        // Recalculate nutrition if servings or percent changed
        let needs_recalc = data.servings.is_some() || data.percent_eaten.is_some();

        if needs_recalc {
            let servings = data.servings.unwrap_or(entry.servings);
            let percent_eaten = data.percent_eaten.unwrap_or(entry.percent_eaten);

            let base_nutrition = if let Some(recipe_id) = entry.recipe_id {
                let recipe = Recipe::get_by_id(conn, recipe_id)?
                    .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
                recipe.cached_nutrition
            } else if let Some(food_item_id) = entry.food_item_id {
                let food_item = FoodItem::get_by_id(conn, food_item_id)?
                    .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
                food_item.nutrition
            } else {
                Nutrition::zero()
            };

            let nutrition = base_nutrition.scale(servings * (percent_eaten / 100.0));

            updates.push(format!("cached_calories = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.calories));
            updates.push(format!("cached_protein = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.protein));
            updates.push(format!("cached_carbs = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.carbs));
            updates.push(format!("cached_fat = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.fat));
            updates.push(format!("cached_fiber = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.fiber));
            updates.push(format!("cached_sodium = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.sodium));
            updates.push(format!("cached_sugar = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.sugar));
            updates.push(format!("cached_saturated_fat = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.saturated_fat));
            updates.push(format!("cached_cholesterol = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(nutrition.cholesterol));
        }

        updates.push("updated_at = datetime('now')".to_string());

        let sql = format!(
            "UPDATE meal_entries SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        // Recalculate day nutrition
        recalculate_day_nutrition(conn, entry.day_id)?;

        Self::get_by_id(conn, id)
    }

    /// Delete a meal entry
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Get day_id before delete for recalculation
        let entry = Self::get_by_id(conn, id)?;

        let rows = conn.execute("DELETE FROM meal_entries WHERE id = ?1", [id])?;

        // Recalculate day nutrition if delete succeeded
        if rows > 0 {
            if let Some(entry) = entry {
                recalculate_day_nutrition(conn, entry.day_id)?;
            }
        }

        Ok(rows > 0)
    }
}

/// Calculate total nutrition for a day from meal entries (uses cached values)
pub fn calculate_day_nutrition(conn: &Connection, day_id: i64) -> DbResult<Nutrition> {
    let entries = MealEntry::get_for_day(conn, day_id)?;

    let total: Nutrition = entries
        .iter()
        .map(|e| e.cached_nutrition.clone())
        .sum();

    Ok(total)
}

/// Refresh a meal entry's cached nutrition from its source (recipe or food_item)
/// Returns the updated nutrition value
fn refresh_meal_entry_nutrition(conn: &Connection, entry: &MealEntry) -> DbResult<Nutrition> {
    // Get current nutrition from source
    let base_nutrition = if let Some(recipe_id) = entry.recipe_id {
        let recipe = Recipe::get_by_id(conn, recipe_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
        recipe.cached_nutrition
    } else if let Some(food_item_id) = entry.food_item_id {
        let food_item = FoodItem::get_by_id(conn, food_item_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
        food_item.nutrition
    } else {
        return Ok(Nutrition::zero());
    };

    // Scale by servings and percent eaten
    let nutrition = base_nutrition.scale(entry.servings * (entry.percent_eaten / 100.0));

    // Update the meal entry's cached nutrition
    conn.execute(
        r#"
        UPDATE meal_entries SET
            cached_calories = ?1,
            cached_protein = ?2,
            cached_carbs = ?3,
            cached_fat = ?4,
            cached_fiber = ?5,
            cached_sodium = ?6,
            cached_sugar = ?7,
            cached_saturated_fat = ?8,
            cached_cholesterol = ?9,
            updated_at = datetime('now')
        WHERE id = ?10
        "#,
        params![
            nutrition.calories,
            nutrition.protein,
            nutrition.carbs,
            nutrition.fat,
            nutrition.fiber,
            nutrition.sodium,
            nutrition.sugar,
            nutrition.saturated_fat,
            nutrition.cholesterol,
            entry.id,
        ],
    )?;

    Ok(nutrition)
}

/// Recalculate and update cached nutrition for a day
///
/// This cascades from source: for each meal entry, it fetches the current
/// nutrition from the recipe or food_item, recalculates with servings/percent,
/// updates the meal entry cache, then sums for day totals.
pub fn recalculate_day_nutrition(conn: &Connection, day_id: i64) -> DbResult<Nutrition> {
    let entries = MealEntry::get_for_day(conn, day_id)?;

    // Refresh each meal entry from its source and sum
    let mut total = Nutrition::zero();
    for entry in &entries {
        let nutrition = refresh_meal_entry_nutrition(conn, entry)?;
        total = total + nutrition;
    }

    // Update day's cached nutrition
    Day::update_cached_nutrition(conn, day_id, &total)?;

    Ok(total)
}

/// Calculate the nutrition multiplier for direct quantity/unit logging
///
/// This converts a quantity in g/ml/count to a multiplier for the food item's
/// per-serving nutrition values.
///
/// Examples:
/// - 150g of chicken (food item is per 100g) → 1.5 multiplier
/// - 2 eggs (food item is per 1 count) → 2.0 multiplier
/// - 240ml of oat milk (food item is per 100ml) → 2.4 multiplier
pub fn calculate_direct_log_multiplier(quantity: f64, unit: &str, food_item: &FoodItem) -> f64 {
    let unit_lower = unit.to_lowercase();

    match unit_lower.as_str() {
        "g" | "grams" | "gram" => {
            // Weight-based: divide by grams_per_serving or serving_size
            let grams_per_serving = food_item.grams_per_serving.unwrap_or(food_item.serving_size);
            quantity / grams_per_serving
        }
        "ml" | "milliliters" | "milliliter" => {
            // Volume-based: divide by ml_per_serving or serving_size
            let ml_per_serving = food_item.ml_per_serving.unwrap_or(food_item.serving_size);
            quantity / ml_per_serving
        }
        "count" | "each" | "piece" | "pieces" => {
            // Count-based: quantity is the number of items, serving_size should be 1
            quantity / food_item.serving_size
        }
        "servings" | "serving" => {
            // Direct servings (backwards compatible)
            quantity
        }
        _ => {
            // Unknown unit - treat as servings
            quantity
        }
    }
}
