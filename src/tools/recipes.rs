//! Recipe MCP Tools
//!
//! Tools for managing recipes and recipe ingredients.

use serde::Serialize;

use crate::db::Database;
use crate::models::{
    Nutrition, Recipe, RecipeCreate, RecipeIngredient, RecipeIngredientCreate,
    RecipeIngredientDetail, RecipeIngredientUpdate, RecipeUpdate,
    recalculate_recipe_nutrition,
};

/// Response for create_recipe
#[derive(Debug, Serialize)]
pub struct CreateRecipeResponse {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

/// Full recipe detail with ingredients
#[derive(Debug, Serialize)]
pub struct RecipeDetail {
    pub id: i64,
    pub name: String,
    pub servings_produced: f64,
    pub is_favorite: bool,
    pub ingredients: Vec<RecipeIngredientDetail>,
    pub nutrition_per_serving: Nutrition,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub times_logged: i64,
}

/// Recipe summary for listing
#[derive(Debug, Serialize)]
pub struct RecipeSummary {
    pub id: i64,
    pub name: String,
    pub servings_produced: f64,
    pub is_favorite: bool,
    pub calories_per_serving: f64,
    pub ingredient_count: usize,
}

/// Response for list_recipes
#[derive(Debug, Serialize)]
pub struct ListRecipesResponse {
    pub recipes: Vec<RecipeSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Response for add_recipe_ingredient
#[derive(Debug, Serialize)]
pub struct AddIngredientResponse {
    pub id: i64,
    pub recipe_id: i64,
    pub food_item_id: i64,
    pub quantity: f64,
    pub unit: String,
}

/// Response for recipe nutrition recalculation
#[derive(Debug, Serialize)]
pub struct RecalculateNutritionResponse {
    pub recipe_id: i64,
    pub nutrition_per_serving: Nutrition,
}

/// Response for update blocked
#[derive(Debug, Serialize)]
pub struct RecipeUpdateBlockedResponse {
    pub error: String,
    pub times_logged: i64,
}

/// Response for successful update
#[derive(Debug, Serialize)]
pub struct RecipeUpdateSuccessResponse {
    pub success: bool,
    pub updated_at: String,
}

// ============================================================================
// Recipe Tools
// ============================================================================

/// Create a new recipe
pub fn create_recipe(db: &Database, data: RecipeCreate) -> Result<CreateRecipeResponse, String> {
    // Validate name
    let name = data.name.trim();
    if name.is_empty() {
        return Err("Recipe name cannot be empty".to_string());
    }

    // Validate servings
    if data.servings_produced <= 0.0 {
        return Err("servings_produced must be greater than 0".to_string());
    }

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let recipe = Recipe::create(&conn, &data)
        .map_err(|e| format!("Failed to create recipe: {}", e))?;

    Ok(CreateRecipeResponse {
        id: recipe.id,
        name: recipe.name,
        created_at: recipe.created_at,
    })
}

/// Get a recipe with full details
pub fn get_recipe(db: &Database, id: i64) -> Result<Option<RecipeDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let recipe = Recipe::get_by_id(&conn, id)
        .map_err(|e| format!("Failed to get recipe: {}", e))?;

    match recipe {
        Some(recipe) => {
            let ingredients = RecipeIngredient::get_details_for_recipe(&conn, id)
                .map_err(|e| format!("Failed to get ingredients: {}", e))?;

            let times_logged = Recipe::get_times_logged(&conn, id)
                .map_err(|e| format!("Failed to get times logged: {}", e))?;

            Ok(Some(RecipeDetail {
                id: recipe.id,
                name: recipe.name,
                servings_produced: recipe.servings_produced,
                is_favorite: recipe.is_favorite,
                ingredients,
                nutrition_per_serving: recipe.cached_nutrition,
                notes: recipe.notes,
                created_at: recipe.created_at,
                updated_at: recipe.updated_at,
                times_logged,
            }))
        }
        None => Ok(None),
    }
}

/// List recipes with filtering
pub fn list_recipes(
    db: &Database,
    query: Option<&str>,
    favorites_only: bool,
    sort_by: &str,
    sort_order: &str,
    limit: i64,
    offset: i64,
) -> Result<ListRecipesResponse, String> {
    let limit = limit.min(200).max(1);
    let offset = offset.max(0);

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let recipes = Recipe::list(&conn, query, favorites_only, sort_by, sort_order, limit, offset)
        .map_err(|e| format!("Failed to list recipes: {}", e))?;

    let total = Recipe::count(&conn, favorites_only)
        .map_err(|e| format!("Failed to count recipes: {}", e))?;

    let mut summaries = Vec::new();
    for recipe in recipes {
        let ingredients = RecipeIngredient::get_for_recipe(&conn, recipe.id)
            .map_err(|e| format!("Failed to get ingredients: {}", e))?;

        summaries.push(RecipeSummary {
            id: recipe.id,
            name: recipe.name,
            servings_produced: recipe.servings_produced,
            is_favorite: recipe.is_favorite,
            calories_per_serving: recipe.cached_nutrition.calories,
            ingredient_count: ingredients.len(),
        });
    }

    Ok(ListRecipesResponse {
        recipes: summaries,
        total,
        limit,
        offset,
    })
}

/// Update a recipe (blocked if used in meal entries)
pub fn update_recipe(
    db: &Database,
    id: i64,
    data: RecipeUpdate,
) -> Result<Result<RecipeUpdateSuccessResponse, RecipeUpdateBlockedResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let times_logged = Recipe::get_times_logged(&conn, id)
        .map_err(|e| format!("Failed to check usage: {}", e))?;

