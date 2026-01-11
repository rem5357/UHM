//! Recipe model
//!
//! Represents a recipe with cached nutritional information.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use super::Nutrition;

/// A recipe with cached nutrition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: i64,
    pub name: String,
    pub servings_produced: f64,
    pub is_favorite: bool,
    pub cached_nutrition: Nutrition,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeCreate {
    pub name: String,
    #[serde(default = "default_servings")]
    pub servings_produced: f64,
    #[serde(default)]
    pub is_favorite: bool,
    pub notes: Option<String>,
}

fn default_servings() -> f64 {
    1.0
}

/// Data for updating a recipe
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecipeUpdate {
    pub name: Option<String>,
    pub servings_produced: Option<f64>,
    pub is_favorite: Option<bool>,
    pub notes: Option<String>,
}

impl Recipe {
    /// Create a Recipe from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            servings_produced: row.get("servings_produced")?,
            is_favorite: row.get::<_, i32>("is_favorite")? != 0,
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

    /// Insert a new recipe into the database
    pub fn create(conn: &Connection, data: &RecipeCreate) -> DbResult<Self> {
        conn.execute(
            r#"
            INSERT INTO recipes (name, servings_produced, is_favorite, notes)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                data.name,
                data.servings_produced,
                data.is_favorite as i32,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a recipe by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM recipes WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(recipe) => Ok(Some(recipe)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List recipes with optional filtering
    pub fn list(
        conn: &Connection,
        query: Option<&str>,
        favorites_only: bool,
        sort_by: &str,
        sort_order: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<Self>> {
        let order = if sort_order.to_lowercase() == "desc" { "DESC" } else { "ASC" };
        let sort_col = match sort_by.to_lowercase().as_str() {
            "created_at" => "created_at",
            "times_logged" => "id", // TODO: implement actual times_logged
            _ => "name",
        };

        let (sql, search_param) = match (query, favorites_only) {
            (Some(q), true) => (
                format!(
                    "SELECT * FROM recipes WHERE name LIKE ?1 AND is_favorite = 1 ORDER BY {} {} LIMIT ?2 OFFSET ?3",
                    sort_col, order
                ),
                Some(format!("%{}%", q)),
            ),
            (Some(q), false) => (
                format!(
                    "SELECT * FROM recipes WHERE name LIKE ?1 ORDER BY {} {} LIMIT ?2 OFFSET ?3",
                    sort_col, order
                ),
                Some(format!("%{}%", q)),
            ),
            (None, true) => (
                format!(
                    "SELECT * FROM recipes WHERE is_favorite = 1 ORDER BY {} {} LIMIT ?1 OFFSET ?2",
                    sort_col, order
                ),
                None,
            ),
            (None, false) => (
                format!(
                    "SELECT * FROM recipes ORDER BY {} {} LIMIT ?1 OFFSET ?2",
                    sort_col, order
                ),
                None,
            ),
        };

        let mut stmt = conn.prepare(&sql)?;

        let recipes = if let Some(pattern) = search_param {
            stmt.query_map(params![pattern, limit, offset], Self::from_row)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![limit, offset], Self::from_row)?
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(recipes)
    }

    /// Update a recipe (only if not used in meal_entries)
    pub fn update(conn: &Connection, id: i64, data: &RecipeUpdate) -> DbResult<Option<Self>> {
        // Check if used in meal entries
        let usage_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM meal_entries WHERE recipe_id = ?1",
            [id],
            |row| row.get(0),
        )?;

        if usage_count > 0 {
            return Ok(None);
        }

        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref name) = data.name {
            updates.push(format!("name = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(name.clone()));
        }
        if let Some(servings) = data.servings_produced {
            updates.push(format!("servings_produced = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(servings));
        }
        if let Some(is_fav) = data.is_favorite {
            updates.push(format!("is_favorite = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(is_fav as i32));
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
            "UPDATE recipes SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Update cached nutrition for a recipe
    pub fn update_cached_nutrition(conn: &Connection, id: i64, nutrition: &Nutrition) -> DbResult<()> {
        conn.execute(
            r#"
            UPDATE recipes SET
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
                id,
            ],
        )?;
        Ok(())
    }

    /// Count recipes
    pub fn count(conn: &Connection, favorites_only: bool) -> DbResult<i64> {
        let count: i64 = if favorites_only {
            conn.query_row(
                "SELECT COUNT(*) FROM recipes WHERE is_favorite = 1",
                [],
                |row| row.get(0),
            )?
        } else {
            conn.query_row("SELECT COUNT(*) FROM recipes", [], |row| row.get(0))?
        };
        Ok(count)
    }

    /// Get meal entry count for a recipe
    pub fn get_times_logged(conn: &Connection, id: i64) -> DbResult<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM meal_entries WHERE recipe_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Check if recipe is used as a component in other recipes
    pub fn get_component_usage_count(conn: &Connection, id: i64) -> DbResult<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM recipe_components WHERE component_recipe_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Delete a recipe (only if not logged and not used as component)
    /// Returns Ok(true) if deleted, Ok(false) if not found
    /// Returns Err with reason if deletion is blocked
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Check if recipe exists
        if Self::get_by_id(conn, id)?.is_none() {
            return Ok(false);
        }

        // Delete will cascade to recipe_ingredients
        // recipe_components has ON DELETE RESTRICT for component_recipe_id,
        // so it will fail if used as a component
        let rows = conn.execute("DELETE FROM recipes WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }
}
