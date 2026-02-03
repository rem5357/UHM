//! Food Item MCP Tools
//!
//! Tools for managing food items in the database.

use serde::Serialize;

use crate::db::Database;
use crate::models::{FoodItem, FoodItemCreate, FoodItemUpdate, Preference};
use crate::nutrition::BaseUnitType;

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
    /// Base unit type (weight, volume, or count)
    pub base_unit_type: Option<BaseUnitType>,
    /// Grams per serving (for unit conversion calculations)
    pub grams_per_serving: Option<f64>,
    /// Milliliters per serving (for unit conversion calculations)
    pub ml_per_serving: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
    pub recipe_usage_count: i64,
    pub meal_usage_count: i64,
    pub used_in_recipes: Vec<String>,
    pub used_in_meal_dates: Vec<String>,
}

impl FoodItemDetail {
    pub fn from_food_item(
        item: FoodItem,
        recipe_usage_count: i64,
        meal_usage_count: i64,
        used_in_recipes: Vec<String>,
        used_in_meal_dates: Vec<String>,
    ) -> Self {
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
            base_unit_type: item.base_unit_type,
            grams_per_serving: item.grams_per_serving,
            ml_per_serving: item.ml_per_serving,
            created_at: item.created_at,
            updated_at: item.updated_at,
            recipe_usage_count,
            meal_usage_count,
            used_in_recipes,
            used_in_meal_dates,
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
    pub recipes_recalculated: i64,  // Number of recipes that had nutrition recalculated
    pub days_recalculated: i64,     // Number of days that had nutrition recalculated
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
    pub recipe_usage_count: i64,
    pub meal_usage_count: i64,
    pub used_in_recipes: Vec<String>,
    pub used_in_meal_dates: Vec<String>,
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
            let recipe_usage_count = FoodItem::get_recipe_usage_count(&conn, id)
                .map_err(|e| format!("Failed to get recipe usage count: {}", e))?;
            let meal_usage_count = FoodItem::get_meal_usage_count(&conn, id)
                .map_err(|e| format!("Failed to get meal usage count: {}", e))?;
            let used_in_recipes = FoodItem::get_used_in_recipes(&conn, id)
                .map_err(|e| format!("Failed to get recipe usage: {}", e))?;
            let used_in_meal_dates = FoodItem::get_used_in_meals(&conn, id)
                .map_err(|e| format!("Failed to get meal usage: {}", e))?;

            Ok(Some(FoodItemDetail::from_food_item(
                item,
                recipe_usage_count,
                meal_usage_count,
                used_in_recipes,
                used_in_meal_dates,
            )))
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

/// Update a food item (automatically recalculates nutrition for all affected recipes and days)
pub fn update_food_item(
    db: &Database,
    id: i64,
    data: FoodItemUpdate,
) -> Result<UpdateFoodItemResponse, String> {
    use crate::models::cascade_recalculate_from_food_item;

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let updated = FoodItem::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update food item: {}", e))?;

    match updated {
        Some(item) => {
            // Cascade recalculation: updates all affected recipes and days
            let cascade_result = cascade_recalculate_from_food_item(&conn, id)
                .map_err(|e| format!("Failed to cascade recalculation: {}", e))?;

            Ok(UpdateFoodItemResponse {
                success: true,
                updated_at: item.updated_at,
                recipes_recalculated: cascade_result.recipes_recalculated,
                days_recalculated: cascade_result.days_recalculated,
            })
        }
        None => Err(format!("Food item not found with id: {}", id)),
    }
}

/// Response for update without cascade (used in batch mode)
#[derive(Debug, Serialize)]
pub struct UpdateFoodItemNoCascadeResponse {
    pub success: bool,
    pub updated_at: String,
    pub cascade_deferred: bool,
}

/// Update a food item WITHOUT triggering cascade recalculation
/// Used during batch updates - cascade happens once at the end
pub fn update_food_item_no_cascade(
    db: &Database,
    id: i64,
    data: FoodItemUpdate,
) -> Result<UpdateFoodItemNoCascadeResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let updated = FoodItem::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update food item: {}", e))?;

    match updated {
        Some(item) => {
            Ok(UpdateFoodItemNoCascadeResponse {
                success: true,
                updated_at: item.updated_at,
                cascade_deferred: true,
            })
        }
        None => Err(format!("Food item not found with id: {}", id)),
    }
}

/// Response for batch cascade recalculation
#[derive(Debug, Serialize)]
pub struct BatchCascadeResponse {
    pub success: bool,
    pub food_items_processed: i64,
    pub recipes_recalculated: i64,
    pub days_recalculated: i64,
}

/// Perform cascade recalculation for multiple food items at once
/// Much more efficient than individual cascades when updating many items
pub fn batch_cascade_recalculate(
    db: &Database,
    food_item_ids: &std::collections::HashSet<i64>,
) -> Result<BatchCascadeResponse, String> {
    use std::collections::HashSet;
    use crate::models::{recalculate_recipe_nutrition, recalculate_day_nutrition};

    if food_item_ids.is_empty() {
        return Ok(BatchCascadeResponse {
            success: true,
            food_items_processed: 0,
            recipes_recalculated: 0,
            days_recalculated: 0,
        });
    }

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Step 1: Find ALL recipes using ANY of the changed food items
    let food_ids_str = food_item_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let direct_recipe_ids: Vec<i64> = {
        let sql = format!(
            "SELECT DISTINCT recipe_id FROM recipe_ingredients WHERE food_item_id IN ({})",
            food_ids_str
        );
        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt.query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to query recipes: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect recipe IDs: {}", e))?
    };

    if direct_recipe_ids.is_empty() {
        return Ok(BatchCascadeResponse {
            success: true,
            food_items_processed: food_item_ids.len() as i64,
            recipes_recalculated: 0,
            days_recalculated: 0,
        });
    }

    // Step 2: Expand to include parent recipes (that use affected recipes as components)
    let mut all_affected: HashSet<i64> = HashSet::new();
    let mut to_process: Vec<i64> = direct_recipe_ids;

    while let Some(recipe_id) = to_process.pop() {
        if all_affected.insert(recipe_id) {
            let parent_ids: Vec<i64> = {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT recipe_id FROM recipe_components WHERE component_recipe_id = ?1"
                ).map_err(|e| format!("Failed to prepare parent query: {}", e))?;
                let rows = stmt.query_map([recipe_id], |row| row.get(0))
                    .map_err(|e| format!("Failed to query parents: {}", e))?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("Failed to collect parent IDs: {}", e))?
            };
            to_process.extend(parent_ids);
        }
    }

    // Step 3: Topologically sort recipes (dependencies first)
    let sorted_recipes = topological_sort_recipes_for_batch(&conn, &all_affected)
        .map_err(|e| format!("Failed to sort recipes: {}", e))?;

    // Step 4: Recalculate all affected recipes
    let mut recipes_recalculated = 0i64;
    for recipe_id in &sorted_recipes {
        recalculate_recipe_nutrition(&conn, *recipe_id)
            .map_err(|e| format!("Failed to recalculate recipe {}: {}", recipe_id, e))?;
        recipes_recalculated += 1;
    }

    // Step 5: Find all days with meal entries using affected recipes OR changed food items
    let recipe_ids_str = sorted_recipes
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let affected_day_ids: Vec<i64> = {
        let sql = format!(
            "SELECT DISTINCT day_id FROM meal_entries WHERE recipe_id IN ({}) OR food_item_id IN ({})",
            if recipe_ids_str.is_empty() { "-1".to_string() } else { recipe_ids_str },
            food_ids_str
        );
        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("Failed to prepare day query: {}", e))?;
        let rows = stmt.query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to query days: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect day IDs: {}", e))?
    };

    // Step 6: Recalculate all affected days
    let mut days_recalculated = 0i64;
    for day_id in affected_day_ids {
        recalculate_day_nutrition(&conn, day_id)
            .map_err(|e| format!("Failed to recalculate day {}: {}", day_id, e))?;
        days_recalculated += 1;
    }

    Ok(BatchCascadeResponse {
        success: true,
        food_items_processed: food_item_ids.len() as i64,
        recipes_recalculated,
        days_recalculated,
    })
}