    if times_logged > 0 {
        return Ok(Err(RecipeUpdateBlockedResponse {
            error: format!("Cannot update recipe: logged {} times in meal entries", times_logged),
            times_logged,
        }));
    }

    let updated = Recipe::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update recipe: {}", e))?;

    match updated {
        Some(recipe) => Ok(Ok(RecipeUpdateSuccessResponse {
            success: true,
            updated_at: recipe.updated_at,
        })),
        None => Ok(Err(RecipeUpdateBlockedResponse {
            error: "Recipe not found or update blocked".to_string(),
            times_logged: 0,
        })),
    }
}

// ============================================================================
// Recipe Ingredient Tools
// ============================================================================

/// Add an ingredient to a recipe
pub fn add_recipe_ingredient(
    db: &Database,
    data: RecipeIngredientCreate,
) -> Result<AddIngredientResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Validate recipe exists
    let recipe = Recipe::get_by_id(&conn, data.recipe_id)
        .map_err(|e| format!("Database error checking recipe: {}", e))?;
    if recipe.is_none() {
        return Err(format!("Recipe not found with id: {}", data.recipe_id));
    }

    // Validate food item exists
    use crate::models::FoodItem;
    let food_item = FoodItem::get_by_id(&conn, data.food_item_id)
        .map_err(|e| format!("Database error checking food item: {}", e))?;
    if food_item.is_none() {
        return Err(format!("Food item not found with id: {}", data.food_item_id));
    }

    // Check if ingredient already exists in recipe
    let existing = RecipeIngredient::get_for_recipe(&conn, data.recipe_id)
        .map_err(|e| format!("Database error checking existing ingredients: {}", e))?;
    if existing.iter().any(|i| i.food_item_id == data.food_item_id) {
        return Err(format!(
            "Food item {} is already an ingredient in recipe {}. Use update_recipe_ingredient to modify quantity.",
            data.food_item_id, data.recipe_id
        ));
    }

    let ingredient = RecipeIngredient::create(&conn, &data)
        .map_err(|e| format!("Failed to add ingredient: {}", e))?;

    // Recalculate nutrition
    recalculate_recipe_nutrition(&conn, data.recipe_id)
        .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;

    Ok(AddIngredientResponse {
        id: ingredient.id,
        recipe_id: ingredient.recipe_id,
        food_item_id: ingredient.food_item_id,
        quantity: ingredient.quantity,
        unit: ingredient.unit,
    })
}

/// Update a recipe ingredient
pub fn update_recipe_ingredient(
    db: &Database,
    id: i64,
    data: RecipeIngredientUpdate,
) -> Result<Option<RecipeIngredient>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get recipe_id before update for recalculation
    let recipe_id = RecipeIngredient::get_recipe_id(&conn, id)
        .map_err(|e| format!("Failed to get recipe: {}", e))?;

    let updated = RecipeIngredient::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update ingredient: {}", e))?;

    // Recalculate nutrition if update succeeded
    if let (Some(_), Some(recipe_id)) = (&updated, recipe_id) {
        recalculate_recipe_nutrition(&conn, recipe_id)
            .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;
    }

    Ok(updated)
}

/// Remove an ingredient from a recipe
pub fn remove_recipe_ingredient(db: &Database, id: i64) -> Result<bool, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get recipe_id before delete for recalculation
    let recipe_id = RecipeIngredient::get_recipe_id(&conn, id)
        .map_err(|e| format!("Failed to get recipe: {}", e))?;

    let deleted = RecipeIngredient::delete(&conn, id)
        .map_err(|e| format!("Failed to remove ingredient: {}", e))?;

    // Recalculate nutrition if delete succeeded
    if deleted {
        if let Some(recipe_id) = recipe_id {
            recalculate_recipe_nutrition(&conn, recipe_id)
                .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;
        }
    }

    Ok(deleted)
}

/// Force recalculate recipe nutrition
pub fn recalculate_nutrition(db: &Database, recipe_id: i64) -> Result<RecalculateNutritionResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let nutrition = recalculate_recipe_nutrition(&conn, recipe_id)
        .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;

    Ok(RecalculateNutritionResponse {
        recipe_id,
        nutrition_per_serving: nutrition,
    })
}
