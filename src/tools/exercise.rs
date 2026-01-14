//! Exercise MCP Tools
//!
//! Tools for managing exercise sessions and segments (e.g., treadmill workouts).

use std::collections::HashMap;
use serde::Serialize;

use crate::db::Database;
use crate::models::{
    Day, Exercise, ExerciseCreate, ExerciseSegment, ExerciseSegmentCreate,
    ExerciseSegmentUpdate, ExerciseType, ExerciseUpdate, CalculatedField,
    VitalGroup,
};

// ============================================================================
// Response Structs
// ============================================================================

/// Response for add_exercise
#[derive(Debug, Serialize)]
pub struct AddExerciseResponse {
    pub id: i64,
    pub day_id: i64,
    pub date: String,
    pub exercise_type: String,
    pub timestamp: String,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Response for get_exercise with full details
#[derive(Debug, Serialize)]
pub struct ExerciseDetail {
    pub id: i64,
    pub day_id: i64,
    pub date: String,
    pub exercise_type: String,
    pub exercise_type_display: String,
    pub timestamp: String,
    pub total_duration_minutes: f64,
    pub total_distance_miles: f64,
    pub total_calories_burned: f64,
    pub pre_vital_group_id: Option<i64>,
    pub post_vital_group_id: Option<i64>,
    pub notes: Option<String>,
    pub segments: Vec<SegmentDetail>,
    pub created_at: String,
    pub updated_at: String,
}

/// Segment detail for display
#[derive(Debug, Serialize)]
pub struct SegmentDetail {
    pub id: i64,
    pub segment_order: i32,
    pub duration_minutes: Option<f64>,
    pub speed_mph: Option<f64>,
    pub distance_miles: Option<f64>,
    pub incline_percent: f64,
    pub calculated_field: String,
    pub is_consistent: bool,
    pub calories_burned: f64,
    pub weight_used_lbs: Option<f64>,
    pub avg_heart_rate: Option<f64>,
    pub notes: Option<String>,
}

/// Summary for listing
#[derive(Debug, Serialize)]
pub struct ExerciseSummary {
    pub id: i64,
    pub date: String,
    pub exercise_type: String,
    pub timestamp: String,
    pub total_duration_minutes: f64,
    pub total_distance_miles: f64,
    pub total_calories_burned: f64,
    pub segment_count: usize,
    pub notes: Option<String>,
}

/// Response for list_exercises
#[derive(Debug, Serialize)]
pub struct ListExercisesResponse {
    pub exercises: Vec<ExerciseSummary>,
    pub total: usize,
}

/// Response for add_exercise_segment
#[derive(Debug, Serialize)]
pub struct AddSegmentResponse {
    pub id: i64,
    pub exercise_id: i64,
    pub segment_order: i32,
    pub duration_minutes: Option<f64>,
    pub speed_mph: Option<f64>,
    pub distance_miles: Option<f64>,
    pub incline_percent: f64,
    pub calculated_field: String,
    pub is_consistent: bool,
    pub calories_burned: f64,
    pub weight_used_lbs: Option<f64>,
    /// Updated exercise totals after adding segment
    pub exercise_totals: ExerciseTotals,
}

/// Exercise cached totals
#[derive(Debug, Serialize)]
pub struct ExerciseTotals {
    pub total_duration_minutes: f64,
    pub total_distance_miles: f64,
    pub total_calories_burned: f64,
}

/// Response for update operations
#[derive(Debug, Serialize)]
pub struct UpdateResponse {
    pub success: bool,
    pub updated_at: String,
}

/// Response for delete operations
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub deleted_id: i64,
}

// ============================================================================
// Exercise Tool Functions
// ============================================================================

