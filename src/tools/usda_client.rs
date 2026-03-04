//! USDA FoodData Central API client
//!
//! Searches the USDA FDC database for food nutritional data.
//! Requires USDA_API_KEY environment variable.

use serde::Deserialize;

/// A nutrient from the USDA API response
#[derive(Debug, Deserialize)]
pub struct UsdaNutrient {
    #[serde(rename = "nutrientName")]
    pub nutrient_name: String,
    pub value: f64,
    #[serde(rename = "unitName")]
    pub unit_name: String,
}

/// A single food result from USDA search
#[derive(Debug, Deserialize)]
pub struct UsdaFoodResult {
    #[serde(rename = "fdcId")]
    pub fdc_id: i64,
    pub description: String,
    #[serde(rename = "brandName")]
    pub brand_name: Option<String>,
    #[serde(rename = "brandOwner")]
    pub brand_owner: Option<String>,
    #[serde(rename = "servingSize")]
    pub serving_size: Option<f64>,
    #[serde(rename = "servingSizeUnit")]
    pub serving_size_unit: Option<String>,
    #[serde(rename = "foodNutrients", default)]
    pub food_nutrients: Vec<UsdaNutrient>,
}

/// USDA search API response
#[derive(Debug, Deserialize)]
pub struct UsdaSearchResponse {
    #[serde(default)]
    pub foods: Vec<UsdaFoodResult>,
    #[serde(rename = "totalHits", default)]
    pub total_hits: i64,
}

/// Extracted nutrition from a USDA result (per 100g)
#[derive(Debug, Default)]
pub struct UsdaNutrition {
    pub calories: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sodium: f64,
    pub sugar: f64,
    pub saturated_fat: f64,
    pub cholesterol: f64,
}

/// USDA FoodData Central API client
pub struct UsdaClient {
    client: reqwest::blocking::Client,
    api_key: String,
}

impl UsdaClient {
    /// Create a new client. Returns None if USDA_API_KEY not set.
    pub fn new() -> Option<Self> {
        let api_key = std::env::var("USDA_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .ok()?;
        Some(Self { client, api_key })
    }

    /// Search for foods matching a query
    pub fn search_foods(&self, query: &str, limit: usize) -> Result<Vec<UsdaFoodResult>, String> {
        let url = format!(
            "https://api.nal.usda.gov/fdc/v1/foods/search?api_key={}&query={}&pageSize={}",
            self.api_key,
            urlencoding::encode(query),
            limit
        );

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("USDA API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(format!("USDA API error: {}", status));
        }

        let search_response: UsdaSearchResponse = response
            .json()
            .map_err(|e| format!("Failed to parse USDA response: {}", e))?;

        Ok(search_response.foods)
    }

    /// Extract standardized nutrition from a USDA food result
    pub fn extract_nutrition(food: &UsdaFoodResult) -> UsdaNutrition {
        let mut nutrition = UsdaNutrition::default();

        for nutrient in &food.food_nutrients {
            match nutrient.nutrient_name.as_str() {
                "Energy" => nutrition.calories = nutrient.value,
                "Protein" => nutrition.protein = nutrient.value,
                "Carbohydrate, by difference" => nutrition.carbs = nutrient.value,
                "Total lipid (fat)" => nutrition.fat = nutrient.value,
                "Fiber, total dietary" => nutrition.fiber = nutrient.value,
                "Sodium, Na" => nutrition.sodium = nutrient.value,
                "Sugars, total including NLEA" | "Total Sugars" => nutrition.sugar = nutrient.value,
                "Fatty acids, total saturated" => nutrition.saturated_fat = nutrient.value,
                "Cholesterol" => nutrition.cholesterol = nutrient.value,
                _ => {}
            }
        }

        nutrition
    }
}
