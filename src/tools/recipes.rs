//! Recipe MCP Tools
//!
//! Tools for managing recipes, recipe ingredients, and recipe components.

use serde::Serialize;

use crate::db::Database;
use crate::models::{
    Nutrition, Recipe, RecipeCreate, RecipeIngredient, RecipeIngredientCreate,
    RecipeIngredientDetail, RecipeIngredientUpdate, RecipeUpdate,
    RecipeComponent, RecipeComponentCreate, RecipeComponentDetail, RecipeComponentUpdate,
    recalculate_recipe_nutrition, would_create_cycle,
};

/// Response for create_recipe
#[derive(Debug, Serialize)]
pub struct CreateRecipeResponse {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

/// Full recipe detail with ingredients and components
#[derive(Debug, Serialize)]
pub struct RecipeDetail {
    pub id: i64,
    pub name: String,
    pub servings_produced: f64,
    pub is_favorite: bool,
    pub ingredients: Vec<RecipeIngredientDetail>,
    pub components: Vec<RecipeComponentDetail>,
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

/// Unused recipe summary (safe to delete)
#[derive(Debug, Serialize)]
pub struct UnusedRecipeSummary {
    pub id: i64,
    pub name: String,
    pub is_favorite: bool,
    pub created_at: String,
}

/// Response for list_unused_recipes
#[derive(Debug, Serialize)]
pub struct ListUnusedRecipesResponse {
    pub recipes: Vec<UnusedRecipeSummary>,
    pub count: usize,
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

/// Response for delete blocked
#[derive(Debug, Serialize)]
pub struct RecipeDeleteBlockedResponse {
    pub error: String,
    pub times_logged: i64,
    pub component_usage_count: i64,
}

/// Response for successful delete
#[derive(Debug, Serialize)]
pub struct RecipeDeleteSuccessResponse {
    pub success: bool,
    pub deleted_id: i64,
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

            let components = RecipeComponent::get_details_for_recipe(&conn, id)
                .map_err(|e| format!("Failed to get components: {}", e))?;

            let times_logged = Recipe::get_times_logged(&conn, id)
                .map_err(|e| format!("Failed to get times logged: {}", e))?;