/// Add a new exercise session
pub fn add_exercise(
    db: &Database,
    date: &str,
    exercise_type: &str,
    timestamp: Option<&str>,
    pre_vital_group_id: Option<i64>,
    post_vital_group_id: Option<i64>,
    notes: Option<&str>,
) -> Result<AddExerciseResponse, String> {
    let et = ExerciseType::from_str(exercise_type)
        .ok_or_else(|| format!("Invalid exercise type: '{}'. Valid types: treadmill (tm)", exercise_type))?;

    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get or create day
    let day = Day::get_or_create(&conn, date)
        .map_err(|e| format!("Failed to get/create day: {}", e))?;

    // Validate vital groups if provided
    if let Some(pre_id) = pre_vital_group_id {
        let group = VitalGroup::get_by_id(&conn, pre_id)
            .map_err(|e| format!("Database error: {}", e))?;
        if group.is_none() {
            return Err(format!("Pre-exercise vital group not found: {}", pre_id));
        }
    }
    if let Some(post_id) = post_vital_group_id {
        let group = VitalGroup::get_by_id(&conn, post_id)
            .map_err(|e| format!("Database error: {}", e))?;
        if group.is_none() {
            return Err(format!("Post-exercise vital group not found: {}", post_id));
        }
    }

    let data = ExerciseCreate {
        day_id: day.id,
        exercise_type: et,
        timestamp: timestamp.map(String::from),
        pre_vital_group_id,
        post_vital_group_id,
        notes: notes.map(String::from),
    };

    let exercise = Exercise::create(&conn, &data)
        .map_err(|e| format!("Failed to create exercise: {}", e))?;

    Ok(AddExerciseResponse {
        id: exercise.id,
        day_id: day.id,
        date: day.date,
        exercise_type: exercise.exercise_type.as_str().to_string(),
        timestamp: exercise.timestamp,
        notes: exercise.notes,
        created_at: exercise.created_at,
    })
}

/// Get an exercise by ID with full details
pub fn get_exercise(db: &Database, id: i64) -> Result<Option<ExerciseDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let exercise = Exercise::get_by_id(&conn, id)
        .map_err(|e| format!("Failed to get exercise: {}", e))?;

    match exercise {
        Some(e) => {
            let day = Day::get_by_id(&conn, e.day_id)
                .map_err(|e| format!("Failed to get day: {}", e))?
                .ok_or_else(|| "Day not found".to_string())?;

            let segments = ExerciseSegment::list_for_exercise(&conn, id)
                .map_err(|e| format!("Failed to list segments: {}", e))?;

            let segment_details: Vec<SegmentDetail> = segments.iter().map(|s| SegmentDetail {
                id: s.id,
                segment_order: s.segment_order,
                duration_minutes: s.duration_minutes,
                speed_mph: s.speed_mph,
                distance_miles: s.distance_miles,
                incline_percent: s.incline_percent,
                calculated_field: s.calculated_field.as_str().to_string(),
                is_consistent: s.is_consistent,
                calories_burned: s.calories_burned,
                weight_used_lbs: s.weight_used_lbs,
                avg_heart_rate: s.avg_heart_rate,
                notes: s.notes.clone(),
            }).collect();

            Ok(Some(ExerciseDetail {
                id: e.id,
                day_id: e.day_id,
                date: day.date,
                exercise_type: e.exercise_type.as_str().to_string(),
                exercise_type_display: e.exercise_type.display_name().to_string(),
                timestamp: e.timestamp,
                total_duration_minutes: e.cached_duration_minutes,
                total_distance_miles: e.cached_distance_miles,
                total_calories_burned: e.cached_calories_burned,
                pre_vital_group_id: e.pre_vital_group_id,
                post_vital_group_id: e.post_vital_group_id,
                notes: e.notes,
                segments: segment_details,
                created_at: e.created_at,
                updated_at: e.updated_at,
            }))
        }
        None => Ok(None),
    }
}

/// List exercises with optional date range filter
pub fn list_exercises(
    db: &Database,
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: Option<i64>,
) -> Result<ListExercisesResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let exercises = if let (Some(start), Some(end)) = (start_date, end_date) {
        Exercise::list_by_date_range(&conn, start, end)
            .map_err(|e| format!("Failed to list exercises: {}", e))?
    } else {
        Exercise::list(&conn, limit)
            .map_err(|e| format!("Failed to list exercises: {}", e))?
    };

    let mut summaries = Vec::new();
    for e in &exercises {
        let day = Day::get_by_id(&conn, e.day_id)
            .map_err(|e| format!("Failed to get day: {}", e))?;
        let date = day.map(|d| d.date).unwrap_or_else(|| "unknown".to_string());

        let segments = ExerciseSegment::list_for_exercise(&conn, e.id)
            .map_err(|e| format!("Failed to list segments: {}", e))?;

        summaries.push(ExerciseSummary {
            id: e.id,
            date,
            exercise_type: e.exercise_type.as_str().to_string(),
            timestamp: e.timestamp.clone(),
            total_duration_minutes: e.cached_duration_minutes,
            total_distance_miles: e.cached_distance_miles,
            total_calories_burned: e.cached_calories_burned,
            segment_count: segments.len(),
            notes: e.notes.clone(),
        });
    }

    let total = summaries.len();
    Ok(ListExercisesResponse {
        exercises: summaries,
        total,
    })
}

