//! Recipe Ingredient model
//!
//! Represents ingredients in a recipe with quantity and unit.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use super::{FoodItem, Nutrition, Recipe};

/// A recipe ingredient linking a food item to a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub id: i64,
    pub recipe_id: i64,
    pub food_item_id: i64,
    pub quantity: f64,
    pub unit: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Recipe ingredient with food item details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredientDetail {
    pub id: i64,
    pub food_item_id: i64,
    pub food_item_name: String,
    pub quantity: f64,
    pub unit: String,
    pub notes: Option<String>,
}

/// Data for adding an ingredient to a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredientCreate {
    pub recipe_id: i64,
    pub food_item_id: i64,
    pub quantity: f64,
    pub unit: String,
    pub notes: Option<String>,
}

/// Data for updating a recipe ingredient
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecipeIngredientUpdate {
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub notes: Option<String>,
}

impl RecipeIngredient {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            recipe_id: row.get("recipe_id")?,
            food_item_id: row.get("food_item_id")?,
            quantity: row.get("quantity")?,
            unit: row.get("unit")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Add an ingredient to a recipe
    pub fn create(conn: &Connection, data: &RecipeIngredientCreate) -> DbResult<Self> {
        conn.execute(
            r#"
            INSERT INTO recipe_ingredients (recipe_id, food_item_id, quantity, unit, notes)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                data.recipe_id,
                data.food_item_id,
                data.quantity,
                data.unit,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get an ingredient by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM recipe_ingredients WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all ingredients for a recipe
    pub fn get_for_recipe(conn: &Connection, recipe_id: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM recipe_ingredients WHERE recipe_id = ?1 ORDER BY id"
        )?;

        let ingredients = stmt
            .query_map([recipe_id], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ingredients)
    }

    /// Get ingredients with food item details for a recipe
    pub fn get_details_for_recipe(conn: &Connection, recipe_id: i64) -> DbResult<Vec<RecipeIngredientDetail>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT ri.id, ri.food_item_id, fi.name as food_item_name,
                   ri.quantity, ri.unit, ri.notes
            FROM recipe_ingredients ri
            INNER JOIN food_items fi ON ri.food_item_id = fi.id
            WHERE ri.recipe_id = ?1
            ORDER BY ri.id
            "#
        )?;

