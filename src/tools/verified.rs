//! Verified Food Item Pipeline
//!
//! Tiered source resolution with sanity checks for creating food items
//! with provenance tracking. Prevents the "Bacon Incident" — food items
//! created from AI training data with no verification or source tracking.
//!
//! Source resolution tiers:
//! 1. Label photo (Sonnet vision extraction)
//! 2. USDA FoodData Central API lookup
//! 3. AI estimate with 80% rule (Sonnet)
//!
//! Sanity checks (local heuristics, no Opus needed):
//! - Macro math: |actual - (P*4+C*4+F*9)| > 15% tolerance
//! - Sodium threshold: >1000mg/100g flags for review
//! - Cross-reference: compare to similar items in DB

use serde::Serialize;

use crate::db::Database;
use crate::models::{FoodItem, FoodItemCreate, Preference};
use super::ai_client::{AnthropicClient, ContentBlock};
use super::food_items;
use super::usda_client::UsdaClient;

/// Extracted nutrition data (per serving or per 100g)
#[derive(Debug, Clone, Default)]
struct ExtractedNutrition {
    name: String,
    brand: Option<String>,
    serving_size: f64,
    serving_unit: String,
    calories: f64,
    protein: f64,
    carbs: f64,
    fat: f64,
    fiber: f64,
    sodium: f64,
    sugar: f64,
    saturated_fat: f64,
    cholesterol: f64,
    source: String,
    source_detail: String,
}

/// Sanity check flag
#[derive(Debug, Serialize)]
pub struct SanityFlag {
    pub check: String,
    pub message: String,
    pub severity: String, // "warning" or "error"
}

/// Result of the verified pipeline
#[derive(Debug, Serialize)]
pub struct VerifiedPipelineResult {
    pub status: String, // "created" or "review_needed"
    pub food_item_id: Option<i64>,
    pub name: String,
    pub brand: Option<String>,
    pub source: String,
    pub source_detail: String,
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
    pub sanity_flags: Vec<SanityFlag>,
    pub suggestions: Vec<String>,
}

/// Run the verified food item pipeline
pub fn run_verified_pipeline(
    db: &Database,
    description: &str,
    image_base64: Option<&str>,
    category: Option<&str>,
    brand_hint: Option<&str>,
) -> Result<VerifiedPipelineResult, String> {
    let ai_client = AnthropicClient::new();
    let usda_client = UsdaClient::new();

    // Step 1: Source resolution — tiered approach
    let extracted = resolve_source(
        &ai_client,
        &usda_client,
        db,
        description,
        image_base64,
        category,
        brand_hint,
    )?;

    // Step 2: Normalize units (solids → per 100g, liquids → per 100ml, countables → per 1 count)
    let normalized = normalize_units(&extracted, category);

    // Step 3: Sanity checks
    let flags = run_sanity_checks(db, &normalized)?;

    // Step 4: Decide whether to create or return for review
    let has_errors = flags.iter().any(|f| f.severity == "error");

    if has_errors {
        // Return for review with flags and suggestions
        let mut suggestions = Vec::new();
        for flag in &flags {
            if flag.check == "sodium" {
                suggestions.push("Check if this is raw vs cooked data. Raw meats have significantly higher sodium.".to_string());
            }
            if flag.check == "macro_math" {
                suggestions.push("Verify the calorie count matches the macronutrient breakdown.".to_string());
            }
        }

        Ok(VerifiedPipelineResult {
            status: "review_needed".to_string(),
            food_item_id: None,
            name: normalized.name.clone(),
            brand: normalized.brand.clone(),
            source: normalized.source.clone(),
            source_detail: normalized.source_detail.clone(),
            serving_size: normalized.serving_size,
            serving_unit: normalized.serving_unit.clone(),
            calories: normalized.calories,
            protein: normalized.protein,
            carbs: normalized.carbs,
            fat: normalized.fat,
            fiber: normalized.fiber,
            sodium: normalized.sodium,
            sugar: normalized.sugar,
            saturated_fat: normalized.saturated_fat,
            cholesterol: normalized.cholesterol,
            sanity_flags: flags,
            suggestions,
        })
    } else {
        // Create the food item
        let create_data = FoodItemCreate {
            name: normalized.name.clone(),
            brand: normalized.brand.clone(),
            serving_size: normalized.serving_size,
            serving_unit: normalized.serving_unit.clone(),
            calories: normalized.calories,
            protein: normalized.protein,
            carbs: normalized.carbs,
            fat: normalized.fat,
            fiber: normalized.fiber,
            sodium: normalized.sodium,
            sugar: normalized.sugar,
            saturated_fat: normalized.saturated_fat,
            cholesterol: normalized.cholesterol,
            preference: Preference::Neutral,
            notes: None,
            base_unit_type: None,
            grams_per_serving: None,
            ml_per_serving: None,
            source: Some(normalized.source.clone()),
            source_detail: Some(normalized.source_detail.clone()),
            ww_zero_point: None,
            ww_source: None,
            ww_points_override: None,
            scoop_grams: None,
        };

        let result = food_items::add_food_item(db, create_data)
            .map_err(|e| format!("Failed to create food item: {}", e))?;

        Ok(VerifiedPipelineResult {
            status: "created".to_string(),
            food_item_id: Some(result.id),
            name: normalized.name,
            brand: normalized.brand,
            source: normalized.source,
            source_detail: normalized.source_detail,
            serving_size: normalized.serving_size,
            serving_unit: normalized.serving_unit,
            calories: normalized.calories,
            protein: normalized.protein,
            carbs: normalized.carbs,
            fat: normalized.fat,
            fiber: normalized.fiber,
            sodium: normalized.sodium,
            sugar: normalized.sugar,
            saturated_fat: normalized.saturated_fat,
            cholesterol: normalized.cholesterol,
            sanity_flags: flags,
            suggestions: Vec::new(),
        })
    }
}