/// List exercises for a specific day
pub fn list_exercises_for_day(db: &Database, date: &str) -> Result<ListExercisesResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let day = Day::get_by_date(&conn, date)
        .map_err(|e| format!("Failed to get day: {}", e))?;

    let day = match day {
        Some(d) => d,
        None => return Ok(ListExercisesResponse { exercises: Vec::new(), total: 0 }),
    };

    let exercises = Exercise::list_for_day(&conn, day.id)
        .map_err(|e| format!("Failed to list exercises: {}", e))?;

    let mut summaries = Vec::new();
    for e in &exercises {
        let segments = ExerciseSegment::list_for_exercise(&conn, e.id)
            .map_err(|e| format!("Failed to list segments: {}", e))?;

        summaries.push(ExerciseSummary {
            id: e.id,
            date: date.to_string(),
            exercise_type: e.exercise_type.as_str().to_string(),
            timestamp: e.timestamp.clone(),
            total_duration_minutes: e.cached_duration_minutes,
            total_distance_miles: e.cached_distance_miles,
            total_calories_burned: e.cached_calories_burned,
            segment_count: segments.len(),
            notes: e.notes.clone(),
        });
    }

    let total = summaries.len();
    Ok(ListExercisesResponse {
        exercises: summaries,
        total,
    })
}

/// Update an exercise
pub fn update_exercise(
    db: &Database,
    id: i64,
    pre_vital_group_id: Option<i64>,
    post_vital_group_id: Option<i64>,
    notes: Option<&str>,
) -> Result<Option<UpdateResponse>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Validate vital groups if provided
    if let Some(pre_id) = pre_vital_group_id {
        let group = VitalGroup::get_by_id(&conn, pre_id)
            .map_err(|e| format!("Database error: {}", e))?;
        if group.is_none() {
            return Err(format!("Pre-exercise vital group not found: {}", pre_id));
        }
    }
    if let Some(post_id) = post_vital_group_id {
        let group = VitalGroup::get_by_id(&conn, post_id)
            .map_err(|e| format!("Database error: {}", e))?;
        if group.is_none() {
            return Err(format!("Post-exercise vital group not found: {}", post_id));
        }
    }

    let data = ExerciseUpdate {
        pre_vital_group_id,
        post_vital_group_id,
        notes: notes.map(String::from),
    };

    let updated = Exercise::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update exercise: {}", e))?;

    Ok(updated.map(|e| UpdateResponse {
        success: true,
        updated_at: e.updated_at,
    }))
}

/// Delete an exercise
pub fn delete_exercise(db: &Database, id: i64) -> Result<DeleteResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if exists
    let existing = Exercise::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Exercise not found with id: {}", id));
    }

    Exercise::delete(&conn, id)
        .map_err(|e| format!("Failed to delete exercise: {}", e))?;

    Ok(DeleteResponse {
        success: true,
        deleted_id: id,
    })
}

// ============================================================================
// Exercise Segment Tool Functions
// ============================================================================