/// Topological sort for batch cascade (same logic as in recipe_ingredient.rs)
fn topological_sort_recipes_for_batch(
    conn: &rusqlite::Connection,
    recipe_ids: &std::collections::HashSet<i64>,
) -> Result<Vec<i64>, String> {
    use std::collections::{HashMap, HashSet, VecDeque};

    if recipe_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut dependencies: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut dependents: HashMap<i64, HashSet<i64>> = HashMap::new();

    for &recipe_id in recipe_ids {
        dependencies.entry(recipe_id).or_default();
        dependents.entry(recipe_id).or_default();
    }

    let ids_str = recipe_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        "SELECT recipe_id, component_recipe_id FROM recipe_components
         WHERE recipe_id IN ({}) AND component_recipe_id IN ({})",
        ids_str, ids_str
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let edges: Vec<(i64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for (parent_id, child_id) in edges {
        dependencies.entry(parent_id).or_default().insert(child_id);
        dependents.entry(child_id).or_default().insert(parent_id);
    }

    let mut result = Vec::new();
    let mut queue: VecDeque<i64> = VecDeque::new();

    for &recipe_id in recipe_ids {
        if dependencies.get(&recipe_id).map_or(true, |d| d.is_empty()) {
            queue.push_back(recipe_id);
        }
    }

    while let Some(recipe_id) = queue.pop_front() {
        result.push(recipe_id);

        if let Some(parents) = dependents.get(&recipe_id).cloned() {
            for parent_id in parents {
                if let Some(deps) = dependencies.get_mut(&parent_id) {
                    deps.remove(&recipe_id);
                    if deps.is_empty() {
                        queue.push_back(parent_id);
                    }
                }
            }
        }
    }

    if result.len() != recipe_ids.len() {
        let missing: Vec<i64> = recipe_ids
            .iter()
            .copied()
            .filter(|id| !result.contains(id))
            .collect();
        result.extend(missing);
    }

    Ok(result)
}

/// List food items with zero uses (not used in any recipe or meal entry)
/// These food items are safe to delete
pub fn list_unused_food_items(db: &Database) -> Result<ListUnusedFoodItemsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Find food items that are not used in any recipe_ingredients AND not used in any meal_entries
    let mut stmt = conn.prepare(
        r#"
        SELECT f.id, f.name, f.brand, f.preference, f.created_at
        FROM food_items f
        WHERE NOT EXISTS (
            SELECT 1 FROM recipe_ingredients ri WHERE ri.food_item_id = f.id
        )
        AND NOT EXISTS (
            SELECT 1 FROM meal_entries me WHERE me.food_item_id = f.id
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

// ============================================================================
// Batch Search
// ============================================================================

/// Full food item summary with all nutrition data (for batch search)
#[derive(Debug, Serialize)]
pub struct FoodItemFullSummary {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub serving_size: f64,
    pub serving_unit: String,
    pub base_unit_type: Option<String>,
    pub grams_per_serving: Option<f64>,
    pub ml_per_serving: Option<f64>,
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
}

impl From<&FoodItem> for FoodItemFullSummary {
    fn from(item: &FoodItem) -> Self {
        Self {
            id: item.id,
            name: item.name.clone(),
            brand: item.brand.clone(),
            serving_size: item.serving_size,
            serving_unit: item.serving_unit.clone(),
            base_unit_type: item.base_unit_type.map(|t| t.to_db_str().to_string()),
            grams_per_serving: item.grams_per_serving,
            ml_per_serving: item.ml_per_serving,
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
        }
    }
}

/// Result for a single query in batch search
#[derive(Debug, Serialize)]
pub struct BatchQueryResult {
    pub query: String,
    pub items: Vec<FoodItemFullSummary>,
    pub count: usize,
    /// Fuzzy match suggestion if no exact results found (from Haiku API)
    pub fuzzy_suggestion: Option<String>,
}

/// Response for search_food_items_batch
#[derive(Debug, Serialize)]
pub struct SearchFoodItemsBatchResponse {
    pub results: Vec<BatchQueryResult>,
    pub total_queries: usize,
    pub total_items_found: usize,
}

/// Search multiple food items in one call with full nutrition data
///
/// This is optimized for the recipe-free workflow where Claude needs to:
/// 1. Search for multiple foods at once
/// 2. Get full nutrition data to calculate meal totals
/// 3. Optionally get fuzzy suggestions for unmatched queries
///
/// Search hierarchy:
/// 1. FTS5 full-text search (handles word tokenization, prefix matching)
/// 2. LIKE fallback (for edge cases)
/// 3. Haiku fuzzy suggestion (semantic matching for synonyms)
pub fn search_food_items_batch(
    db: &Database,
    queries: &[String],
    fuzzy_match: bool,
    limit_per_query: i64,
) -> Result<SearchFoodItemsBatchResponse, String> {
    let limit = limit_per_query.min(20).max(1);
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let mut results = Vec::new();
    let mut total_items_found = 0;

    for query in queries {
        // Try FTS5 search first (handles word tokenization)
        let mut items = FoodItem::search_fts(&conn, query, limit).unwrap_or_default();

        // Fall back to LIKE search if FTS5 returns nothing
        if items.is_empty() {
            items = FoodItem::search(&conn, query, limit)
                .map_err(|e| format!("Search failed for '{}': {}", query, e))?;
        }

        let summaries: Vec<FoodItemFullSummary> = items.iter().map(FoodItemFullSummary::from).collect();
        let count = summaries.len();
        total_items_found += count;

        // Get fuzzy suggestion if no results and fuzzy_match is enabled
        let fuzzy_suggestion = if count == 0 && fuzzy_match {
            get_fuzzy_suggestion(&conn, query).ok().flatten()
        } else {
            None
        };

        results.push(BatchQueryResult {
            query: query.clone(),
            items: summaries,
            count,
            fuzzy_suggestion,
        });
    }

    Ok(SearchFoodItemsBatchResponse {
        total_queries: queries.len(),
        total_items_found,
        results,
    })
}

/// Food item with display name for fuzzy matching
struct FoodItemForFuzzy {
    display_name: String, // "Brand - Name" or just "Name"
    name: String,         // Original name for matching
}

/// Get fuzzy match suggestion using Haiku API (when no exact results found)
///
/// This calls Claude Haiku to suggest the closest matching food name
/// from the database when the user's query doesn't match exactly.
fn get_fuzzy_suggestion(conn: &rusqlite::Connection, query: &str) -> Result<Option<String>, String> {
    use std::env;

    // Get API key from environment
    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => return Ok(None), // No API key, skip fuzzy matching
    };

    // Get all food items with brand info for context
    let all_items = FoodItem::list(conn, None, "name", "asc", 500, 0)
        .map_err(|e| format!("Failed to list food items: {}", e))?;

    if all_items.is_empty() {
        return Ok(None);
    }

    // Build display names that include brand for better matching
    let food_items: Vec<FoodItemForFuzzy> = all_items
        .iter()
        .map(|f| {
            let display_name = match &f.brand {
                Some(brand) if !brand.is_empty() => format!("{} - {}", brand, f.name),
                _ => f.name.clone(),
            };
            FoodItemForFuzzy {
                display_name,
                name: f.name.clone(),
            }
        })
        .collect();

    let names_list = food_items
        .iter()
        .map(|f| f.display_name.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Build the Haiku request with improved prompt
    let prompt = format!(
        "User searched for: \"{}\"\n\n\
         Available food items (format: \"Brand - Name\" or just \"Name\"):\n{}\n\n\
         Return the SINGLE best matching item from the list above.\n\
         - Consider synonyms (shell=tortilla, protein powder=whey)\n\
         - Consider partial matches and word order flexibility\n\
         - Return ONLY the exact text as shown in the list, nothing else\n\
         - If no reasonable match exists, return exactly: NONE",
        query, names_list
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-3-haiku-20240307",
            "max_tokens": 100,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .map_err(|e| format!("Haiku API request failed: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse Haiku response: {}", e))?;

    // Extract the suggestion from the response
    let response_text = body["content"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string());

    let suggestion = response_text.and_then(|text| {
        if text == "NONE" || text.is_empty() {
            return None;
        }

        // Try exact match first (case-insensitive)
        let text_lower = text.to_lowercase();
        for item in &food_items {
            if item.display_name.to_lowercase() == text_lower {
                return Some(item.display_name.clone());
            }
        }

        // Try matching just the name part (in case Haiku returned without brand)
        for item in &food_items {
            if item.name.to_lowercase() == text_lower {
                return Some(item.display_name.clone());
            }
        }

        // Try contains match as last resort
        for item in &food_items {
            if item.display_name.to_lowercase().contains(&text_lower)
                || text_lower.contains(&item.name.to_lowercase())
            {
                return Some(item.display_name.clone());
            }
        }

        None
    });

    Ok(suggestion)
}

/// Delete a food item (blocked if used in any recipe or meal entry)
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
    let recipe_usage_count = FoodItem::get_recipe_usage_count(&conn, id)
        .map_err(|e| format!("Failed to check recipe usage: {}", e))?;

    // Check if used in any meal entries directly
    let meal_usage_count = FoodItem::get_meal_usage_count(&conn, id)
        .map_err(|e| format!("Failed to check meal usage: {}", e))?;

    if recipe_usage_count > 0 || meal_usage_count > 0 {
        let used_in_recipes = FoodItem::get_used_in_recipes(&conn, id)
            .map_err(|e| format!("Failed to get recipe usage: {}", e))?;
        let used_in_meal_dates = FoodItem::get_used_in_meals(&conn, id)
            .map_err(|e| format!("Failed to get meal usage: {}", e))?;

        let mut reasons = Vec::new();
        if recipe_usage_count > 0 {
            reasons.push(format!("used in {} recipe(s)", recipe_usage_count));
        }
        if meal_usage_count > 0 {
            reasons.push(format!("logged in {} meal(s)", meal_usage_count));
        }

        return Ok(Err(DeleteFoodItemBlockedResponse {
            error: format!("Cannot delete food item: {}", reasons.join(", ")),
            recipe_usage_count,
            meal_usage_count,
            used_in_recipes,
            used_in_meal_dates,
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
