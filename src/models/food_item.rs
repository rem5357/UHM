//! Food Item model
//!
//! Represents a food item with nutritional information.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use crate::nutrition::BaseUnitType;
use super::Nutrition;

/// Food preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Preference {
    Liked,
    Disliked,
    #[default]
    Neutral,
}

impl Preference {
    pub fn as_str(&self) -> &'static str {
        match self {
            Preference::Liked => "liked",
            Preference::Disliked => "disliked",
            Preference::Neutral => "neutral",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "liked" => Preference::Liked,
            "disliked" => Preference::Disliked,
            _ => Preference::Neutral,
        }
    }
}

/// A food item with nutritional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodItem {
    pub id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub serving_size: f64,
    pub serving_unit: String,
    pub nutrition: Nutrition,
    pub preference: Preference,
    pub notes: Option<String>,
    /// Base unit type for this food item (weight, volume, or count)
    pub base_unit_type: Option<BaseUnitType>,
    /// Grams per serving (for weight-based and count items with known weight)
    pub grams_per_serving: Option<f64>,
    /// Milliliters per serving (for volume-based items)
    pub ml_per_serving: Option<f64>,
    /// Source of nutritional data: "label_photo", "usda", "estimate", or NULL for legacy
    pub source: Option<String>,
    /// Free-text provenance details (USDA FDC ID, photo description, etc.)
    pub source_detail: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new food item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodItemCreate {
    pub name: String,
    pub brand: Option<String>,
    pub serving_size: f64,
    pub serving_unit: String,
    pub calories: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    #[serde(default)]
    pub fiber: f64,
    #[serde(default)]
    pub sodium: f64,
    #[serde(default)]
    pub sugar: f64,
    #[serde(default)]
    pub saturated_fat: f64,
    #[serde(default)]
    pub cholesterol: f64,
    #[serde(default)]
    pub preference: Preference,
    pub notes: Option<String>,
    /// Override auto-detected base unit type
    pub base_unit_type: Option<BaseUnitType>,
    /// Override auto-calculated grams per serving
    pub grams_per_serving: Option<f64>,
    /// Override auto-calculated ml per serving
    pub ml_per_serving: Option<f64>,
    /// Source of nutritional data
    pub source: Option<String>,
    /// Free-text provenance details
    pub source_detail: Option<String>,
}

/// Data for updating a food item
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FoodItemUpdate {
    pub name: Option<String>,
    pub brand: Option<String>,
    pub serving_size: Option<f64>,
    pub serving_unit: Option<String>,
    pub calories: Option<f64>,
    pub protein: Option<f64>,
    pub carbs: Option<f64>,
    pub fat: Option<f64>,
    pub fiber: Option<f64>,
    pub sodium: Option<f64>,
    pub sugar: Option<f64>,
    pub saturated_fat: Option<f64>,
    pub cholesterol: Option<f64>,
    pub preference: Option<Preference>,
    pub notes: Option<String>,
    /// Override base unit type
    pub base_unit_type: Option<BaseUnitType>,
    /// Override grams per serving
    pub grams_per_serving: Option<f64>,
    /// Override ml per serving
    pub ml_per_serving: Option<f64>,
    /// Update source of nutritional data
    pub source: Option<String>,
    /// Update provenance details
    pub source_detail: Option<String>,
}

