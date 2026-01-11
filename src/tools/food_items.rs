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

/// Response for update_food_item when blocked
#[derive(Debug, Serialize)]
pub struct UpdateBlockedResponse {
    pub error: String,
    pub used_in: Vec<String>,
}

/// Response for successful update
#[derive(Debug, Serialize)]
pub struct UpdateSuccessResponse {
    pub success: bool,
    pub updated_at: String,
}

/// Add a new food item
pub fn add_food_item(db: &Database, data: FoodItemCreate) -> Result<AddFoodItemResponse, String> {
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

/// Update a food item (blocked if used in recipes)
pub fn update_food_item(
    db: &Database,
    id: i64,
    data: FoodItemUpdate,
) -> Result<Result<UpdateSuccessResponse, UpdateBlockedResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check usage first
    let usage_count = FoodItem::get_usage_count(&conn, id)
        .map_err(|e| format!("Failed to check usage: {}", e))?;

    if usage_count > 0 {
        let used_in = FoodItem::get_used_in_recipes(&conn, id)
            .map_err(|e| format!("Failed to get recipe usage: {}", e))?;

        return Ok(Err(UpdateBlockedResponse {
            error: format!("Cannot update food item: currently used in {} recipes", usage_count),
            used_in,
        }));
    }

    let updated = FoodItem::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update food item: {}", e))?;

    match updated {
        Some(item) => Ok(Ok(UpdateSuccessResponse {
            success: true,
            updated_at: item.updated_at,
        })),
        None => Ok(Err(UpdateBlockedResponse {
            error: "Food item not found or update blocked".to_string(),
            used_in: vec![],
        })),
    }
}
