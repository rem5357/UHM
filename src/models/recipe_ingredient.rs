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

/// Calculate total nutrition for a recipe based on its ingredients
pub fn calculate_recipe_nutrition(conn: &Connection, recipe_id: i64) -> DbResult<Nutrition> {
    let recipe = Recipe::get_by_id(conn, recipe_id)?
        .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

    let ingredients = RecipeIngredient::get_for_recipe(conn, recipe_id)?;

    let mut total = Nutrition::zero();

    for ingredient in ingredients {
        let food_item = FoodItem::get_by_id(conn, ingredient.food_item_id)?
            .ok_or_else(|| crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        // Calculate multiplier based on ingredient unit type
        let multiplier = calculate_multiplier(
            ingredient.quantity,
            &ingredient.unit,
            food_item.serving_size,
            &food_item.serving_unit,
        );

        total = total + food_item.nutrition.scale(multiplier);
    }

    // Divide by servings to get per-serving nutrition
    let per_serving = total.scale(1.0 / recipe.servings_produced);

    Ok(per_serving)
}

/// Calculate the nutrition multiplier based on ingredient quantity and units
fn calculate_multiplier(
    quantity: f64,
    ingredient_unit: &str,
    serving_size: f64,
    serving_unit: &str,
) -> f64 {
    let unit_lower = ingredient_unit.to_lowercase();

    // If ingredient is specified in "servings", quantity IS the multiplier
    if unit_lower == "serving" || unit_lower == "servings" {
        return quantity;
    }

    // If units match, divide quantity by serving_size to get multiplier
    // e.g., 200g ingredient / 100g serving_size = 2.0 servings
    if unit_lower == serving_unit.to_lowercase() {
        return quantity / serving_size;
    }

    // For mismatched units, assume quantity represents servings
    // (user should use matching units for accuracy)
    quantity
}

/// Recalculate and update cached nutrition for a recipe
pub fn recalculate_recipe_nutrition(conn: &Connection, recipe_id: i64) -> DbResult<Nutrition> {
    let nutrition = calculate_recipe_nutrition(conn, recipe_id)?;
    Recipe::update_cached_nutrition(conn, recipe_id, &nutrition)?;
    Ok(nutrition)
}