/// Tiered source resolution
fn resolve_source(
    ai_client: &Option<AnthropicClient>,
    usda_client: &Option<UsdaClient>,
    db: &Database,
    description: &str,
    image_base64: Option<&str>,
    category: Option<&str>,
    brand_hint: Option<&str>,
) -> Result<ExtractedNutrition, String> {
    // Tier 1: Label photo (if provided)
    if let Some(base64) = image_base64 {
        if let Some(client) = ai_client {
            if let Ok(extracted) = extract_from_label_photo(client, base64, description) {
                return Ok(extracted);
            }
        }
    }

    // Tier 2: USDA API lookup
    if let Some(client) = usda_client {
        if let Ok(Some(extracted)) = search_usda_nutrition(client, description, brand_hint) {
            return Ok(extracted);
        }
    }

    // Tier 3: AI estimate with 80% rule
    if let Some(client) = ai_client {
        return estimate_from_similar(client, db, description, category, brand_hint);
    }

    Err("No source available: ANTHROPIC_API_KEY not set and USDA_API_KEY not set".to_string())
}

/// Tier 1: Extract nutrition from a label photo using Sonnet vision
fn extract_from_label_photo(
    client: &AnthropicClient,
    base64: &str,
    description: &str,
) -> Result<ExtractedNutrition, String> {
    let system = "You are a nutrition label reader. Extract nutritional information from food label photos. \
                  Return ONLY valid JSON with these fields: name, brand, serving_size (number), serving_unit (string), \
                  calories, protein, carbs, fat, fiber, sodium, sugar, saturated_fat, cholesterol. \
                  All nutrition values should be per the serving size shown on the label. \
                  Sodium should be in milligrams. Cholesterol in milligrams. All others in grams except calories.";

    let prompt = format!(
        "Extract the nutrition facts from this food label. The product is: {}\n\
         Return ONLY a JSON object, no markdown formatting.",
        description
    );

    let content = vec![
        ContentBlock::Image {
            base64: base64.to_string(),
            media_type: "image/jpeg".to_string(),
        },
        ContentBlock::Text(prompt),
    ];

    let response = client.call_sonnet(system, &content, 500)?;

    // Parse the JSON response
    let clean = response
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json: serde_json::Value = serde_json::from_str(clean)
        .map_err(|e| format!("Failed to parse label extraction: {}", e))?;

    Ok(ExtractedNutrition {
        name: json["name"].as_str().unwrap_or(description).to_string(),
        brand: json["brand"].as_str().map(|s| s.to_string()),
        serving_size: json["serving_size"].as_f64().unwrap_or(100.0),
        serving_unit: json["serving_unit"].as_str().unwrap_or("g").to_string(),
        calories: json["calories"].as_f64().unwrap_or(0.0),
        protein: json["protein"].as_f64().unwrap_or(0.0),
        carbs: json["carbs"].as_f64().unwrap_or(0.0),
        fat: json["fat"].as_f64().unwrap_or(0.0),
        fiber: json["fiber"].as_f64().unwrap_or(0.0),
        sodium: json["sodium"].as_f64().unwrap_or(0.0),
        sugar: json["sugar"].as_f64().unwrap_or(0.0),
        saturated_fat: json["saturated_fat"].as_f64().unwrap_or(0.0),
        cholesterol: json["cholesterol"].as_f64().unwrap_or(0.0),
        source: "label_photo".to_string(),
        source_detail: "Sonnet vision extraction from nutrition label".to_string(),
    })
}