impl FoodItem {
    /// Create a FoodItem from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        // Parse base_unit_type from string
        let base_unit_type: Option<BaseUnitType> = row
            .get::<_, Option<String>>("base_unit_type")?
            .and_then(|s| BaseUnitType::from_str(&s));

        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            brand: row.get("brand")?,
            serving_size: row.get("serving_size")?,
            serving_unit: row.get("serving_unit")?,
            nutrition: Nutrition {
                calories: row.get("calories")?,
                protein: row.get("protein")?,
                carbs: row.get("carbs")?,
                fat: row.get("fat")?,
                fiber: row.get("fiber")?,
                sodium: row.get("sodium")?,
                sugar: row.get("sugar")?,
                saturated_fat: row.get("saturated_fat")?,
                cholesterol: row.get("cholesterol")?,
            },
            preference: Preference::from_str(row.get::<_, String>("preference")?.as_str()),
            notes: row.get("notes")?,
            base_unit_type,
            grams_per_serving: row.get("grams_per_serving")?,
            ml_per_serving: row.get("ml_per_serving")?,
            source: row.get("source")?,
            source_detail: row.get("source_detail")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Insert a new food item into the database
    pub fn create(conn: &Connection, data: &FoodItemCreate) -> DbResult<Self> {
        use crate::nutrition::{
            calculate_grams_per_serving, calculate_ml_per_serving, infer_base_unit_type,
        };

        // Auto-calculate unit conversion fields if not provided
        let base_unit_type = data
            .base_unit_type
            .unwrap_or_else(|| infer_base_unit_type(&data.serving_unit));
        let grams_per_serving = data
            .grams_per_serving
            .or_else(|| calculate_grams_per_serving(data.serving_size, &data.serving_unit));
        let ml_per_serving = data
            .ml_per_serving
            .or_else(|| calculate_ml_per_serving(data.serving_size, &data.serving_unit));

        conn.execute(
            r#"
            INSERT INTO food_items (
                name, brand, serving_size, serving_unit,
                calories, protein, carbs, fat, fiber, sodium, sugar, saturated_fat, cholesterol,
                preference, notes, base_unit_type, grams_per_serving, ml_per_serving,
                source, source_detail
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
            "#,
            params![
                data.name,
                data.brand,
                data.serving_size,
                data.serving_unit,
                data.calories,
                data.protein,
                data.carbs,
                data.fat,
                data.fiber,
                data.sodium,
                data.sugar,
                data.saturated_fat,
                data.cholesterol,
                data.preference.as_str(),
                data.notes,
                base_unit_type.to_db_str(),
                grams_per_serving,
                ml_per_serving,
                data.source,
                data.source_detail,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get a food item by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM food_items WHERE id = ?1"
        )?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Search food items by name or brand (LIKE pattern matching)
    /// Also tries concatenated "brand name" for cross-column matching
    pub fn search(conn: &Connection, query: &str, limit: i64) -> DbResult<Vec<Self>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            r#"
            SELECT * FROM food_items
            WHERE name LIKE ?1 OR brand LIKE ?1
                OR (COALESCE(brand, '') || ' ' || name) LIKE ?1
            ORDER BY name ASC
            LIMIT ?2
            "#
        )?;

        let items = stmt
            .query_map([&search_pattern, &limit.to_string()], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// Search food items using FTS5 full-text search
    ///
    /// Tokenizes the query and searches for ALL words across the unified search_text column.
    /// The search_text column contains "brand name" concatenated, so cross-column queries
    /// like "Willa's oat milk" match naturally.
    /// Returns items ordered by FTS5 rank (relevance).
    pub fn search_fts(conn: &Connection, query: &str, limit: i64) -> DbResult<Vec<Self>> {
        // Build FTS5 query: split into words, add prefix matching
        let words: Vec<&str> = query.split_whitespace().collect();
        if words.is_empty() {
            return Ok(Vec::new());
        }

        // Create FTS5 query with prefix matching and implicit AND: "word1* word2*"
        // Space-separated prefix terms in FTS5 = all must match
        let fts_query = words
            .iter()
            .map(|w| format!("{}*", w.replace('\'', "''"))) // Escape single quotes
            .collect::<Vec<_>>()
            .join(" ");

        let sql = r#"
            SELECT f.*
            FROM food_items f
            JOIN food_items_fts fts ON f.id = fts.rowid
            WHERE food_items_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
        "#;

        let mut stmt = conn.prepare(sql)?;

        let items = stmt
            .query_map(params![fts_query, limit], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// List food items with optional filtering and sorting
    pub fn list(
        conn: &Connection,
        preference: Option<Preference>,
        sort_by: &str,
        sort_order: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<Self>> {
        let order = if sort_order.to_lowercase() == "desc" { "DESC" } else { "ASC" };
        let sort_col = match sort_by.to_lowercase().as_str() {
            "created_at" => "created_at",
            "calories" => "calories",
            _ => "name",
        };

        let sql = if preference.is_some() {
            format!(
                "SELECT * FROM food_items WHERE preference = ?1 ORDER BY {} {} LIMIT ?2 OFFSET ?3",
                sort_col, order
            )
        } else {
            format!(
                "SELECT * FROM food_items ORDER BY {} {} LIMIT ?1 OFFSET ?2",
                sort_col, order
            )
        };

        let mut stmt = conn.prepare(&sql)?;

        let items = if let Some(pref) = preference {
            stmt.query_map(params![pref.as_str(), limit, offset], Self::from_row)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![limit, offset], Self::from_row)?
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(items)
    }

    /// Update a food item
    pub fn update(conn: &Connection, id: i64, data: &FoodItemUpdate) -> DbResult<Option<Self>> {
        use crate::nutrition::{
            calculate_grams_per_serving, calculate_ml_per_serving, infer_base_unit_type,
        };

        // Get the current food item to determine if we need to recalculate unit fields
        let current = Self::get_by_id(conn, id)?;
        let current = match current {
            Some(c) => c,
            None => return Ok(None),
        };

        // Build dynamic UPDATE query
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        macro_rules! add_update {
            ($field:ident, $col:expr) => {
                if let Some(ref val) = data.$field {
                    updates.push(format!("{} = ?{}", $col, params_vec.len() + 1));
                    params_vec.push(Box::new(val.clone()));
                }
            };
        }

        add_update!(name, "name");
        add_update!(brand, "brand");
        add_update!(serving_size, "serving_size");
        add_update!(serving_unit, "serving_unit");
        add_update!(calories, "calories");
        add_update!(protein, "protein");
        add_update!(carbs, "carbs");
        add_update!(fat, "fat");
        add_update!(fiber, "fiber");
        add_update!(sodium, "sodium");
        add_update!(sugar, "sugar");
        add_update!(saturated_fat, "saturated_fat");
        add_update!(cholesterol, "cholesterol");
        add_update!(notes, "notes");
        add_update!(source, "source");
        add_update!(source_detail, "source_detail");

        if let Some(ref pref) = data.preference {
            updates.push(format!("preference = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(pref.as_str().to_string()));
        }

        // Handle unit conversion fields
        // If serving_size or serving_unit changed, recalculate unless explicitly overridden
        let serving_size = data.serving_size.unwrap_or(current.serving_size);
        let serving_unit = data
            .serving_unit
            .as_ref()
            .unwrap_or(&current.serving_unit);
        let serving_changed =
            data.serving_size.is_some() || data.serving_unit.is_some();

        // base_unit_type
        if let Some(ref but) = data.base_unit_type {
            updates.push(format!("base_unit_type = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(but.to_db_str().to_string()));
        } else if serving_changed {
            let inferred = infer_base_unit_type(serving_unit);
            updates.push(format!("base_unit_type = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(inferred.to_db_str().to_string()));
        }

        // grams_per_serving
        if let Some(gps) = data.grams_per_serving {
            updates.push(format!("grams_per_serving = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(gps));
        } else if serving_changed {
            let calculated = calculate_grams_per_serving(serving_size, serving_unit);
            updates.push(format!("grams_per_serving = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(calculated));
        }

        // ml_per_serving
        if let Some(mps) = data.ml_per_serving {
            updates.push(format!("ml_per_serving = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(mps));
        } else if serving_changed {
            let calculated = calculate_ml_per_serving(serving_size, serving_unit);
            updates.push(format!("ml_per_serving = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(calculated));
        }

        if updates.is_empty() {
            return Ok(Some(current));
        }

        // Add updated_at
        updates.push("updated_at = datetime('now')".to_string());

        let sql = format!(
            "UPDATE food_items SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Get the count of recipes using this food item
    pub fn get_recipe_usage_count(conn: &Connection, id: i64) -> DbResult<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM recipe_ingredients WHERE food_item_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get the count of meal entries directly using this food item
    pub fn get_meal_usage_count(conn: &Connection, id: i64) -> DbResult<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM meal_entries WHERE food_item_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get total usage count (recipes + direct meal entries)
    pub fn get_usage_count(conn: &Connection, id: i64) -> DbResult<i64> {
        let recipe_count = Self::get_recipe_usage_count(conn, id)?;
        let meal_count = Self::get_meal_usage_count(conn, id)?;
        Ok(recipe_count + meal_count)
    }

    /// Get recipe names that use this food item
    pub fn get_used_in_recipes(conn: &Connection, id: i64) -> DbResult<Vec<String>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT r.name FROM recipes r
            INNER JOIN recipe_ingredients ri ON r.id = ri.recipe_id
            WHERE ri.food_item_id = ?1
            ORDER BY r.name
            "#
        )?;

        let names = stmt
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(names)
    }

    /// Get dates where this food item was logged directly as a meal
    pub fn get_used_in_meals(conn: &Connection, id: i64) -> DbResult<Vec<String>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT d.date FROM days d
            INNER JOIN meal_entries me ON d.id = me.day_id
            WHERE me.food_item_id = ?1
            ORDER BY d.date DESC
            "#
        )?;

        let dates = stmt
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(dates)
    }

    /// Get recipe IDs that use this food item (for recalculation)
    pub fn get_recipe_ids_using_item(conn: &Connection, id: i64) -> DbResult<Vec<i64>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT recipe_id FROM recipe_ingredients WHERE food_item_id = ?1"
        )?;

        let ids = stmt
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;

        Ok(ids)
    }

    /// Fuzzy search food items using Jaro-Winkler string similarity
    ///
    /// Loads all food items and scores them word-by-word against the query.
    /// An item matches if at least one query word has J-W score > 0.82 against
    /// any item word. Overall score = average of best per-query-word scores.
    /// Returns top `limit` matches sorted by score descending.
    pub fn search_fuzzy(conn: &Connection, query: &str, limit: i64) -> DbResult<Vec<Self>> {
        use strsim::jaro_winkler;

        let query_words: Vec<String> = query.split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();

        if query_words.is_empty() {
            return Ok(Vec::new());
        }

        // Load all food items
        let all_items = Self::list(conn, None, "name", "asc", 500, 0)?;

        const THRESHOLD: f64 = 0.82;

        let mut scored: Vec<(f64, &FoodItem)> = Vec::new();

        for item in &all_items {
            // Build word list from brand + name
            let text = format!(
                "{} {}",
                item.brand.as_deref().unwrap_or(""),
                &item.name
            ).to_lowercase();

            let item_words: Vec<&str> = text.split_whitespace().collect();

            if item_words.is_empty() {
                continue;
            }

            // For each query word, find best J-W score against any item word
            let mut best_scores: Vec<f64> = Vec::new();
            let mut has_match = false;

            for qw in &query_words {
                let best = item_words.iter()
                    .map(|iw| jaro_winkler(qw, iw) as f64)
                    .fold(0.0_f64, f64::max);

                if best > THRESHOLD {
                    has_match = true;
                }
                best_scores.push(best);
            }

            if has_match {
                let avg_score: f64 = best_scores.iter().sum::<f64>() / best_scores.len() as f64;
                scored.push((avg_score, item));
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Return top `limit` matches (clone the items)
        let results: Vec<Self> = scored.into_iter()
            .take(limit as usize)
            .map(|(_, item)| item.clone())
            .collect();

        Ok(results)
    }

    /// Count total food items (optionally filtered by preference)
    pub fn count(conn: &Connection, preference: Option<Preference>) -> DbResult<i64> {
        let count: i64 = if let Some(pref) = preference {
            conn.query_row(
                "SELECT COUNT(*) FROM food_items WHERE preference = ?1",
                [pref.as_str()],
                |row| row.get(0),
            )?
        } else {
            conn.query_row("SELECT COUNT(*) FROM food_items", [], |row| row.get(0))?
        };
        Ok(count)
    }

    /// Delete a food item (only if not used in any recipes)
    /// Returns Ok(true) if deleted, Ok(false) if not found
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Check if food item exists
        if Self::get_by_id(conn, id)?.is_none() {
            return Ok(false);
        }

        // Delete will fail if used in recipe_ingredients due to foreign key constraint
        let rows = conn.execute("DELETE FROM food_items WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }
}
