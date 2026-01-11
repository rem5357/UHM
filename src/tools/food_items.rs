//! Food Item MCP Tools
//!
//! Tools for managing food items in the database.

use serde::Serialize;

use crate::db::Database;
use crate::models::{FoodItem, FoodItemCreate, FoodItemUpdate, Preference};

/// Response for add_food_item
#[derive(Debug, Serialize)]
pub struct AddFoodItemResponse {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub created_at: String,
}

/// Response for search_food_items
#[derive(Debug, Serialize)]
pub struct SearchFoodItemsResponse {
    pub items: Vec<FoodItemSummary>,
    pub total: usize,
}

/// Summary of a food item for list/search results
#[derive(Debug, Serialize)]
pub struct FoodItemSummary {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub serving_size: f64,
    pub serving_unit: String,
    pub calories: f64,
    pub preference: Preference,
}

impl From<&FoodItem> for FoodItemSummary {
    fn from(item: &FoodItem) -> Self {
        Self {
            id: item.id,
            name: item.name.clone(),
            brand: item.brand.clone(),
            serving_size: item.serving_size,
            serving_unit: item.serving_unit.clone(),
            calories: item.nutrition.calories,
            preference: item.preference,
        }
    }
}

/// Full food item detail response
#[derive(Debug, Serialize)]
pub struct FoodItemDetail {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub serving_size: f64,
    pub serving_unit: String,
    pub calories: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sodium: f64,
    pub sugar: f64,
    pub saturated_fat: f64,
    pub cholesterol: f64,
    pub preference: Preference,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i64,
    pub used_in_recipes: Vec<String>,
}

impl FoodItemDetail {
    pub fn from_food_item(item: FoodItem, usage_count: i64, used_in_recipes: Vec<String>) -> Self {
        Self {
            id: item.id,
            name: item.name,
            brand: item.brand,
            serving_size: item.serving_size,
            serving_unit: item.serving_unit,
            calories: item.nutrition.calories,
            protein: item.nutrition.protein,
            carbs: item.nutrition.carbs,
            fat: item.nutrition.fat,
            fiber: item.nutrition.fiber,
            sodium: item.nutrition.sodium,
            sugar: item.nutrition.sugar,
            saturated_fat: item.nutrition.saturated_fat,
            cholesterol: item.nutrition.cholesterol,
            preference: item.preference,
            notes: item.notes,
            created_at: item.created_at,
            updated_at: item.updated_at,
            usage_count,
            used_in_recipes,
        }
    }
}

/// Response for list_food_items
#[derive(Debug, Serialize)]
pub struct ListFoodItemsResponse {
    pub items: Vec<FoodItemSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Response for update_food_item
#[derive(Debug, Serialize)]
pub struct UpdateFoodItemResponse {
    pub success: bool,
    pub updated_at: String,
    pub recipes_updated: Vec<i64>,  // Recipe IDs that had nutrition recalculated
}

/// Unused food item summary (safe to delete)
#[derive(Debug, Serialize)]
pub struct UnusedFoodItemSummary {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub preference: Preference,
    pub created_at: String,
}

/// Response for list_unused_food_items
#[derive(Debug, Serialize)]
pub struct ListUnusedFoodItemsResponse {
    pub items: Vec<UnusedFoodItemSummary>,
    pub count: usize,
}

/// Response for delete_food_item blocked
#[derive(Debug, Serialize)]
pub struct DeleteFoodItemBlockedResponse {
    pub error: String,
    pub usage_count: i64,
    pub used_in_recipes: Vec<String>,
}

/// Response for successful delete_food_item
#[derive(Debug, Serialize)]
pub struct DeleteFoodItemSuccessResponse {
    pub success: bool,
    pub deleted_id: i64,
}

/// Add a new food item
pub fn add_food_item(db: &Database, data: FoodItemCreate) -> Result<AddFoodItemResponse, String> {
    // Validate name
    let name = data.name.trim();
    if name.is_empty() {
        return Err("Food item name cannot be empty".to_string());
    }

    // Validate serving info
    if data.serving_size <= 0.0 {
        return Err("serving_size must be greater than 0".to_string());
    }
    let unit = data.serving_unit.trim();
    if unit.is_empty() {
        return Err("serving_unit cannot be empty".to_string());
    }

    // Validate nutrition values are non-negative
    if data.calories < 0.0 {
        return Err("calories cannot be negative".to_string());
    }
    if data.protein < 0.0 {
        return Err("protein cannot be negative".to_string());
    }
    if data.carbs < 0.0 {
        return Err("carbs cannot be negative".to_string());
    }
    if data.fat < 0.0 {
        return Err("fat cannot be negative".to_string());
    }

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let item = FoodItem::create(&conn, &data)
        .map_err(|e| format!("Failed to create food item: {}", e))?;

    Ok(AddFoodItemResponse {
        id: item.id,
        name: item.name,
        brand: item.brand,
        created_at: item.created_at,
    })
}

/// Search food items by name or brand
pub fn search_food_items(db: &Database, query: &str, limit: i64) -> Result<SearchFoodItemsResponse, String> {
    let limit = limit.min(100).max(1);
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let items = FoodItem::search(&conn, query, limit)
        .map_err(|e| format!("Search failed: {}", e))?;

    let summaries: Vec<FoodItemSummary> = items.iter().map(FoodItemSummary::from).collect();
    let total = summaries.len();

    Ok(SearchFoodItemsResponse {
        items: summaries,
        total,
    })
}

/// Get a food item by ID with usage information
pub fn get_food_item(db: &Database, id: i64) -> Result<Option<FoodItemDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let item = FoodItem::get_by_id(&conn, id)
        .map_err(|e| format!("Failed to get food item: {}", e))?;