/// Tier 2: Search USDA FoodData Central for nutrition data
fn search_usda_nutrition(
    client: &UsdaClient,
    description: &str,
    brand_hint: Option<&str>,
) -> Result<Option<ExtractedNutrition>, String> {
    let query = if let Some(brand) = brand_hint {
        format!("{} {}", brand, description)
    } else {
        description.to_string()
    };

    let foods = client.search_foods(&query, 5)?;

    if foods.is_empty() {
        return Ok(None);
    }

    // Use the first result
    let food = &foods[0];
    let nutrition = UsdaClient::extract_nutrition(food);

    let serving_size = food.serving_size.unwrap_or(100.0);
    let serving_unit = food
        .serving_size_unit
        .as_deref()
        .unwrap_or("g")
        .to_string();

    let brand = food
        .brand_owner
        .as_ref()
        .or(food.brand_name.as_ref())
        .cloned();

    Ok(Some(ExtractedNutrition {
        name: food.description.clone(),
        brand,
        serving_size,
        serving_unit,
        calories: nutrition.calories,
        protein: nutrition.protein,
        carbs: nutrition.carbs,
        fat: nutrition.fat,
        fiber: nutrition.fiber,
        sodium: nutrition.sodium,
        sugar: nutrition.sugar,
        saturated_fat: nutrition.saturated_fat,
        cholesterol: nutrition.cholesterol,
        source: "usda".to_string(),
        source_detail: format!("USDA FDC ID: {}", food.fdc_id),
    }))
}