            Ok(Some(RecipeDetail {
                id: recipe.id,
                name: recipe.name,
                servings_produced: recipe.servings_produced,
                is_favorite: recipe.is_favorite,
                ingredients,
                components,
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

/// Delete a recipe (blocked if logged in meals or used as component)
pub fn delete_recipe(
    db: &Database,
    id: i64,
) -> Result<Result<RecipeDeleteSuccessResponse, RecipeDeleteBlockedResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if recipe exists
    let recipe = Recipe::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;
    if recipe.is_none() {
        return Err(format!("Recipe not found with id: {}", id));
    }

    // Check if logged in meal entries
    let times_logged = Recipe::get_times_logged(&conn, id)
        .map_err(|e| format!("Failed to check meal usage: {}", e))?;

    // Check if used as component in other recipes
    let component_usage_count = Recipe::get_component_usage_count(&conn, id)
        .map_err(|e| format!("Failed to check component usage: {}", e))?;

    if times_logged > 0 || component_usage_count > 0 {
        let mut reasons = Vec::new();
        if times_logged > 0 {
            reasons.push(format!("logged {} times in meal entries", times_logged));
        }
        if component_usage_count > 0 {
            reasons.push(format!("used as component in {} other recipe(s)", component_usage_count));
        }
        return Ok(Err(RecipeDeleteBlockedResponse {
            error: format!("Cannot delete recipe: {}", reasons.join(", ")),
            times_logged,
            component_usage_count,
        }));
    }

    // Delete the recipe (cascades to recipe_ingredients and recipe_components where this is parent)
    Recipe::delete(&conn, id)
        .map_err(|e| format!("Failed to delete recipe: {}", e))?;

    Ok(Ok(RecipeDeleteSuccessResponse {
        success: true,
        deleted_id: id,
    }))
}

/// List recipes with zero uses (not logged in meals, not used as component in other recipes)
/// These recipes are safe to delete
pub fn list_unused_recipes(db: &Database) -> Result<ListUnusedRecipesResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Find recipes that:
    // 1. Have no meal_entries referencing them
    // 2. Are not used as a component in any other recipe
    let mut stmt = conn.prepare(
        r#"
        SELECT r.id, r.name, r.is_favorite, r.created_at
        FROM recipes r
        WHERE NOT EXISTS (
            SELECT 1 FROM meal_entries me WHERE me.recipe_id = r.id
        )
        AND NOT EXISTS (
            SELECT 1 FROM recipe_components rc WHERE rc.component_recipe_id = r.id
        )
        ORDER BY r.name ASC
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let recipes: Vec<UnusedRecipeSummary> = stmt
        .query_map([], |row| {
            Ok(UnusedRecipeSummary {
                id: row.get("id")?,
                name: row.get("name")?,
                is_favorite: row.get::<_, i32>("is_favorite")? != 0,
                created_at: row.get("created_at")?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    let count = recipes.len();

    Ok(ListUnusedRecipesResponse { recipes, count })
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

// ============================================================================
// Recipe Component Tools
// ============================================================================

/// Response for add_recipe_component
#[derive(Debug, Serialize)]
pub struct AddComponentResponse {
    pub id: i64,
    pub recipe_id: i64,
    pub component_recipe_id: i64,
    pub component_recipe_name: String,
    pub servings: f64,
}

/// Add a recipe as a component of another recipe
pub fn add_recipe_component(
    db: &Database,
    data: RecipeComponentCreate,
) -> Result<AddComponentResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Validate parent recipe exists
    let parent_recipe = Recipe::get_by_id(&conn, data.recipe_id)
        .map_err(|e| format!("Database error checking recipe: {}", e))?;
    if parent_recipe.is_none() {
        return Err(format!("Recipe not found with id: {}", data.recipe_id));
    }

    // Validate component recipe exists
    let component_recipe = Recipe::get_by_id(&conn, data.component_recipe_id)
        .map_err(|e| format!("Database error checking component recipe: {}", e))?;
    let component_recipe = match component_recipe {
        Some(r) => r,
        None => return Err(format!("Component recipe not found with id: {}", data.component_recipe_id)),
    };

    // Check for circular reference
    if would_create_cycle(&conn, data.recipe_id, data.component_recipe_id)
        .map_err(|e| format!("Failed to check for circular reference: {}", e))?
    {
        return Err(format!(
            "Cannot add component: would create circular reference (recipe {} already uses recipe {} directly or indirectly)",
            data.component_recipe_id, data.recipe_id
        ));
    }

    // Check if component already exists
    let existing = RecipeComponent::get_for_recipe(&conn, data.recipe_id)
        .map_err(|e| format!("Database error checking existing components: {}", e))?;
    if existing.iter().any(|c| c.component_recipe_id == data.component_recipe_id) {
        return Err(format!(
            "Recipe {} is already a component of recipe {}. Use update_recipe_component to modify servings.",
            data.component_recipe_id, data.recipe_id
        ));
    }

    let component = RecipeComponent::create(&conn, &data)
        .map_err(|e| format!("Failed to add component: {}", e))?;

    // Recalculate nutrition
    recalculate_recipe_nutrition(&conn, data.recipe_id)
        .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;

    Ok(AddComponentResponse {
        id: component.id,
        recipe_id: component.recipe_id,
        component_recipe_id: component.component_recipe_id,
        component_recipe_name: component_recipe.name,
        servings: component.servings,
    })
}

/// Update a recipe component's servings
pub fn update_recipe_component(
    db: &Database,
    id: i64,
    data: RecipeComponentUpdate,
) -> Result<Option<RecipeComponent>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get recipe_id before update for recalculation
    let recipe_id = RecipeComponent::get_recipe_id(&conn, id)
        .map_err(|e| format!("Failed to get recipe: {}", e))?;

    let updated = RecipeComponent::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update component: {}", e))?;

    // Recalculate nutrition if update succeeded
    if let (Some(_), Some(recipe_id)) = (&updated, recipe_id) {
        recalculate_recipe_nutrition(&conn, recipe_id)
            .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;
    }

    Ok(updated)
}

/// Remove a component from a recipe
pub fn remove_recipe_component(db: &Database, id: i64) -> Result<bool, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get recipe_id before delete for recalculation
    let recipe_id = RecipeComponent::get_recipe_id(&conn, id)
        .map_err(|e| format!("Failed to get recipe: {}", e))?;

    let deleted = RecipeComponent::delete(&conn, id)
        .map_err(|e| format!("Failed to remove component: {}", e))?;

    // Recalculate nutrition if delete succeeded
    if deleted {
        if let Some(recipe_id) = recipe_id {
            recalculate_recipe_nutrition(&conn, recipe_id)
                .map_err(|e| format!("Failed to recalculate nutrition: {}", e))?;
        }
    }

    Ok(deleted)
}