    match item {
        Some(item) => {
            let usage_count = FoodItem::get_usage_count(&conn, id)
                .map_err(|e| format!("Failed to get usage count: {}", e))?;
            let used_in_recipes = FoodItem::get_used_in_recipes(&conn, id)
                .map_err(|e| format!("Failed to get recipe usage: {}", e))?;

            Ok(Some(FoodItemDetail::from_food_item(item, usage_count, used_in_recipes)))
        }
        None => Ok(None),
    }
}

/// List food items with filtering and pagination
pub fn list_food_items(
    db: &Database,
    preference: Option<&str>,
    sort_by: &str,
    sort_order: &str,
    limit: i64,
    offset: i64,
) -> Result<ListFoodItemsResponse, String> {
    let limit = limit.min(200).max(1);
    let offset = offset.max(0);
    let pref = preference.map(Preference::from_str);

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let items = FoodItem::list(&conn, pref, sort_by, sort_order, limit, offset)
        .map_err(|e| format!("Failed to list food items: {}", e))?;

    let total = FoodItem::count(&conn, pref)
        .map_err(|e| format!("Failed to count food items: {}", e))?;

    let summaries: Vec<FoodItemSummary> = items.iter().map(FoodItemSummary::from).collect();

    Ok(ListFoodItemsResponse {
        items: summaries,
        total,
        limit,
        offset,
    })
}

/// Update a food item (automatically recalculates nutrition for recipes using this item)
pub fn update_food_item(
    db: &Database,
    id: i64,
    data: FoodItemUpdate,
) -> Result<UpdateFoodItemResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get recipes using this food item (for recalculation after update)
    let recipe_ids = FoodItem::get_recipe_ids_using_item(&conn, id)
        .map_err(|e| format!("Failed to get recipe usage: {}", e))?;

    let updated = FoodItem::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update food item: {}", e))?;

    match updated {
        Some(item) => {
            // Recalculate nutrition for all recipes using this food item
            let mut recipes_updated = Vec::new();
            for recipe_id in &recipe_ids {
                use crate::models::recalculate_recipe_nutrition;
                recalculate_recipe_nutrition(&conn, *recipe_id)
                    .map_err(|e| format!("Failed to recalculate recipe {}: {}", recipe_id, e))?;
                recipes_updated.push(*recipe_id);
            }

            Ok(UpdateFoodItemResponse {
                success: true,
                updated_at: item.updated_at,
                recipes_updated,
            })
        }
        None => Err(format!("Food item not found with id: {}", id)),
    }
}

/// List food items with zero uses (not used in any recipe)
/// These food items are safe to delete
pub fn list_unused_food_items(db: &Database) -> Result<ListUnusedFoodItemsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Find food items that are not used in any recipe_ingredients
    let mut stmt = conn.prepare(
        r#"
        SELECT f.id, f.name, f.brand, f.preference, f.created_at
        FROM food_items f
        WHERE NOT EXISTS (
            SELECT 1 FROM recipe_ingredients ri WHERE ri.food_item_id = f.id
        )
        ORDER BY f.name ASC
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let items: Vec<UnusedFoodItemSummary> = stmt
        .query_map([], |row| {
            Ok(UnusedFoodItemSummary {
                id: row.get("id")?,
                name: row.get("name")?,
                brand: row.get("brand")?,
                preference: Preference::from_str(row.get::<_, String>("preference")?.as_str()),
                created_at: row.get("created_at")?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    let count = items.len();

    Ok(ListUnusedFoodItemsResponse { items, count })
}

/// Delete a food item (blocked if used in any recipe)
pub fn delete_food_item(
    db: &Database,
    id: i64,
) -> Result<Result<DeleteFoodItemSuccessResponse, DeleteFoodItemBlockedResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if food item exists
    let food_item = FoodItem::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;
    if food_item.is_none() {
        return Err(format!("Food item not found with id: {}", id));
    }

    // Check if used in any recipes
    let usage_count = FoodItem::get_usage_count(&conn, id)
        .map_err(|e| format!("Failed to check usage: {}", e))?;

    if usage_count > 0 {
        let used_in_recipes = FoodItem::get_used_in_recipes(&conn, id)
            .map_err(|e| format!("Failed to get recipe usage: {}", e))?;

        return Ok(Err(DeleteFoodItemBlockedResponse {
            error: format!("Cannot delete food item: used in {} recipe(s)", usage_count),
            usage_count,
            used_in_recipes,
        }));
    }

    // Delete the food item
    FoodItem::delete(&conn, id)
        .map_err(|e| format!("Failed to delete food item: {}", e))?;

    Ok(Ok(DeleteFoodItemSuccessResponse {
        success: true,
        deleted_id: id,
    }))
}
