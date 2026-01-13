//! UHM MCP Server Implementation
//!
//! Implements the MCP server with all UHM tools.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::db::Database;
use crate::models::{
    FoodItemCreate, FoodItemUpdate, Preference,
    RecipeCreate, RecipeUpdate, RecipeIngredientCreate, RecipeIngredientUpdate,
    RecipeComponentCreate, RecipeComponentUpdate,
    MedicationCreate, MedicationUpdate, MedType, DosageUnit,
};
use crate::tools::days;
use crate::tools::food_items;
use crate::tools::medications;
use crate::tools::recipes;
use crate::tools::status::StatusTracker;
use crate::tools::vitals;

/// Batch update state for efficient bulk food item updates
#[derive(Default)]
struct BatchUpdateState {
    /// Whether batch mode is active
    active: bool,
    /// Food item IDs that have been modified during this batch
    changed_food_item_ids: HashSet<i64>,
}

/// UHM MCP Service
#[derive(Clone)]
pub struct UhmService {
    status_tracker: Arc<Mutex<StatusTracker>>,
    database: Database,
    tool_router: ToolRouter<UhmService>,
    /// Batch update state for efficient bulk operations
    batch_state: Arc<std::sync::Mutex<BatchUpdateState>>,
}

impl UhmService {
    pub fn new(database_path: PathBuf, database: Database) -> Self {
        Self {
            status_tracker: Arc::new(Mutex::new(StatusTracker::new(database_path))),
            database,
            tool_router: Self::tool_router(),
            batch_state: Arc::new(std::sync::Mutex::new(BatchUpdateState::default())),
        }
    }
}

// ============================================================================
// Batch Update Response Structs
// ============================================================================

#[derive(Debug, Serialize)]
struct StartBatchUpdateResponse {
    success: bool,
    message: String,
    pending_items: i64,
}

#[derive(Debug, Serialize)]
struct FinishBatchUpdateResponse {
    success: bool,
    message: String,
    food_items_processed: i64,
    recipes_recalculated: i64,
    days_recalculated: i64,
}

// ============================================================================
// Food Item Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddFoodItemParams {
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
    pub preference: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchFoodItemsParams {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: i64,
}

