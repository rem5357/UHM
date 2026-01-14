//! Exercise model
//!
//! Represents exercise sessions (e.g., treadmill workouts) with multiple segments
//! and automatic calorie calculation based on current weight.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::db::DbResult;

/// Exercise type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseType {
    Treadmill,
}

impl ExerciseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExerciseType::Treadmill => "treadmill",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "treadmill" | "tm" => Some(ExerciseType::Treadmill),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ExerciseType::Treadmill => "Treadmill",
        }
    }
}

/// Which field was calculated from the other two
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalculatedField {
    Duration,
    Speed,
    Distance,
    None,
}

impl CalculatedField {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalculatedField::Duration => "duration",
            CalculatedField::Speed => "speed",
            CalculatedField::Distance => "distance",
            CalculatedField::None => "none",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "duration" => CalculatedField::Duration,
            "speed" => CalculatedField::Speed,
            "distance" => CalculatedField::Distance,
            _ => CalculatedField::None,
        }
    }
}

/// An exercise session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub id: i64,
    pub day_id: i64,
    pub exercise_type: ExerciseType,
    pub timestamp: String,
    pub cached_duration_minutes: f64,
    pub cached_distance_miles: f64,
    pub cached_calories_burned: f64,
    pub pre_vital_group_id: Option<i64>,
    pub post_vital_group_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new exercise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseCreate {
    pub day_id: i64,
    pub exercise_type: ExerciseType,
    pub timestamp: Option<String>,
    pub pre_vital_group_id: Option<i64>,
    pub post_vital_group_id: Option<i64>,
    pub notes: Option<String>,
}

/// Data for updating an exercise
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExerciseUpdate {
    pub timestamp: Option<String>,
    pub pre_vital_group_id: Option<i64>,
    pub post_vital_group_id: Option<i64>,
    pub notes: Option<String>,
}

impl Exercise {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let exercise_type_str: String = row.get("exercise_type")?;
        let exercise_type = ExerciseType::from_str(&exercise_type_str)
            .unwrap_or(ExerciseType::Treadmill);