/// Add a segment to an exercise
pub fn add_exercise_segment(
    db: &Database,
    exercise_id: i64,
    duration_minutes: Option<f64>,
    speed_mph: Option<f64>,
    distance_miles: Option<f64>,
    incline_percent: Option<f64>,
    avg_heart_rate: Option<f64>,
    notes: Option<&str>,
) -> Result<AddSegmentResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Verify exercise exists
    let _exercise = Exercise::get_by_id(&conn, exercise_id)
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| format!("Exercise not found with id: {}", exercise_id))?;

    // Need at least 2 of 3 values
    let provided_count = [duration_minutes.is_some(), speed_mph.is_some(), distance_miles.is_some()]
        .iter()
        .filter(|&&x| x)
        .count();

    if provided_count < 2 {
        return Err("Must provide at least 2 of: duration_minutes, speed_mph, distance_miles".to_string());
    }

    let data = ExerciseSegmentCreate {
        exercise_id,
        duration_minutes,
        speed_mph,
        distance_miles,
        incline_percent,
        avg_heart_rate,
        notes: notes.map(String::from),
    };

    let segment = ExerciseSegment::create(&conn, &data)
        .map_err(|e| format!("Failed to create segment: {}", e))?;

    // Get updated exercise totals
    let updated_exercise = Exercise::get_by_id(&conn, exercise_id)
        .map_err(|e| format!("Database error: {}", e))?
        .unwrap();

    Ok(AddSegmentResponse {
        id: segment.id,
        exercise_id: segment.exercise_id,
        segment_order: segment.segment_order,
        duration_minutes: segment.duration_minutes,
        speed_mph: segment.speed_mph,
        distance_miles: segment.distance_miles,
        incline_percent: segment.incline_percent,
        calculated_field: segment.calculated_field.as_str().to_string(),
        is_consistent: segment.is_consistent,
        calories_burned: segment.calories_burned,
        weight_used_lbs: segment.weight_used_lbs,
        exercise_totals: ExerciseTotals {
            total_duration_minutes: updated_exercise.cached_duration_minutes,
            total_distance_miles: updated_exercise.cached_distance_miles,
            total_calories_burned: updated_exercise.cached_calories_burned,
        },
    })
}

/// Update an exercise segment
pub fn update_exercise_segment(
    db: &Database,
    id: i64,
    duration_minutes: Option<f64>,
    speed_mph: Option<f64>,
    distance_miles: Option<f64>,
    incline_percent: Option<f64>,
    avg_heart_rate: Option<f64>,
    notes: Option<&str>,
) -> Result<Option<SegmentDetail>, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    let data = ExerciseSegmentUpdate {
        duration_minutes,
        speed_mph,
        distance_miles,
        incline_percent,
        avg_heart_rate,
        notes: notes.map(String::from),
    };

    let updated = ExerciseSegment::update(&conn, id, &data)
        .map_err(|e| format!("Failed to update segment: {}", e))?;

    Ok(updated.map(|s| SegmentDetail {
        id: s.id,
        segment_order: s.segment_order,
        duration_minutes: s.duration_minutes,
        speed_mph: s.speed_mph,
        distance_miles: s.distance_miles,
        incline_percent: s.incline_percent,
        calculated_field: s.calculated_field.as_str().to_string(),
        is_consistent: s.is_consistent,
        calories_burned: s.calories_burned,
        weight_used_lbs: s.weight_used_lbs,
        avg_heart_rate: s.avg_heart_rate,
        notes: s.notes,
    }))
}

/// Delete an exercise segment
pub fn delete_exercise_segment(db: &Database, id: i64) -> Result<DeleteResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Check if exists
    let existing = ExerciseSegment::get_by_id(&conn, id)
        .map_err(|e| format!("Database error: {}", e))?;

    if existing.is_none() {
        return Err(format!("Exercise segment not found with id: {}", id));
    }

    ExerciseSegment::delete(&conn, id)
        .map_err(|e| format!("Failed to delete segment: {}", e))?;

    Ok(DeleteResponse {
        success: true,
        deleted_id: id,
    })
}

// ============================================================================
// Exercise Statistics
// ============================================================================

/// An outlier reading
#[derive(Debug, Serialize)]
pub struct ExerciseOutlier {
    pub date: String,
    pub exercise_id: i64,
    pub value: f64,
    pub z_score: f64,
}

/// Statistics for a single value
#[derive(Debug, Serialize)]
pub struct ExerciseValueStats {
    pub count: i64,
    pub sum: f64,
    pub average: f64,
    pub median: f64,
    pub mode: Option<f64>,
    pub standard_deviation: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub range: f64,
    pub percentile_25: f64,
    pub percentile_75: f64,
    pub iqr: f64,
    pub coefficient_of_variation: f64,
    pub outliers: Vec<ExerciseOutlier>,
}