fn default_search_limit() -> i64 { 20 }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetFoodItemParams {
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListFoodItemsParams {
    pub preference: Option<String>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    #[serde(default = "default_list_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_sort_by() -> String { "name".to_string() }
fn default_sort_order() -> String { "asc".to_string() }
fn default_list_limit() -> i64 { 50 }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateFoodItemParams {
    pub id: i64,
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
    pub preference: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteFoodItemParams {
    /// Food item ID to delete
    pub id: i64,
}

// ============================================================================
// Recipe Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateRecipeParams {
    /// Name of the recipe
    pub name: String,
    /// Number of servings this recipe produces (default 1.0)
    #[serde(default = "default_servings")]
    pub servings_produced: f64,
    /// Mark as favorite (default false)
    #[serde(default)]
    pub is_favorite: bool,
    /// Optional notes
    pub notes: Option<String>,
}

fn default_servings() -> f64 { 1.0 }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetRecipeParams {
    /// Recipe ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListRecipesParams {
    /// Search query for recipe name (optional)
    pub query: Option<String>,
    /// Only show favorites (default false)
    #[serde(default)]
    pub favorites_only: bool,
    /// Sort by: name, created_at, or times_logged (default name)
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    /// Sort order: asc or desc (default asc)
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    /// Maximum results (default 50, max 200)
    #[serde(default = "default_list_limit")]
    pub limit: i64,
    /// Offset for pagination (default 0)
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateRecipeParams {
    /// Recipe ID to update
    pub id: i64,
    /// New name (optional)
    pub name: Option<String>,
    /// New servings produced (optional)
    pub servings_produced: Option<f64>,
    /// New favorite status (optional)
    pub is_favorite: Option<bool>,
    /// New notes (optional)
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteRecipeParams {
    /// Recipe ID to delete
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddRecipeIngredientParams {
    /// Recipe ID to add ingredient to
    pub recipe_id: i64,
    /// Food item ID to add
    pub food_item_id: i64,
    /// Quantity of the ingredient
    pub quantity: f64,
    /// Unit for the quantity (should match food item's unit type)
    pub unit: String,
    /// Optional notes
    pub notes: Option<String>,
}

/// Single ingredient for batch add
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BatchIngredientParam {
    /// Food item ID to add
    pub food_item_id: i64,
    /// Quantity in grams (for solids) or ml (for liquids)
    pub quantity: f64,
    /// Unit: "g" for solids, "ml" for liquids, "servings" for count items
    pub unit: String,
    /// Optional notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddRecipeIngredientsBatchParams {
    /// Recipe ID to add ingredients to
    pub recipe_id: i64,
    /// Array of ingredients to add
    pub ingredients: Vec<BatchIngredientParam>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateRecipeIngredientParams {
    /// Recipe ingredient ID to update
    pub id: i64,
    /// New quantity (optional)
    pub quantity: Option<f64>,
    /// New unit (optional)
    pub unit: Option<String>,
    /// New notes (optional)
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveRecipeIngredientParams {
    /// Recipe ingredient ID to remove
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecalculateNutritionParams {
    /// Recipe ID to recalculate
    pub recipe_id: i64,
}

// ============================================================================
// Recipe Component Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddRecipeComponentParams {
    /// Parent recipe ID (the recipe to add the component to)
    pub recipe_id: i64,
    /// Component recipe ID (the recipe to use as an ingredient)
    pub component_recipe_id: i64,
    /// Number of servings of the component recipe to use (default 1.0)
    #[serde(default = "default_servings")]
    pub servings: f64,
    /// Optional notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateRecipeComponentParams {
    /// Recipe component ID to update
    pub id: i64,
    /// New servings (optional)
    pub servings: Option<f64>,
    /// New notes (optional)
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveRecipeComponentParams {
    /// Recipe component ID to remove
    pub id: i64,
}

// ============================================================================
// Day Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetOrCreateDayParams {
    /// Date in ISO format: YYYY-MM-DD
    pub date: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDayParams {
    /// Date in ISO format: YYYY-MM-DD
    pub date: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDaysParams {
    /// Start date (inclusive) - optional
    pub start_date: Option<String>,
    /// End date (inclusive) - optional
    pub end_date: Option<String>,
    /// Maximum results (default 50, max 200)
    #[serde(default = "default_list_limit")]
    pub limit: i64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateDayParams {
    /// Date in ISO format: YYYY-MM-DD
    pub date: String,
    /// Notes for the day
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteDayParams {
    /// Date in ISO format: YYYY-MM-DD
    pub date: String,
}

// ============================================================================
// Meal Entry Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LogMealParams {
    /// Date in ISO format: YYYY-MM-DD
    pub date: String,
    /// Meal type: breakfast, lunch, dinner, snack, or unspecified
    #[serde(default = "default_meal_type")]
    pub meal_type: String,
    /// Recipe ID (provide either recipe_id OR food_item_id, not both)
    pub recipe_id: Option<i64>,
    /// Food item ID (provide either recipe_id OR food_item_id, not both)
    pub food_item_id: Option<i64>,
    /// Number of servings consumed (default 1.0)
    #[serde(default = "default_servings")]
    pub servings: f64,
    /// Percentage eaten (0-100, default 100)
    pub percent_eaten: Option<f64>,
    /// Optional notes
    pub notes: Option<String>,
}

fn default_meal_type() -> String { "unspecified".to_string() }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetMealEntryParams {
    /// Meal entry ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateMealEntryParams {
    /// Meal entry ID
    pub id: i64,
    /// New meal type (optional)
    pub meal_type: Option<String>,
    /// New servings (optional)
    pub servings: Option<f64>,
    /// New percent eaten (optional)
    pub percent_eaten: Option<f64>,
    /// New notes (optional)
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteMealEntryParams {
    /// Meal entry ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecalculateDayNutritionParams {
    /// Date in ISO format: YYYY-MM-DD
    pub date: String,
}

// ============================================================================
// Medication Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddMedicationParams {
    /// Medication name (e.g., "Lisinopril", "Vitamin D3")
    pub name: String,
    /// Type: prescription, supplement, otc, natural, compound, medical_device, other
    pub med_type: String,
    /// Dosage amount (e.g., 10.0)
    pub dosage_amount: f64,
    /// Dosage unit: mg, mcg, g, ml, fl_oz, pill, tablet, capsule, spray, drop, patch, injection, unit, iu, puff, other
    pub dosage_unit: String,
    /// Instructions (e.g., "Take 1 tablet daily with food")
    pub instructions: Option<String>,
    /// Frequency (e.g., "twice daily", "PRN", "weekly")
    pub frequency: Option<String>,
    /// Prescribing doctor's name (for prescriptions)
    pub prescribing_doctor: Option<String>,
    /// Date prescribed (ISO format: YYYY-MM-DD)
    pub prescribed_date: Option<String>,
    /// Pharmacy name
    pub pharmacy: Option<String>,
    /// Prescription number
    pub rx_number: Option<String>,
    /// Number of refills remaining
    pub refills_remaining: Option<i32>,
    /// Date started taking (ISO format: YYYY-MM-DD)
    pub start_date: Option<String>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetMedicationParams {
    /// Medication ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListMedicationsParams {
    /// Only show active medications (default true)
    #[serde(default = "default_true")]
    pub active_only: bool,
    /// Filter by type: prescription, supplement, otc, natural, compound, medical_device, other
    pub med_type: Option<String>,
}

fn default_true() -> bool { true }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchMedicationsParams {
    /// Search query (matches name)
    pub query: String,
    /// Only show active medications (default true)
    #[serde(default = "default_true")]
    pub active_only: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateMedicationParams {
    /// Medication ID
    pub id: i64,
    /// REQUIRED: Must be true to confirm the update
    #[serde(default)]
    pub force: bool,
    /// New name
    pub name: Option<String>,
    /// New type
    pub med_type: Option<String>,
    /// New dosage amount
    pub dosage_amount: Option<f64>,
    /// New dosage unit
    pub dosage_unit: Option<String>,
    /// New instructions
    pub instructions: Option<String>,
    /// New frequency
    pub frequency: Option<String>,
    /// New prescribing doctor
    pub prescribing_doctor: Option<String>,
    /// New prescribed date
    pub prescribed_date: Option<String>,
    /// New pharmacy
    pub pharmacy: Option<String>,
    /// New rx number
    pub rx_number: Option<String>,
    /// New refills remaining
    pub refills_remaining: Option<i32>,
    /// New start date
    pub start_date: Option<String>,
    /// New notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeprecateMedicationParams {
    /// Medication ID
    pub id: i64,
    /// End date (defaults to today if not provided)
    pub end_date: Option<String>,
    /// Reason for discontinuing
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReactivateMedicationParams {
    /// Medication ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteMedicationParams {
    /// Medication ID
    pub id: i64,
    /// REQUIRED: Must be true to confirm deletion
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportMedicationsParams {
    /// Patient name to display on the document
    pub patient_name: String,
}

// ============================================================================
// Vital Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateVitalGroupParams {
    /// Description of the group (e.g., "BP & HR reading", "Post Exercise")
    pub description: Option<String>,
    /// Timestamp (defaults to now if not provided)
    pub timestamp: Option<String>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetVitalGroupParams {
    /// Vital group ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListVitalGroupsParams {
    /// Maximum number of groups to return
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateVitalGroupParams {
    /// Vital group ID
    pub id: i64,
    /// New description
    pub description: Option<String>,
    /// New notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteVitalGroupParams {
    /// Vital group ID (vitals will be unlinked but not deleted)
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddVitalParams {
    /// Vital type: weight, blood_pressure (bp), heart_rate (hr/pulse), oxygen_saturation (o2/spo2), glucose
    pub vital_type: String,
    /// Primary value (weight, systolic BP, heart rate, O2%, glucose)
    pub value1: f64,
    /// Secondary value (diastolic BP - required for blood_pressure)
    pub value2: Option<f64>,
    /// Unit (defaults to standard for vital type: lbs, mmHg, bpm, %, mg/dL)
    pub unit: Option<String>,
    /// Timestamp (defaults to now if not provided)
    pub timestamp: Option<String>,
    /// Group ID to associate with related readings
    pub group_id: Option<i64>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetVitalParams {
    /// Vital ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListVitalsByTypeParams {
    /// Vital type: weight, blood_pressure, heart_rate, oxygen_saturation, glucose
    pub vital_type: String,
    /// Maximum results
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListRecentVitalsParams {
    /// Maximum results (default 20)
    #[serde(default = "default_search_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListVitalsByDateRangeParams {
    /// Start date (ISO format: YYYY-MM-DD or full timestamp)
    pub start_date: String,
    /// End date (ISO format: YYYY-MM-DD or full timestamp)
    pub end_date: String,
    /// Filter by vital type (optional)
    pub vital_type: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateVitalParams {
    /// Vital ID
    pub id: i64,
    /// New primary value
    pub value1: Option<f64>,
    /// New secondary value (for blood pressure)
    pub value2: Option<f64>,
    /// New unit
    pub unit: Option<String>,
    /// New notes
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssignVitalToGroupParams {
    /// Vital ID
    pub vital_id: i64,
    /// Group ID (or null to remove from group)
    pub group_id: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteVitalParams {
    /// Vital ID
    pub id: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImportOmronBpCsvParams {
    /// Full path to the Omron CSV file (e.g., "C:\\Users\\name\\Downloads\\report.csv")
    pub file_path: String,
}

// ============================================================================
// Tool Implementations
// ============================================================================

#[tool_router]
impl UhmService {
    // --- Status ---

    #[tool(description = "Get the current status of the UHM service including build info, database status, and process information")]
    async fn uhm_status(&self) -> Result<CallToolResult, McpError> {
        let tracker = self.status_tracker.lock().await;
        let status = tracker.get_status();
        let json = serde_json::to_string_pretty(&status)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get step-by-step instructions for logging meals. Call this when starting a new food logging session or when unsure how to use the meal tracking tools.")]
    fn meal_instructions(&self) -> Result<CallToolResult, McpError> {
        use crate::tools::status::MEAL_INSTRUCTIONS;
        Ok(CallToolResult::success(vec![Content::text(MEAL_INSTRUCTIONS)]))
    }

    #[tool(description = "Get step-by-step instructions for managing medications. Call this when starting a medication tracking session or when unsure how to use the medication tools.")]
    fn medication_instructions(&self) -> Result<CallToolResult, McpError> {
        use crate::tools::status::MEDICATION_INSTRUCTIONS;
        Ok(CallToolResult::success(vec![Content::text(MEDICATION_INSTRUCTIONS)]))
    }

    // --- Food Items ---

    #[tool(description = "Create a new food item with nutritional information")]
    fn add_food_item(&self, Parameters(p): Parameters<AddFoodItemParams>) -> Result<CallToolResult, McpError> {
        let data = FoodItemCreate {
            name: p.name, brand: p.brand, serving_size: p.serving_size, serving_unit: p.serving_unit,
            calories: p.calories, protein: p.protein, carbs: p.carbs, fat: p.fat,
            fiber: p.fiber, sodium: p.sodium, sugar: p.sugar, saturated_fat: p.saturated_fat,
            cholesterol: p.cholesterol, preference: p.preference.as_deref().map(Preference::from_str).unwrap_or_default(),
            notes: p.notes,
            base_unit_type: None, grams_per_serving: None, ml_per_serving: None,
        };
        let result = food_items::add_food_item(&self.database, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Search for food items by name or brand")]
    fn search_food_items(&self, Parameters(p): Parameters<SearchFoodItemsParams>) -> Result<CallToolResult, McpError> {
        let result = food_items::search_food_items(&self.database, &p.query, p.limit).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get full details for a food item including nutritional data and recipe usage")]
    fn get_food_item(&self, Parameters(p): Parameters<GetFoodItemParams>) -> Result<CallToolResult, McpError> {
        let result = food_items::get_food_item(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(item) => serde_json::to_string_pretty(&item),
            None => Ok(format!(r#"{{"error": "Food item not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List food items with optional filtering by preference, sorting, and pagination")]
    fn list_food_items(&self, Parameters(p): Parameters<ListFoodItemsParams>) -> Result<CallToolResult, McpError> {
        let result = food_items::list_food_items(&self.database, p.preference.as_deref(), &p.sort_by, &p.sort_order, p.limit, p.offset)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a food item. Automatically recalculates nutrition for any recipes using this item (unless batch mode is active).")]
    fn update_food_item(&self, Parameters(p): Parameters<UpdateFoodItemParams>) -> Result<CallToolResult, McpError> {
        let data = FoodItemUpdate {
            name: p.name, brand: p.brand, serving_size: p.serving_size, serving_unit: p.serving_unit,
            calories: p.calories, protein: p.protein, carbs: p.carbs, fat: p.fat,
            fiber: p.fiber, sodium: p.sodium, sugar: p.sugar, saturated_fat: p.saturated_fat,
            cholesterol: p.cholesterol, preference: p.preference.map(|s| Preference::from_str(&s)), notes: p.notes,
            base_unit_type: None, grams_per_serving: None, ml_per_serving: None,
        };

        // Check if batch mode is active
        let batch_active = {
            let state = self.batch_state.lock().unwrap();
            state.active
        };

        if batch_active {
            // Batch mode: update without cascade, record the ID
            let result = food_items::update_food_item_no_cascade(&self.database, p.id, data)
                .map_err(|e| McpError::internal_error(e, None))?;

            // Record this food item ID for later cascade
            {
                let mut state = self.batch_state.lock().unwrap();
                state.changed_food_item_ids.insert(p.id);
            }

            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        } else {
            // Normal mode: update with immediate cascade
            let result = food_items::update_food_item(&self.database, p.id, data)
                .map_err(|e| McpError::internal_error(e, None))?;
            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
    }

    #[tool(description = "Delete a food item (only allowed if not used in any recipes)")]
    fn delete_food_item(&self, Parameters(p): Parameters<DeleteFoodItemParams>) -> Result<CallToolResult, McpError> {
        let result = food_items::delete_food_item(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Ok(success) => serde_json::to_string_pretty(&success),
            Err(blocked) => serde_json::to_string_pretty(&blocked),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Batch Update Tools ---

    #[tool(description = "Start batch update mode. While active, update_food_item will skip cascade recalculation. Call finish_batch_update when done to perform one combined cascade for all changed items. Use this when updating many food items to avoid performance issues.")]
    fn start_batch_update(&self) -> Result<CallToolResult, McpError> {
        let mut state = self.batch_state.lock().unwrap();

        if state.active {
            // Already in batch mode - return current state
            let response = StartBatchUpdateResponse {
                success: true,
                message: "Batch mode was already active".to_string(),
                pending_items: state.changed_food_item_ids.len() as i64,
            };
            let json = serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // Start batch mode
        state.active = true;
        state.changed_food_item_ids.clear();

        let response = StartBatchUpdateResponse {
            success: true,
            message: "Batch mode started. update_food_item calls will now defer cascade recalculation.".to_string(),
            pending_items: 0,
        };
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Finish batch update mode and perform combined cascade recalculation for all food items that were updated. This is much more efficient than individual cascades when updating many items.")]
    fn finish_batch_update(&self) -> Result<CallToolResult, McpError> {
        // Get the changed IDs and clear state
        let changed_ids = {
            let mut state = self.batch_state.lock().unwrap();

            if !state.active {
                let response = FinishBatchUpdateResponse {
                    success: false,
                    message: "Batch mode was not active".to_string(),
                    food_items_processed: 0,
                    recipes_recalculated: 0,
                    days_recalculated: 0,
                };
                let json = serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }

            // End batch mode and take the IDs
            state.active = false;
            std::mem::take(&mut state.changed_food_item_ids)
        };

        // Perform the combined cascade
        let result = food_items::batch_cascade_recalculate(&self.database, &changed_ids)
            .map_err(|e| McpError::internal_error(e, None))?;

        let response = FinishBatchUpdateResponse {
            success: true,
            message: "Batch update completed successfully".to_string(),
            food_items_processed: result.food_items_processed,
            recipes_recalculated: result.recipes_recalculated,
            days_recalculated: result.days_recalculated,
        };
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Recipes ---

    #[tool(description = "Create a new recipe (ingredients added separately)")]
    fn create_recipe(&self, Parameters(p): Parameters<CreateRecipeParams>) -> Result<CallToolResult, McpError> {
        let data = RecipeCreate { name: p.name, servings_produced: p.servings_produced, is_favorite: p.is_favorite, notes: p.notes };
        let result = recipes::create_recipe(&self.database, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get full recipe details with ingredients and calculated nutrition")]
    fn get_recipe(&self, Parameters(p): Parameters<GetRecipeParams>) -> Result<CallToolResult, McpError> {
        let result = recipes::get_recipe(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(recipe) => serde_json::to_string_pretty(&recipe),
            None => Ok(format!(r#"{{"error": "Recipe not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List recipes with optional search, favorites filter, sorting, and pagination")]
    fn list_recipes(&self, Parameters(p): Parameters<ListRecipesParams>) -> Result<CallToolResult, McpError> {
        let result = recipes::list_recipes(&self.database, p.query.as_deref(), p.favorites_only, &p.sort_by, &p.sort_order, p.limit, p.offset)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update recipe metadata (only allowed if not used in meal entries)")]
    fn update_recipe(&self, Parameters(p): Parameters<UpdateRecipeParams>) -> Result<CallToolResult, McpError> {
        let data = RecipeUpdate { name: p.name, servings_produced: p.servings_produced, is_favorite: p.is_favorite, notes: p.notes };
        let result = recipes::update_recipe(&self.database, p.id, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Ok(success) => serde_json::to_string_pretty(&success),
            Err(blocked) => serde_json::to_string_pretty(&blocked),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a recipe (only allowed if not logged in meals and not used as a component in other recipes)")]
    fn delete_recipe(&self, Parameters(p): Parameters<DeleteRecipeParams>) -> Result<CallToolResult, McpError> {
        let result = recipes::delete_recipe(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Ok(success) => serde_json::to_string_pretty(&success),
            Err(blocked) => serde_json::to_string_pretty(&blocked),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Recipe Ingredients ---

    #[tool(description = "Add a food item to a recipe as an ingredient")]
    fn add_recipe_ingredient(&self, Parameters(p): Parameters<AddRecipeIngredientParams>) -> Result<CallToolResult, McpError> {
        let data = RecipeIngredientCreate { recipe_id: p.recipe_id, food_item_id: p.food_item_id, quantity: p.quantity, unit: p.unit, notes: p.notes };
        let result = recipes::add_recipe_ingredient(&self.database, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Add multiple ingredients to a recipe in one call. PREFERRED over add_recipe_ingredient for efficiency - reduces tool calls from N to 1 and only recalculates nutrition once.")]
    fn add_recipe_ingredients_batch(&self, Parameters(p): Parameters<AddRecipeIngredientsBatchParams>) -> Result<CallToolResult, McpError> {
        use crate::tools::recipes::BatchIngredient;
        let ingredients: Vec<BatchIngredient> = p.ingredients.into_iter().map(|i| BatchIngredient {
            food_item_id: i.food_item_id,
            quantity: i.quantity,
            unit: i.unit,
            notes: i.notes,
        }).collect();
        let result = recipes::add_recipe_ingredients_batch(&self.database, p.recipe_id, ingredients)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a recipe ingredient's quantity or unit")]
    fn update_recipe_ingredient(&self, Parameters(p): Parameters<UpdateRecipeIngredientParams>) -> Result<CallToolResult, McpError> {
        let data = RecipeIngredientUpdate { quantity: p.quantity, unit: p.unit, notes: p.notes };
        let result = recipes::update_recipe_ingredient(&self.database, p.id, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(ing) => serde_json::to_string_pretty(&ing),
            None => Ok(format!(r#"{{"error": "Recipe ingredient not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Remove an ingredient from a recipe")]
    fn remove_recipe_ingredient(&self, Parameters(p): Parameters<RemoveRecipeIngredientParams>) -> Result<CallToolResult, McpError> {
        let deleted = recipes::remove_recipe_ingredient(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::json!({"success": deleted, "id": p.id}).to_string();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Force recalculate cached nutrition values for a recipe")]
    fn recalculate_recipe_nutrition(&self, Parameters(p): Parameters<RecalculateNutritionParams>) -> Result<CallToolResult, McpError> {
        let result = recipes::recalculate_nutrition(&self.database, p.recipe_id).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Recipe Components ---

    #[tool(description = "Add another recipe as a component of a recipe (recipe within a recipe). Automatically calculates combined nutrition.")]
    fn add_recipe_component(&self, Parameters(p): Parameters<AddRecipeComponentParams>) -> Result<CallToolResult, McpError> {
        let data = RecipeComponentCreate { recipe_id: p.recipe_id, component_recipe_id: p.component_recipe_id, servings: p.servings, notes: p.notes };
        let result = recipes::add_recipe_component(&self.database, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a recipe component's servings")]
    fn update_recipe_component(&self, Parameters(p): Parameters<UpdateRecipeComponentParams>) -> Result<CallToolResult, McpError> {
        let data = RecipeComponentUpdate { servings: p.servings, notes: p.notes };
        let result = recipes::update_recipe_component(&self.database, p.id, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(comp) => serde_json::to_string_pretty(&comp),
            None => Ok(format!(r#"{{"error": "Recipe component not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Remove a component recipe from a recipe")]
    fn remove_recipe_component(&self, Parameters(p): Parameters<RemoveRecipeComponentParams>) -> Result<CallToolResult, McpError> {
        let deleted = recipes::remove_recipe_component(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::json!({"success": deleted, "id": p.id}).to_string();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Days ---

    #[tool(description = "Get or create a day by date. Creates a new day if it doesn't exist.")]
    fn get_or_create_day(&self, Parameters(p): Parameters<GetOrCreateDayParams>) -> Result<CallToolResult, McpError> {
        let result = days::get_or_create_day(&self.database, &p.date).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get full day details including all meals organized by type and nutrition totals")]
    fn get_day(&self, Parameters(p): Parameters<GetDayParams>) -> Result<CallToolResult, McpError> {
        let result = days::get_day(&self.database, &p.date).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(day) => serde_json::to_string_pretty(&day),
            None => Ok(format!(r#"{{"error": "Day not found", "date": "{}"}}"#, p.date)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List days with optional date range filter and pagination")]
    fn list_days(&self, Parameters(p): Parameters<ListDaysParams>) -> Result<CallToolResult, McpError> {
        let result = days::list_days(&self.database, p.start_date.as_deref(), p.end_date.as_deref(), p.limit, p.offset)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update day notes")]
    fn update_day(&self, Parameters(p): Parameters<UpdateDayParams>) -> Result<CallToolResult, McpError> {
        let result = days::update_day(&self.database, &p.date, p.notes).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(day) => serde_json::to_string_pretty(&day),
            None => Ok(format!(r#"{{"error": "Day not found", "date": "{}"}}"#, p.date)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Meal Entries ---

    #[tool(description = "Log a meal entry. Provide either recipe_id OR food_item_id (not both). Automatically creates the day if needed.")]
    fn log_meal(&self, Parameters(p): Parameters<LogMealParams>) -> Result<CallToolResult, McpError> {
        let result = days::log_meal(&self.database, &p.date, &p.meal_type, p.recipe_id, p.food_item_id, p.servings, p.percent_eaten, p.notes)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get a meal entry by ID with full details")]
    fn get_meal_entry(&self, Parameters(p): Parameters<GetMealEntryParams>) -> Result<CallToolResult, McpError> {
        let result = days::get_meal_entry(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(entry) => serde_json::to_string_pretty(&entry),
            None => Ok(format!(r#"{{"error": "Meal entry not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a meal entry (servings, percent eaten, meal type, or notes)")]
    fn update_meal_entry(&self, Parameters(p): Parameters<UpdateMealEntryParams>) -> Result<CallToolResult, McpError> {
        let result = days::update_meal_entry(&self.database, p.id, p.meal_type.as_deref(), p.servings, p.percent_eaten, p.notes)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(entry) => serde_json::to_string_pretty(&entry),
            None => Ok(format!(r#"{{"error": "Meal entry not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a meal entry")]
    fn delete_meal_entry(&self, Parameters(p): Parameters<DeleteMealEntryParams>) -> Result<CallToolResult, McpError> {
        let deleted = days::delete_meal_entry(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::json!({"success": deleted, "id": p.id}).to_string();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Force recalculate cached nutrition totals for a day")]
    fn recalculate_day_nutrition(&self, Parameters(p): Parameters<RecalculateDayNutritionParams>) -> Result<CallToolResult, McpError> {
        let result = days::recalculate_day_nutrition_tool(&self.database, &p.date).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Medications ---

    #[tool(description = "Add a new medication (prescription, supplement, OTC, natural remedy, etc.)")]
    fn add_medication(&self, Parameters(p): Parameters<AddMedicationParams>) -> Result<CallToolResult, McpError> {
        let data = MedicationCreate {
            name: p.name,
            med_type: MedType::from_str(&p.med_type),
            dosage_amount: p.dosage_amount,
            dosage_unit: DosageUnit::from_str(&p.dosage_unit),
            instructions: p.instructions,
            frequency: p.frequency,
            prescribing_doctor: p.prescribing_doctor,
            prescribed_date: p.prescribed_date,
            pharmacy: p.pharmacy,
            rx_number: p.rx_number,
            refills_remaining: p.refills_remaining,
            start_date: p.start_date,
            notes: p.notes,
        };
        let result = medications::add_medication(&self.database, data).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get full details for a medication")]
    fn get_medication(&self, Parameters(p): Parameters<GetMedicationParams>) -> Result<CallToolResult, McpError> {
        let result = medications::get_medication(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(med) => serde_json::to_string_pretty(&med),
            None => Ok(format!(r#"{{"error": "Medication not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List medications with optional filtering by active status and type")]
    fn list_medications(&self, Parameters(p): Parameters<ListMedicationsParams>) -> Result<CallToolResult, McpError> {
        let result = medications::list_medications(&self.database, p.active_only, p.med_type.as_deref())
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Search medications by name")]
    fn search_medications(&self, Parameters(p): Parameters<SearchMedicationsParams>) -> Result<CallToolResult, McpError> {
        let result = medications::search_medications(&self.database, &p.query, p.active_only)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a medication. Requires force=true. For dosage changes, consider deprecating and adding new instead.")]
    fn update_medication(&self, Parameters(p): Parameters<UpdateMedicationParams>) -> Result<CallToolResult, McpError> {
        let data = MedicationUpdate {
            name: p.name,
            med_type: p.med_type.map(|s| MedType::from_str(&s)),
            dosage_amount: p.dosage_amount,
            dosage_unit: p.dosage_unit.map(|s| DosageUnit::from_str(&s)),
            instructions: p.instructions,
            frequency: p.frequency,
            prescribing_doctor: p.prescribing_doctor,
            prescribed_date: p.prescribed_date,
            pharmacy: p.pharmacy,
            rx_number: p.rx_number,
            refills_remaining: p.refills_remaining,
            start_date: p.start_date,
            notes: p.notes,
        };
        let result = medications::update_medication(&self.database, p.id, data, p.force)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Ok(success) => serde_json::to_string_pretty(&success),
            Err(blocked) => serde_json::to_string_pretty(&blocked),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Deprecate a medication (mark as inactive). Preferred over deletion to preserve history.")]
    fn deprecate_medication(&self, Parameters(p): Parameters<DeprecateMedicationParams>) -> Result<CallToolResult, McpError> {
        let result = medications::deprecate_medication(&self.database, p.id, p.end_date.as_deref(), p.reason.as_deref())
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Reactivate a previously deprecated medication")]
    fn reactivate_medication(&self, Parameters(p): Parameters<ReactivateMedicationParams>) -> Result<CallToolResult, McpError> {
        let result = medications::reactivate_medication(&self.database, p.id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a medication. Requires force=true. Consider deprecating instead to preserve history.")]
    fn delete_medication(&self, Parameters(p): Parameters<DeleteMedicationParams>) -> Result<CallToolResult, McpError> {
        let result = medications::delete_medication(&self.database, p.id, p.force)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Ok(success) => serde_json::to_string_pretty(&success),
            Err(blocked) => serde_json::to_string_pretty(&blocked),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Export active medications to a formatted markdown document")]
    fn export_medications_markdown(&self, Parameters(p): Parameters<ExportMedicationsParams>) -> Result<CallToolResult, McpError> {
        let result = medications::export_medications_markdown(&self.database, &p.patient_name)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Cleanup/Maintenance ---

    #[tool(description = "List all food items with zero uses (not used in any recipe). These are safe to delete with delete_food_item.")]
    fn list_unused_food_items(&self) -> Result<CallToolResult, McpError> {
        let result = food_items::list_unused_food_items(&self.database).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List all recipes with zero uses (not logged in meals, not used as component in other recipes). These are safe to delete with delete_recipe.")]
    fn list_unused_recipes(&self) -> Result<CallToolResult, McpError> {
        let result = recipes::list_unused_recipes(&self.database).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List all days with no meal entries (orphaned days). These are safe to delete with delete_day.")]
    fn list_orphaned_days(&self) -> Result<CallToolResult, McpError> {
        let result = days::list_orphaned_days(&self.database).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a day by date. Only succeeds if the day has no meal entries. Use list_orphaned_days to find days safe to delete.")]
    fn delete_day(&self, Parameters(p): Parameters<DeleteDayParams>) -> Result<CallToolResult, McpError> {
        let result = days::delete_day(&self.database, &p.date).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // --- Vitals ---

    #[tool(description = "Get step-by-step instructions for tracking vitals. Call this when starting a vital tracking session or when unsure how to use the vital tools.")]
    fn vital_instructions(&self) -> Result<CallToolResult, McpError> {
        use crate::tools::status::VITAL_INSTRUCTIONS;
        Ok(CallToolResult::success(vec![Content::text(VITAL_INSTRUCTIONS)]))
    }

    #[tool(description = "Create a vital group to link related readings together (e.g., BP + HR taken at the same time)")]
    fn create_vital_group(&self, Parameters(p): Parameters<CreateVitalGroupParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::create_vital_group(&self.database, p.description.as_deref(), p.timestamp.as_deref(), p.notes.as_deref())
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get a vital group with all its linked vital readings")]
    fn get_vital_group(&self, Parameters(p): Parameters<GetVitalGroupParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::get_vital_group(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(group) => serde_json::to_string_pretty(&group),
            None => Ok(format!(r#"{{"error": "Vital group not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List vital groups with summary of linked vitals")]
    fn list_vital_groups(&self, Parameters(p): Parameters<ListVitalGroupsParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::list_vital_groups(&self.database, p.limit)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a vital group's description or notes")]
    fn update_vital_group(&self, Parameters(p): Parameters<UpdateVitalGroupParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::update_vital_group(&self.database, p.id, p.description.as_deref(), p.notes.as_deref())
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(group) => serde_json::to_string_pretty(&group),
            None => Ok(format!(r#"{{"error": "Vital group not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a vital group (vitals are unlinked but not deleted)")]
    fn delete_vital_group(&self, Parameters(p): Parameters<DeleteVitalGroupParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::delete_vital_group(&self.database, p.id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Add a vital reading (weight, blood_pressure, heart_rate, oxygen_saturation, glucose)")]
    fn add_vital(&self, Parameters(p): Parameters<AddVitalParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::add_vital(
            &self.database,
            &p.vital_type,
            p.value1,
            p.value2,
            p.unit.as_deref(),
            p.timestamp.as_deref(),
            p.group_id,
            p.notes.as_deref(),
        ).map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get a vital reading by ID")]
    fn get_vital(&self, Parameters(p): Parameters<GetVitalParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::get_vital(&self.database, p.id).map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(vital) => serde_json::to_string_pretty(&vital),
            None => Ok(format!(r#"{{"error": "Vital not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List vitals by type (e.g., all weight readings or all blood pressure readings)")]
    fn list_vitals_by_type(&self, Parameters(p): Parameters<ListVitalsByTypeParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::list_vitals_by_type(&self.database, &p.vital_type, p.limit)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List recent vitals across all types")]
    fn list_recent_vitals(&self, Parameters(p): Parameters<ListRecentVitalsParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::list_recent_vitals(&self.database, p.limit)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List vitals within a date range, optionally filtered by type")]
    fn list_vitals_by_date_range(&self, Parameters(p): Parameters<ListVitalsByDateRangeParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::list_vitals_by_date_range(&self.database, &p.start_date, &p.end_date, p.vital_type.as_deref())
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get the latest reading for each vital type")]
    fn get_latest_vitals(&self) -> Result<CallToolResult, McpError> {
        let result = vitals::get_latest_vitals(&self.database)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Update a vital reading's values or notes")]
    fn update_vital(&self, Parameters(p): Parameters<UpdateVitalParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::update_vital(&self.database, p.id, p.value1, p.value2, p.unit.as_deref(), p.notes.as_deref())
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = match result {
            Some(resp) => serde_json::to_string_pretty(&resp),
            None => Ok(format!(r#"{{"error": "Vital not found", "id": {}}}"#, p.id)),
        }.map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Assign a vital to a group (or remove from group by passing null)")]
    fn assign_vital_to_group(&self, Parameters(p): Parameters<AssignVitalToGroupParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::assign_vital_to_group(&self.database, p.vital_id, p.group_id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Delete a vital reading")]
    fn delete_vital(&self, Parameters(p): Parameters<DeleteVitalParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::delete_vital(&self.database, p.id)
            .map_err(|e| McpError::internal_error(e, None))?;
        let json = serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Import blood pressure and heart rate data from an Omron CSV export file. Creates grouped BP/HR vitals for each reading. File format: Date,Time,Systolic,Diastolic,Pulse,...")]
    fn import_omron_bp_csv(&self, Parameters(p): Parameters<ImportOmronBpCsvParams>) -> Result<CallToolResult, McpError> {
        let result = vitals::import_omron_bp_csv(&self.database, &p.file_path)
            .map_err(|e| McpError::internal_error(e, None))?;
        // Only return summary, not all readings (can be huge)
        let summary = serde_json::json!({
            "success": result.success,
            "file_path": result.file_path,
            "total_rows": result.total_rows,
            "imported": result.imported,
            "skipped": result.skipped,
            "errors": result.errors,
            "date_range": result.date_range,
            "message": format!("Imported {} BP/HR readings from {} rows", result.imported, result.total_rows)
        });
        let json = serde_json::to_string_pretty(&summary).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// ============================================================================
// Server Handler
// ============================================================================

#[tool_handler]
impl ServerHandler for UhmService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "uhm".into(),
                version: crate::build_info::VERSION.into(),
                title: Some("Universal Health Manager".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Universal Health Manager (UHM) - Health, nutrition, and vital sign tracking. \
                 IMPORTANT: Call meal_instructions for food logging, medication_instructions for meds, vital_instructions for vitals. \
                 Food: add/search/get/list/update/delete_food_item. \
                 Recipes: create/get/list/update/delete_recipe, add/update/remove_recipe_ingredient, \
                 add/update/remove_recipe_component, recalculate_recipe_nutrition. \
                 Days: get_or_create_day/get_day/list_days/update_day. \
                 Meals: log_meal/get_meal_entry/update_meal_entry/delete_meal_entry, recalculate_day_nutrition. \
                 Medications: add/get/list/search/update/deprecate/reactivate/delete_medication, export_medications_markdown. \
                 For medication dosage changes: deprecate old entry and add new one to preserve history. \
                 update/delete_medication require force=true. \
                 Vitals: add/get/update/delete_vital, list_vitals_by_type, list_recent_vitals, list_vitals_by_date_range, get_latest_vitals. \
                 Vital Groups: create/get/list/update/delete_vital_group, assign_vital_to_group (for linking BP+HR etc). \
                 Cleanup: list_unused_food_items, list_unused_recipes, list_orphaned_days, delete_day."
                    .into(),
            ),
        }
    }
}