        Ok(Self {
            id: row.get("id")?,
            day_id: row.get("day_id")?,
            exercise_type,
            timestamp: row.get("timestamp")?,
            cached_duration_minutes: row.get("cached_duration_minutes")?,
            cached_distance_miles: row.get("cached_distance_miles")?,
            cached_calories_burned: row.get("cached_calories_burned")?,
            pre_vital_group_id: row.get("pre_vital_group_id")?,
            post_vital_group_id: row.get("post_vital_group_id")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Create a new exercise session
    pub fn create(conn: &Connection, data: &ExerciseCreate) -> DbResult<Self> {
        let timestamp = data.timestamp.clone().unwrap_or_else(|| {
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
        });

        conn.execute(
            r#"
            INSERT INTO exercises (day_id, exercise_type, timestamp, pre_vital_group_id, post_vital_group_id, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                data.day_id,
                data.exercise_type.as_str(),
                timestamp,
                data.pre_vital_group_id,
                data.post_vital_group_id,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get an exercise by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM exercises WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(exercise) => Ok(Some(exercise)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List exercises for a day
    pub fn list_for_day(conn: &Connection, day_id: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM exercises WHERE day_id = ?1 ORDER BY timestamp"
        )?;
        let exercises = stmt
            .query_map([day_id], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(exercises)
    }

    /// List exercises by date range
    pub fn list_by_date_range(
        conn: &Connection,
        start_date: &str,
        end_date: &str,
    ) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT e.* FROM exercises e
            JOIN days d ON e.day_id = d.id
            WHERE d.date >= ?1 AND d.date <= ?2
            ORDER BY e.timestamp DESC
            "#
        )?;
        let exercises = stmt
            .query_map(params![start_date, end_date], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(exercises)
    }

    /// List all exercises
    pub fn list(conn: &Connection, limit: Option<i64>) -> DbResult<Vec<Self>> {
        let sql = match limit {
            Some(n) => format!(
                "SELECT * FROM exercises ORDER BY timestamp DESC LIMIT {}",
                n
            ),
            None => "SELECT * FROM exercises ORDER BY timestamp DESC".to_string(),
        };

        let mut stmt = conn.prepare(&sql)?;
        let exercises = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(exercises)
    }

    /// Update an exercise
    pub fn update(conn: &Connection, id: i64, data: &ExerciseUpdate) -> DbResult<Option<Self>> {
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref ts) = data.timestamp {
            updates.push(format!("timestamp = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(ts.clone()));
        }
        if let Some(pre_id) = data.pre_vital_group_id {
            updates.push(format!("pre_vital_group_id = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(pre_id));
        }
        if let Some(post_id) = data.post_vital_group_id {
            updates.push(format!("post_vital_group_id = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(post_id));
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
            "UPDATE exercises SET {} WHERE id = ?{}",
            updates.join(", "),
            params_vec.len() + 1
        );

        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;

        Self::get_by_id(conn, id)
    }

    /// Delete an exercise and its segments
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Segments are deleted via CASCADE
        let rows = conn.execute("DELETE FROM exercises WHERE id = ?1", [id])?;

        // Recalculate day's calories burned
        if let Some(exercise) = Self::get_by_id(conn, id)? {
            recalculate_day_exercise_calories(conn, exercise.day_id)?;
        }

        Ok(rows > 0)
    }

    /// Recalculate cached totals from segments
    pub fn recalculate_totals(conn: &Connection, id: i64) -> DbResult<Self> {
        let segments = ExerciseSegment::list_for_exercise(conn, id)?;

        let total_duration: f64 = segments.iter()
            .filter_map(|s| s.duration_minutes)
            .sum();
        let total_distance: f64 = segments.iter()
            .filter_map(|s| s.distance_miles)
            .sum();
        let total_calories: f64 = segments.iter()
            .map(|s| s.calories_burned)
            .sum();

        conn.execute(
            r#"
            UPDATE exercises
            SET cached_duration_minutes = ?1,
                cached_distance_miles = ?2,
                cached_calories_burned = ?3,
                updated_at = datetime('now')
            WHERE id = ?4
            "#,
            params![total_duration, total_distance, total_calories, id],
        )?;

        // Also update the day's total calories burned
        if let Some(exercise) = Self::get_by_id(conn, id)? {
            recalculate_day_exercise_calories(conn, exercise.day_id)?;
            return Self::get_by_id(conn, id)?.ok_or_else(|| {
                crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
            });
        }

        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }
}

/// An exercise segment (e.g., 15 min at 2.3 mph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseSegment {
    pub id: i64,
    pub exercise_id: i64,
    pub segment_order: i32,
    pub duration_minutes: Option<f64>,
    pub speed_mph: Option<f64>,
    pub distance_miles: Option<f64>,
    pub incline_percent: f64,
    pub calculated_field: CalculatedField,
    pub is_consistent: bool,
    pub calories_burned: f64,
    pub weight_used_lbs: Option<f64>,
    pub avg_heart_rate: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new exercise segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseSegmentCreate {
    pub exercise_id: i64,
    pub duration_minutes: Option<f64>,
    pub speed_mph: Option<f64>,
    pub distance_miles: Option<f64>,
    pub incline_percent: Option<f64>,
    pub avg_heart_rate: Option<f64>,
    pub notes: Option<String>,
}

/// Data for updating an exercise segment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExerciseSegmentUpdate {
    pub duration_minutes: Option<f64>,
    pub speed_mph: Option<f64>,
    pub distance_miles: Option<f64>,
    pub incline_percent: Option<f64>,
    pub avg_heart_rate: Option<f64>,
    pub notes: Option<String>,
}

impl ExerciseSegment {
    /// Create from a database row
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let calc_field_str: Option<String> = row.get("calculated_field")?;
        let calculated_field = calc_field_str
            .map(|s| CalculatedField::from_str(&s))
            .unwrap_or(CalculatedField::None);

        Ok(Self {
            id: row.get("id")?,
            exercise_id: row.get("exercise_id")?,
            segment_order: row.get("segment_order")?,
            duration_minutes: row.get("duration_minutes")?,
            speed_mph: row.get("speed_mph")?,
            distance_miles: row.get("distance_miles")?,
            incline_percent: row.get("incline_percent")?,
            calculated_field,
            is_consistent: row.get::<_, i32>("is_consistent")? != 0,
            calories_burned: row.get("calories_burned")?,
            weight_used_lbs: row.get("weight_used_lbs")?,
            avg_heart_rate: row.get("avg_heart_rate")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Create a new exercise segment
    pub fn create(conn: &Connection, data: &ExerciseSegmentCreate) -> DbResult<Self> {
        // Get the next segment order
        let next_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(segment_order), 0) + 1 FROM exercise_segments WHERE exercise_id = ?1",
            [data.exercise_id],
            |row| row.get(0),
        )?;

        // Calculate the third value if two are provided
        let (duration, speed, distance, calculated_field, is_consistent) =
            calculate_missing_value(data.duration_minutes, data.speed_mph, data.distance_miles);

        // Get current weight for calorie calculation
        let weight_lbs = get_latest_weight(conn)?;

        // Calculate calories burned
        let calories = calculate_calories_burned(
            duration,
            speed,
            data.incline_percent.unwrap_or(0.0),
            weight_lbs,
        );

        let incline = data.incline_percent.unwrap_or(0.0);

        conn.execute(
            r#"
            INSERT INTO exercise_segments
            (exercise_id, segment_order, duration_minutes, speed_mph, distance_miles,
             incline_percent, calculated_field, is_consistent, calories_burned,
             weight_used_lbs, avg_heart_rate, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                data.exercise_id,
                next_order,
                duration,
                speed,
                distance,
                incline,
                calculated_field.as_str(),
                is_consistent as i32,
                calories,
                weight_lbs,
                data.avg_heart_rate,
                data.notes,
            ],
        )?;

        let id = conn.last_insert_rowid();

        // Recalculate exercise totals
        Exercise::recalculate_totals(conn, data.exercise_id)?;

        Self::get_by_id(conn, id)?.ok_or_else(|| {
            crate::db::DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)
        })
    }

    /// Get an exercise segment by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> DbResult<Option<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM exercise_segments WHERE id = ?1")?;

        let result = stmt.query_row([id], Self::from_row);
        match result {
            Ok(segment) => Ok(Some(segment)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List segments for an exercise
    pub fn list_for_exercise(conn: &Connection, exercise_id: i64) -> DbResult<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM exercise_segments WHERE exercise_id = ?1 ORDER BY segment_order"
        )?;
        let segments = stmt
            .query_map([exercise_id], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(segments)
    }

    /// Update an exercise segment
    pub fn update(conn: &Connection, id: i64, data: &ExerciseSegmentUpdate) -> DbResult<Option<Self>> {
        // Get current segment to merge values
        let current = match Self::get_by_id(conn, id)? {
            Some(s) => s,
            None => return Ok(None),
        };

        let new_duration = data.duration_minutes.or(current.duration_minutes);
        let new_speed = data.speed_mph.or(current.speed_mph);
        let new_distance = data.distance_miles.or(current.distance_miles);
        let new_incline = data.incline_percent.unwrap_or(current.incline_percent);

        // Recalculate values
        let (duration, speed, distance, calculated_field, is_consistent) =
            calculate_missing_value(new_duration, new_speed, new_distance);

        // Get current weight for calorie calculation
        let weight_lbs = get_latest_weight(conn)?;

        // Calculate calories burned
        let calories = calculate_calories_burned(
            duration,
            speed,
            new_incline,
            weight_lbs,
        );

        conn.execute(
            r#"
            UPDATE exercise_segments
            SET duration_minutes = ?1,
                speed_mph = ?2,
                distance_miles = ?3,
                incline_percent = ?4,
                calculated_field = ?5,
                is_consistent = ?6,
                calories_burned = ?7,
                weight_used_lbs = ?8,
                avg_heart_rate = ?9,
                notes = ?10,
                updated_at = datetime('now')
            WHERE id = ?11
            "#,
            params![
                duration,
                speed,
                distance,
                new_incline,
                calculated_field.as_str(),
                is_consistent as i32,
                calories,
                weight_lbs,
                data.avg_heart_rate.or(current.avg_heart_rate),
                data.notes.clone().or(current.notes),
                id,
            ],
        )?;

        // Recalculate exercise totals
        Exercise::recalculate_totals(conn, current.exercise_id)?;

        Self::get_by_id(conn, id)
    }

    /// Delete an exercise segment
    pub fn delete(conn: &Connection, id: i64) -> DbResult<bool> {
        // Get exercise_id before deleting
        let exercise_id = match Self::get_by_id(conn, id)? {
            Some(s) => s.exercise_id,
            None => return Ok(false),
        };

        let rows = conn.execute("DELETE FROM exercise_segments WHERE id = ?1", [id])?;

        // Recalculate exercise totals
        Exercise::recalculate_totals(conn, exercise_id)?;

        Ok(rows > 0)
    }
}

/// Calculate the missing value from the other two (distance = speed × time)
/// Returns (duration, speed, distance, calculated_field, is_consistent)
fn calculate_missing_value(
    duration: Option<f64>,
    speed: Option<f64>,
    distance: Option<f64>,
) -> (Option<f64>, Option<f64>, Option<f64>, CalculatedField, bool) {
    match (duration, speed, distance) {
        // All three provided - check consistency
        (Some(d), Some(s), Some(dist)) => {
            let expected_dist = s * (d / 60.0);
            let tolerance = 0.01; // 1% tolerance
            let is_consistent = (expected_dist - dist).abs() / dist.max(0.001) < tolerance;
            (Some(d), Some(s), Some(dist), CalculatedField::None, is_consistent)
        }
        // Calculate distance from speed and time
        (Some(d), Some(s), None) => {
            let dist = s * (d / 60.0);
            (Some(d), Some(s), Some(dist), CalculatedField::Distance, true)
        }
        // Calculate speed from distance and time
        (Some(d), None, Some(dist)) => {
            let s = if d > 0.0 { dist / (d / 60.0) } else { 0.0 };
            (Some(d), Some(s), Some(dist), CalculatedField::Speed, true)
        }
        // Calculate time from speed and distance
        (None, Some(s), Some(dist)) => {
            let d = if s > 0.0 { (dist / s) * 60.0 } else { 0.0 };
            (Some(d), Some(s), Some(dist), CalculatedField::Duration, true)
        }
        // Only one or none provided - can't calculate
        _ => (duration, speed, distance, CalculatedField::None, true)
    }
}

/// Get the latest weight reading from vitals
fn get_latest_weight(conn: &Connection) -> DbResult<Option<f64>> {
    let result: Result<f64, _> = conn.query_row(
        r#"
        SELECT value1 FROM vitals
        WHERE vital_type = 'weight'
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
        [],
        |row| row.get(0),
    );

    match result {
        Ok(weight) => Ok(Some(weight)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Calculate calories burned for treadmill exercise
/// Uses MET (Metabolic Equivalent of Task) formula:
/// Calories = MET × weight_kg × duration_hours
fn calculate_calories_burned(
    duration_minutes: Option<f64>,
    speed_mph: Option<f64>,
    incline_percent: f64,
    weight_lbs: Option<f64>,
) -> f64 {
    let duration = duration_minutes.unwrap_or(0.0);
    let speed = speed_mph.unwrap_or(0.0);
    let weight_lbs = weight_lbs.unwrap_or(150.0); // Default to 150 lbs if no weight recorded

    if duration <= 0.0 || speed <= 0.0 {
        return 0.0;
    }

    // Convert weight to kg
    let weight_kg = weight_lbs * 0.453592;

    // Base MET values for treadmill (approximations)
    let base_met = if speed < 2.0 {
        2.0 // Very slow walking
    } else if speed < 2.5 {
        2.5
    } else if speed < 3.0 {
        3.0
    } else if speed < 3.5 {
        3.5
    } else if speed < 4.0 {
        4.3
    } else if speed < 4.5 {
        5.0
    } else if speed < 5.0 {
        6.0
    } else if speed < 5.5 {
        8.3
    } else if speed < 6.0 {
        9.0
    } else if speed < 7.0 {
        9.8
    } else if speed < 8.0 {
        10.5
    } else {
        11.5
    };

    // Add incline adjustment (~0.1 MET per 1% grade at walking speeds)
    let incline_adjustment = incline_percent * 0.1;
    let met = base_met + incline_adjustment;

    // Calculate calories: MET × weight_kg × duration_hours
    let duration_hours = duration / 60.0;
    let calories = met * weight_kg * duration_hours;

    // Round to 1 decimal place
    (calories * 10.0).round() / 10.0
}

/// Recalculate total calories burned for a day
pub fn recalculate_day_exercise_calories(conn: &Connection, day_id: i64) -> DbResult<f64> {
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(cached_calories_burned), 0) FROM exercises WHERE day_id = ?1",
        [day_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE days SET cached_calories_burned = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![total, day_id],
    )?;

    Ok(total)
}