        let details = stmt
            .query_map([recipe_id], |row| {
                Ok(RecipeIngredientDetail {
                    id: row.get("id")?,
                    food_item_id: row.get("food_item_id")?,
                    food_item_name: row.get("food_item_name")?,
                    quantity: row.get("quantity")?,
                    unit: row.get("unit")?,
                    notes: row.get("notes")?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(details)
    }

    /// Update an ingredient
    pub fn update(conn: &Connection, id: i64, data: &RecipeIngredientUpdate) -> DbResult<Option<Self>> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(qty) = data.quantity {
            updates.push(format!("quantity = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(qty));
        }
        if let Some(ref unit) = data.unit {
            updates.push(format!("unit = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(unit.clone()));
        }
        if let Some(ref notes) = data.notes {
            updates.push(format!("notes = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(notes.clone()));
        }

        if updates.is_empty() {
            return Self::get_by_id(conn, id);
        }

        updates.push("updated_at = datetime('now')".to_string());

        let sql = format!(
            "UPDATE recipe_ingredients SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Delete an ingredient
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        let rows = conn.execute("DELETE FROM recipe_ingredients WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Get the recipe_id for an ingredient
    pub fn get_recipe_id(conn: &Connection, id: i64) -> DbResult<Option<i64>> {
        let result: Result<i64, _> = conn.query_row(
            "SELECT recipe_id FROM recipe_ingredients WHERE id = ?1",
            [id],
            |row| row.get(0),
        );
        match result {
            Ok(recipe_id) => Ok(Some(recipe_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// Calculate total nutrition for a recipe based on its ingredients and component recipes
pub fn calculate_recipe_nutrition(conn: &Connection, recipe_id: i64) -> DbResult<Nutrition> {
    use crate::nutrition::calculate_nutrition_multiplier;

    let recipe = Recipe::get_by_id(conn, recipe_id)?
        .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

    let mut total = Nutrition::zero();

    // Sum nutrition from food item ingredients
    let ingredients = RecipeIngredient::get_for_recipe(conn, recipe_id)?;
    for ingredient in ingredients {
        let food_item = FoodItem::get_by_id(conn, ingredient.food_item_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        // Calculate multiplier using the new unit conversion system
        let multiplier = calculate_nutrition_multiplier(
            ingredient.quantity,
            &ingredient.unit,
            food_item.serving_size,
            &food_item.serving_unit,
            food_item.grams_per_serving,
            food_item.ml_per_serving,
        );

        total = total + food_item.nutrition.scale(multiplier);
    }

    // Sum nutrition from component recipes
    use super::recipe_component::RecipeComponent;
    let components = RecipeComponent::get_for_recipe(conn, recipe_id)?;
    for component in components {
        // Get the component recipe's cached nutrition (per serving)
        let component_recipe = Recipe::get_by_id(conn, component.component_recipe_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        // Scale by number of servings used
        total = total + component_recipe.cached_nutrition.scale(component.servings);
    }

    // Divide by servings to get per-serving nutrition
    let per_serving = total.scale(1.0 / recipe.servings_produced);

    Ok(per_serving)
}

/// Recalculate and update cached nutrition for a recipe
pub fn recalculate_recipe_nutrition(conn: &Connection, recipe_id: i64) -> DbResult<Nutrition> {
    let nutrition = calculate_recipe_nutrition(conn, recipe_id)?;
    Recipe::update_cached_nutrition(conn, recipe_id, &nutrition)?;
    Ok(nutrition)
}

/// Result of cascading recalculation
#[derive(Debug, Clone, Default)]
pub struct CascadeRecalculateResult {
    pub recipes_recalculated: i64,
    pub days_recalculated: i64,
}

/// Cascading recalculation: when a food item changes, recalculate all affected recipes and days
pub fn cascade_recalculate_from_food_item(
    conn: &Connection,
    food_item_id: i64,
) -> DbResult<CascadeRecalculateResult> {
    use std::collections::HashSet;
    use super::recipe_component::RecipeComponent;

    let mut result = CascadeRecalculateResult::default();

    // Step 1: Find all recipes directly using this food item
    let direct_recipe_ids: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT recipe_id FROM recipe_ingredients WHERE food_item_id = ?1",
        )?;
        let rows = stmt.query_map([food_item_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    if direct_recipe_ids.is_empty() {
        return Ok(result);
    }

    // Step 2: Collect all recipes affected (including parent recipes that use affected recipes as components)
    let mut all_affected: HashSet<i64> = HashSet::new();
    let mut to_process: Vec<i64> = direct_recipe_ids;

    while let Some(recipe_id) = to_process.pop() {
        if all_affected.insert(recipe_id) {
            // Find parent recipes that use this recipe as a component
            let parent_ids: Vec<i64> = {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT recipe_id FROM recipe_components WHERE component_recipe_id = ?1",
                )?;
                let rows = stmt.query_map([recipe_id], |row| row.get(0))?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            to_process.extend(parent_ids);
        }
    }

    // Step 3: Sort recipes by dependency order (leaf recipes first, then parents)
    // We need to recalculate in order so parent recipes get updated child values
    let sorted_recipes = topological_sort_recipes(conn, &all_affected)?;

    // Step 4: Recalculate all affected recipes
    for recipe_id in &sorted_recipes {
        recalculate_recipe_nutrition(conn, *recipe_id)?;
        result.recipes_recalculated += 1;
    }

    // Step 5: Find all days with meal entries using affected recipes or the food item directly
    let affected_day_ids: Vec<i64> = {
        let recipe_ids_str = sorted_recipes
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            "SELECT DISTINCT day_id FROM meal_entries WHERE recipe_id IN ({}) OR food_item_id = ?1",
            if recipe_ids_str.is_empty() {
                "-1".to_string() // No recipes, just check food_item_id
            } else {
                recipe_ids_str
            }
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([food_item_id], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // Step 6: Recalculate all affected days
    use super::meal_entry::recalculate_day_nutrition;
    for day_id in affected_day_ids {
        recalculate_day_nutrition(conn, day_id)?;
        result.days_recalculated += 1;
    }

    Ok(result)
}

/// Sort recipes in topological order (dependencies first)
fn topological_sort_recipes(
    conn: &Connection,
    recipe_ids: &std::collections::HashSet<i64>,
) -> DbResult<Vec<i64>> {
    use std::collections::{HashMap, HashSet, VecDeque};

    if recipe_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Build dependency graph: recipe_id -> set of component recipe IDs it depends on
    let mut dependencies: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut dependents: HashMap<i64, HashSet<i64>> = HashMap::new();

    for &recipe_id in recipe_ids {
        dependencies.entry(recipe_id).or_default();
        dependents.entry(recipe_id).or_default();
    }

    // Query component relationships
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

    let mut stmt = conn.prepare(&sql)?;
    let edges: Vec<(i64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (parent_id, child_id) in edges {
        dependencies.entry(parent_id).or_default().insert(child_id);
        dependents.entry(child_id).or_default().insert(parent_id);
    }

    // Kahn's algorithm for topological sort
    let mut result = Vec::new();
    let mut queue: VecDeque<i64> = VecDeque::new();

    // Start with recipes that have no dependencies (within our set)
    for &recipe_id in recipe_ids {
        if dependencies.get(&recipe_id).map_or(true, |d| d.is_empty()) {
            queue.push_back(recipe_id);
        }
    }

    while let Some(recipe_id) = queue.pop_front() {
        result.push(recipe_id);

        // Remove this recipe from dependents' dependency lists
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

    // If we didn't process all recipes, there's a cycle (shouldn't happen with proper constraints)
    if result.len() != recipe_ids.len() {
        // Fall back to arbitrary order - collect missing IDs first to avoid borrow conflict
        let missing: Vec<i64> = recipe_ids
            .iter()
            .copied()
            .filter(|id| !result.contains(id))
            .collect();
        result.extend(missing);
    }

    Ok(result)
}
