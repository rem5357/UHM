//! Recipe Component model
//!
//! Allows recipes to use other recipes as components.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::db::DbResult;

/// A recipe component linking one recipe to another as an ingredient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeComponent {
    pub id: i64,
    pub recipe_id: i64,
    pub component_recipe_id: i64,
    pub servings: f64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Recipe component with details about the component recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeComponentDetail {
    pub id: i64,
    pub component_recipe_id: i64,
    pub component_recipe_name: String,
    pub servings: f64,
    pub notes: Option<String>,
}

/// Data for adding a component to a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeComponentCreate {
    pub recipe_id: i64,
    pub component_recipe_id: i64,
    pub servings: f64,
    pub notes: Option<String>,
}

/// Data for updating a recipe component
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecipeComponentUpdate {
    pub servings: Option<f64>,
    pub notes: Option<String>,
}

impl RecipeComponent {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            recipe_id: row.get("recipe_id")?,
            component_recipe_id: row.get("component_recipe_id")?,
            servings: row.get("servings")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Add a component recipe to a recipe
    pub fn create(conn: &Connection, data: &RecipeComponentCreate) -> DbResult<Self> {
        // Check for circular reference before creating
        if would_create_cycle(conn, data.recipe_id, data.component_recipe_id)? {
            return Err(crate::db::DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                "Adding this component would create a circular reference".to_string()
            )));
        }

        conn.execute(
            r#"
            INSERT INTO recipe_components (recipe_id, component_recipe_id, servings, notes)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                data.recipe_id,
                data.component_recipe_id,
                data.servings,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a component by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM recipe_components WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all components for a recipe
    pub fn get_for_recipe(conn: &Connection, recipe_id: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM recipe_components WHERE recipe_id = ?1 ORDER BY id"
        )?;

        let components = stmt
            .query_map([recipe_id], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(components)
    }

    /// Get components with recipe details for a recipe
    pub fn get_details_for_recipe(conn: &Connection, recipe_id: i64) -> DbResult<Vec<RecipeComponentDetail>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT rc.id, rc.component_recipe_id, r.name as component_recipe_name,
                   rc.servings, rc.notes
            FROM recipe_components rc
            INNER JOIN recipes r ON rc.component_recipe_id = r.id
            WHERE rc.recipe_id = ?1
            ORDER BY rc.id
            "#
        )?;

        let details = stmt
            .query_map([recipe_id], |row| {
                Ok(RecipeComponentDetail {
                    id: row.get("id")?,
                    component_recipe_id: row.get("component_recipe_id")?,
                    component_recipe_name: row.get("component_recipe_name")?,
                    servings: row.get("servings")?,
                    notes: row.get("notes")?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(details)
    }

    /// Update a component
    pub fn update(conn: &Connection, id: i64, data: &RecipeComponentUpdate) -> DbResult<Option<Self>> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(servings) = data.servings {
            updates.push(format!("servings = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(servings));
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
            "UPDATE recipe_components SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Delete a component
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        let rows = conn.execute("DELETE FROM recipe_components WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Get the recipe_id for a component
    pub fn get_recipe_id(conn: &Connection, id: i64) -> DbResult<Option<i64>> {
        let result: Result<i64, _> = conn.query_row(
            "SELECT recipe_id FROM recipe_components WHERE id = ?1",
            [id],
            |row| row.get(0),
        );
        match result {
            Ok(recipe_id) => Ok(Some(recipe_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get recipes that use this recipe as a component
    pub fn get_parent_recipe_ids(conn: &Connection, component_recipe_id: i64) -> DbResult<Vec<i64>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT recipe_id FROM recipe_components WHERE component_recipe_id = ?1"
        )?;

        let ids = stmt
            .query_map([component_recipe_id], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;

        Ok(ids)
    }
}

/// Check if adding component_recipe_id to recipe_id would create a circular reference
pub fn would_create_cycle(conn: &Connection, recipe_id: i64, component_recipe_id: i64) -> DbResult<bool> {
    // If we're adding component_recipe_id as a component of recipe_id,
    // we need to check if recipe_id is reachable from component_recipe_id
    // (i.e., component_recipe_id already uses recipe_id directly or indirectly)

    let mut visited = HashSet::new();
    let mut to_check = vec![component_recipe_id];

    while let Some(current) = to_check.pop() {
        if current == recipe_id {
            return Ok(true); // Found a cycle!
        }

        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Get all components of the current recipe
        let components = RecipeComponent::get_for_recipe(conn, current)?;
        for comp in components {
            to_check.push(comp.component_recipe_id);
        }
    }

    Ok(false)
}

/// Get all component recipe IDs recursively (for nutrition calculation)
pub fn get_all_component_ids(conn: &Connection, recipe_id: i64) -> DbResult<Vec<i64>> {
    let mut all_ids = Vec::new();
    let mut visited = HashSet::new();
    let mut to_check = vec![recipe_id];

    while let Some(current) = to_check.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        let components = RecipeComponent::get_for_recipe(conn, current)?;
        for comp in components {
            all_ids.push(comp.component_recipe_id);
            to_check.push(comp.component_recipe_id);
        }
    }

    Ok(all_ids)
}