/// Tier 3: AI estimate using Sonnet with the 80% rule
fn estimate_from_similar(
    client: &AnthropicClient,
    db: &Database,
    description: &str,
    category: Option<&str>,
    brand_hint: Option<&str>,
) -> Result<ExtractedNutrition, String> {
    // Get similar items from DB for context
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;
    let similar = FoodItem::search(&conn, description, 5).unwrap_or_default();

    let mut context = String::new();
    if !similar.is_empty() {
        context.push_str("Similar items in database for reference:\n");
        for item in &similar {
            let brand_str = item.brand.as_deref().unwrap_or("");
            context.push_str(&format!(
                "- {} {} (per {}{}): cal={}, protein={}g, carbs={}g, fat={}g, sodium={}mg\n",
                brand_str, item.name, item.serving_size, item.serving_unit,
                item.nutrition.calories, item.nutrition.protein,
                item.nutrition.carbs, item.nutrition.fat, item.nutrition.sodium,
            ));
        }
    }

    let category_hint = category.unwrap_or("unknown");
    let brand_str = brand_hint.unwrap_or("unknown brand");

    let system = "You are a nutrition estimation expert. When estimating nutrition values, apply the 80% rule: \
                  estimate conservatively at ~80% of what you think the value might be, especially for sodium \
                  and calories. This prevents overestimation from training data that may mix raw/cooked values. \
                  Return ONLY valid JSON.";

    let prompt = format!(
        "Estimate nutritional data for: \"{}\"\n\
         Brand hint: {}\n\
         Category: {} (solid=per 100g, liquid=per 100ml, countable=per 1 count)\n\
         {}\n\
         IMPORTANT: Apply the 80% rule for sodium and calories — estimate conservatively.\n\
         For solids, normalize to per 100g. For liquids, per 100ml. For countables, per 1 count.\n\n\
         Return ONLY a JSON object with: name, brand, serving_size, serving_unit, \
         calories, protein, carbs, fat, fiber, sodium, sugar, saturated_fat, cholesterol",
        description, brand_str, category_hint, context
    );

    let response = client.call_sonnet(system, &[ContentBlock::Text(prompt)], 500)?;

    let clean = response
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json: serde_json::Value = serde_json::from_str(clean)
        .map_err(|e| format!("Failed to parse estimate: {}", e))?;

    Ok(ExtractedNutrition {
        name: json["name"].as_str().unwrap_or(description).to_string(),
        brand: json["brand"].as_str()
            .filter(|s| !s.is_empty() && *s != "unknown brand")
            .map(|s| s.to_string())
            .or_else(|| brand_hint.map(|s| s.to_string())),
        serving_size: json["serving_size"].as_f64().unwrap_or(100.0),
        serving_unit: json["serving_unit"].as_str().unwrap_or("g").to_string(),
        calories: json["calories"].as_f64().unwrap_or(0.0),
        protein: json["protein"].as_f64().unwrap_or(0.0),
        carbs: json["carbs"].as_f64().unwrap_or(0.0),
        fat: json["fat"].as_f64().unwrap_or(0.0),
        fiber: json["fiber"].as_f64().unwrap_or(0.0),
        sodium: json["sodium"].as_f64().unwrap_or(0.0),
        sugar: json["sugar"].as_f64().unwrap_or(0.0),
        saturated_fat: json["saturated_fat"].as_f64().unwrap_or(0.0),
        cholesterol: json["cholesterol"].as_f64().unwrap_or(0.0),
        source: "estimate".to_string(),
        source_detail: "Sonnet estimate with 80% rule".to_string(),
    })
}

/// Normalize units: solids → per 100g, liquids → per 100ml, countables → per 1 count
fn normalize_units(data: &ExtractedNutrition, category: Option<&str>) -> ExtractedNutrition {
    let mut result = data.clone();

    // Detect category from serving_unit if not provided
    let cat = category.unwrap_or_else(|| {
        let unit = data.serving_unit.to_lowercase();
        if unit == "ml" || unit == "fl oz" || unit == "fl_oz" {
            "liquid"
        } else if unit == "each" || unit == "count" || unit == "piece" || unit == "can" || unit == "bottle" {
            "countable"
        } else {
            "solid"
        }
    });

    match cat {
        "solid" => {
            // Normalize to per 100g
            if data.serving_unit.to_lowercase() == "g" && (data.serving_size - 100.0).abs() > 0.01 {
                let factor = 100.0 / data.serving_size;
                scale_nutrition(&mut result, factor);
                result.serving_size = 100.0;
                result.serving_unit = "g".to_string();
            }
        }
        "liquid" => {
            // Normalize to per 100ml
            if data.serving_unit.to_lowercase() == "ml" && (data.serving_size - 100.0).abs() > 0.01 {
                let factor = 100.0 / data.serving_size;
                scale_nutrition(&mut result, factor);
                result.serving_size = 100.0;
                result.serving_unit = "ml".to_string();
            }
        }
        "countable" => {
            // Keep as-is (per 1 count)
            // Already normalized if serving_size == 1
        }
        _ => {}
    }

    result
}

/// Scale all nutrition values by a factor
fn scale_nutrition(data: &mut ExtractedNutrition, factor: f64) {
    data.calories *= factor;
    data.protein *= factor;
    data.carbs *= factor;
    data.fat *= factor;
    data.fiber *= factor;
    data.sodium *= factor;
    data.sugar *= factor;
    data.saturated_fat *= factor;
    data.cholesterol *= factor;
}