/// Response for list_exercise_stats
#[derive(Debug, Serialize)]
pub struct ListExerciseStatsResponse {
    pub exercises_analyzed: i64,
    pub date_range: Option<ExerciseDateRange>,
    pub duration: ExerciseValueStats,
    pub distance: ExerciseValueStats,
    pub calories: ExerciseValueStats,
    /// Average speed across all segments
    pub speed: ExerciseValueStats,
    /// Average incline across all segments
    pub incline: ExerciseValueStats,
}

/// Date range for stats
#[derive(Debug, Serialize)]
pub struct ExerciseDateRange {
    pub start: String,
    pub end: String,
}

/// Internal value for stats
struct ExerciseValue {
    date: String,
    exercise_id: i64,
    value: f64,
}

/// Calculate percentile using linear interpolation
fn exercise_percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len();
    let rank = (p / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let weight = rank - lower as f64;

    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

/// Calculate statistics for a list of values
fn calculate_exercise_stats(values: &[ExerciseValue]) -> ExerciseValueStats {
    if values.is_empty() {
        return ExerciseValueStats {
            count: 0,
            sum: 0.0,
            average: 0.0,
            median: 0.0,
            mode: None,
            standard_deviation: 0.0,
            variance: 0.0,
            min: 0.0,
            max: 0.0,
            range: 0.0,
            percentile_25: 0.0,
            percentile_75: 0.0,
            iqr: 0.0,
            coefficient_of_variation: 0.0,
            outliers: Vec::new(),
        };
    }

    let count = values.len() as i64;
    let mut sorted_values: Vec<f64> = values.iter().map(|v| v.value).collect();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let sum: f64 = sorted_values.iter().sum();
    let average = sum / count as f64;

    let min = sorted_values[0];
    let max = sorted_values[sorted_values.len() - 1];
    let range = max - min;

    let median = if count % 2 == 0 {
        let mid = count as usize / 2;
        (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
    } else {
        sorted_values[count as usize / 2]
    };

    let mode = {
        let mut freq: HashMap<i64, i64> = HashMap::new();
        for v in &sorted_values {
            let key = (v * 10.0).round() as i64;
            *freq.entry(key).or_insert(0) += 1;
        }
        let max_freq = freq.values().max().copied().unwrap_or(0);
        if max_freq > 1 {
            freq.iter()
                .find(|(_, &f)| f == max_freq)
                .map(|(&k, _)| k as f64 / 10.0)
        } else {
            None
        }
    };

    let variance = if count > 1 {
        let sum_sq_diff: f64 = sorted_values.iter().map(|v| (v - average).powi(2)).sum();
        sum_sq_diff / (count - 1) as f64
    } else {
        0.0
    };
    let standard_deviation = variance.sqrt();

    let percentile_25 = exercise_percentile(&sorted_values, 25.0);
    let percentile_75 = exercise_percentile(&sorted_values, 75.0);
    let iqr = percentile_75 - percentile_25;

    let coefficient_of_variation = if average != 0.0 {
        (standard_deviation / average) * 100.0
    } else {
        0.0
    };

    let outliers: Vec<ExerciseOutlier> = if standard_deviation > 0.0 {
        values
            .iter()
            .filter_map(|ev| {
                let z_score = (ev.value - average) / standard_deviation;
                if z_score.abs() > 1.0 {
                    Some(ExerciseOutlier {
                        date: ev.date.clone(),
                        exercise_id: ev.exercise_id,
                        value: ev.value,
                        z_score: (z_score * 100.0).round() / 100.0,
                    })
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    ExerciseValueStats {
        count,
        sum: (sum * 100.0).round() / 100.0,
        average: (average * 100.0).round() / 100.0,
        median: (median * 100.0).round() / 100.0,
        mode: mode.map(|m| (m * 100.0).round() / 100.0),
        standard_deviation: (standard_deviation * 100.0).round() / 100.0,
        variance: (variance * 100.0).round() / 100.0,
        min: (min * 100.0).round() / 100.0,
        max: (max * 100.0).round() / 100.0,
        range: (range * 100.0).round() / 100.0,
        percentile_25: (percentile_25 * 100.0).round() / 100.0,
        percentile_75: (percentile_75 * 100.0).round() / 100.0,
        iqr: (iqr * 100.0).round() / 100.0,
        coefficient_of_variation: (coefficient_of_variation * 100.0).round() / 100.0,
        outliers,
    }
}

/// Get comprehensive statistics for exercises
pub fn list_exercise_stats(
    db: &Database,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<ListExerciseStatsResponse, String> {
    let conn = db.get_conn().map_err(|e| format!("Database error: {}", e))?;

    // Get all exercises in range
    let exercises = if let (Some(start), Some(end)) = (start_date, end_date) {
        Exercise::list_by_date_range(&conn, start, end)
            .map_err(|e| format!("Failed to list exercises: {}", e))?
    } else {
        Exercise::list(&conn, None)
            .map_err(|e| format!("Failed to list exercises: {}", e))?
    };

    if exercises.is_empty() {
        return Ok(ListExerciseStatsResponse {
            exercises_analyzed: 0,
            date_range: None,
            duration: calculate_exercise_stats(&[]),
            distance: calculate_exercise_stats(&[]),
            calories: calculate_exercise_stats(&[]),
            speed: calculate_exercise_stats(&[]),
            incline: calculate_exercise_stats(&[]),
        });
    }

    // Collect values
    let mut duration_values: Vec<ExerciseValue> = Vec::new();
    let mut distance_values: Vec<ExerciseValue> = Vec::new();
    let mut calories_values: Vec<ExerciseValue> = Vec::new();
    let mut speed_values: Vec<ExerciseValue> = Vec::new();
    let mut incline_values: Vec<ExerciseValue> = Vec::new();
    let mut dates: Vec<String> = Vec::new();

    for exercise in &exercises {
        let day = Day::get_by_id(&conn, exercise.day_id)
            .map_err(|e| format!("Failed to get day: {}", e))?;
        let date = day.map(|d| d.date).unwrap_or_else(|| "unknown".to_string());
        dates.push(date.clone());

        if exercise.cached_duration_minutes > 0.0 {
            duration_values.push(ExerciseValue {
                date: date.clone(),
                exercise_id: exercise.id,
                value: exercise.cached_duration_minutes,
            });
        }
        if exercise.cached_distance_miles > 0.0 {
            distance_values.push(ExerciseValue {
                date: date.clone(),
                exercise_id: exercise.id,
                value: exercise.cached_distance_miles,
            });
        }
        if exercise.cached_calories_burned > 0.0 {
            calories_values.push(ExerciseValue {
                date: date.clone(),
                exercise_id: exercise.id,
                value: exercise.cached_calories_burned,
            });
        }

        // Get segment averages for speed and incline
        let segments = ExerciseSegment::list_for_exercise(&conn, exercise.id)
            .map_err(|e| format!("Failed to list segments: {}", e))?;

        if !segments.is_empty() {
            let avg_speed: f64 = segments.iter()
                .filter_map(|s| s.speed_mph)
                .sum::<f64>() / segments.len() as f64;
            let avg_incline: f64 = segments.iter()
                .map(|s| s.incline_percent)
                .sum::<f64>() / segments.len() as f64;

            if avg_speed > 0.0 {
                speed_values.push(ExerciseValue {
                    date: date.clone(),
                    exercise_id: exercise.id,
                    value: avg_speed,
                });
            }
            incline_values.push(ExerciseValue {
                date: date.clone(),
                exercise_id: exercise.id,
                value: avg_incline,
            });
        }
    }

    dates.sort();
    let date_range = if !dates.is_empty() {
        Some(ExerciseDateRange {
            start: dates.first().unwrap().clone(),
            end: dates.last().unwrap().clone(),
        })
    } else {
        None
    };

    Ok(ListExerciseStatsResponse {
        exercises_analyzed: exercises.len() as i64,
        date_range,
        duration: calculate_exercise_stats(&duration_values),
        distance: calculate_exercise_stats(&distance_values),
        calories: calculate_exercise_stats(&calories_values),
        speed: calculate_exercise_stats(&speed_values),
        incline: calculate_exercise_stats(&incline_values),
    })
}