/// Run all sanity checks
fn run_sanity_checks(db: &Database, data: &ExtractedNutrition) -> Result<Vec<SanityFlag>, String> {
    let mut flags = Vec::new();

    // Check 1: Macro math
    if let Some(flag) = sanity_check_macro_math(data) {
        flags.push(flag);
    }

    // Check 2: Sodium threshold
    if let Some(flag) = sanity_check_sodium(data) {
        flags.push(flag);
    }

    // Check 3: Cross-reference against DB
    flags.extend(sanity_check_cross_reference(db, data)?);

    Ok(flags)
}

/// Macro math check: |actual_cal - (P*4 + C*4 + F*9)| > 15% tolerance
fn sanity_check_macro_math(data: &ExtractedNutrition) -> Option<SanityFlag> {
    if data.calories < 1.0 {
        return None; // Can't check zero-calorie items
    }

    let calculated = data.protein * 4.0 + data.carbs * 4.0 + data.fat * 9.0;
    let diff_pct = ((data.calories - calculated) / data.calories).abs() * 100.0;

    if diff_pct > 15.0 {
        Some(SanityFlag {
            check: "macro_math".to_string(),
            message: format!(
                "Calorie mismatch: stated={:.0}, calculated from macros={:.0} (P*4+C*4+F*9), diff={:.1}%",
                data.calories, calculated, diff_pct
            ),
            severity: if diff_pct > 30.0 { "error" } else { "warning" }.to_string(),
        })
    } else {
        None
    }
}

/// Sodium threshold check: >1000mg per 100g is suspicious
fn sanity_check_sodium(data: &ExtractedNutrition) -> Option<SanityFlag> {
    // Normalize to per 100g/100ml for comparison
    let sodium_per_100 = if data.serving_size > 0.0 {
        data.sodium * (100.0 / data.serving_size)
    } else {
        data.sodium
    };

    if sodium_per_100 > 1000.0 {
        Some(SanityFlag {
            check: "sodium".to_string(),
            message: format!(
                "High sodium: {:.0}mg per 100{} — may be raw/uncooked data or cured/processed. \
                 Verify this is for the preparation method intended.",
                sodium_per_100,
                data.serving_unit
            ),
            severity: if sodium_per_100 > 2000.0 { "error" } else { "warning" }.to_string(),
        })
    } else {
        None
    }
}

/// Cross-reference against similar items in the database
fn sanity_check_cross_reference(
    db: &Database,
    data: &ExtractedNutrition,
) -> Result<Vec<SanityFlag>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;
    let similar = FoodItem::search(&conn, &data.name, 5).unwrap_or_default();

    if similar.is_empty() {
        return Ok(Vec::new());
    }

    let mut flags = Vec::new();

    // Compare calories against similar items
    let avg_calories: f64 = similar.iter().map(|i| i.nutrition.calories).sum::<f64>() / similar.len() as f64;
    if avg_calories > 0.0 {
        let diff_pct = ((data.calories - avg_calories) / avg_calories).abs() * 100.0;
        if diff_pct > 50.0 {
            flags.push(SanityFlag {
                check: "cross_reference".to_string(),
                message: format!(
                    "Calories ({:.0}) differ by {:.0}% from {} similar items in DB (avg {:.0})",
                    data.calories, diff_pct, similar.len(), avg_calories
                ),
                severity: "warning".to_string(),
            });
        }
    }

    // Compare sodium against similar items
    let avg_sodium: f64 = similar.iter().map(|i| i.nutrition.sodium).sum::<f64>() / similar.len() as f64;
    if avg_sodium > 0.0 {
        let diff_pct = ((data.sodium - avg_sodium) / avg_sodium).abs() * 100.0;
        if diff_pct > 100.0 {
            flags.push(SanityFlag {
                check: "cross_reference_sodium".to_string(),
                message: format!(
                    "Sodium ({:.0}mg) differs by {:.0}% from {} similar items in DB (avg {:.0}mg)",
                    data.sodium, diff_pct, similar.len(), avg_sodium
                ),
                severity: "warning".to_string(),
            });
        }
    }

    Ok(flags)
}
