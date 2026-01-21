//! Simple utility to recalculate exercise calories
//! Usage: cargo run --bin recalculate_exercises -- [date]

use std::path::PathBuf;

fn get_database_path() -> PathBuf {
    std::env::var("UHM_DATABASE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));

            if path.ends_with("release") || path.ends_with("debug") {
                if let Some(parent) = path.parent() {
                    if let Some(grandparent) = parent.parent() {
                        path = grandparent.to_path_buf();
                    }
                }
            }

            path.push("data");
            path.push("uhm.db");
            path
        })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let date = args.get(1).map(|s| s.as_str()).unwrap_or("2026-01-15");

    let db_path = get_database_path();
    println!("Database: {}", db_path.display());

    let database = uhm::db::Database::new(&db_path)?;

    database.with_conn(|conn| {
        // Get exercises for the date
        let day = uhm::models::Day::get_by_date(conn, date)?;
        let day = match day {
            Some(d) => d,
            None => {
                println!("No data found for date: {}", date);
                return Ok(());
            }
        };

        let exercises = uhm::models::Exercise::list_for_day(conn, day.id)?;
        println!("Found {} exercises for {}", exercises.len(), date);

        for exercise in &exercises {
            println!("\nExercise ID: {}", exercise.id);
            println!("  Old calories: {:.1}", exercise.cached_calories_burned);

            // Get segments before recalculation
            let segments = uhm::models::ExerciseSegment::list_for_exercise(conn, exercise.id)?;
            for seg in &segments {
                println!("  Segment {}: {:.1} min @ {:.1} mph, {:.1}% incline = {:.1} cal",
                    seg.segment_order,
                    seg.duration_minutes.unwrap_or(0.0),
                    seg.speed_mph.unwrap_or(0.0),
                    seg.incline_percent,
                    seg.calories_burned
                );
            }

            // Recalculate
            let updated = uhm::models::ExerciseSegment::recalculate_all_calories_for_exercise(conn, exercise.id)?;

            // Get updated exercise totals
            let updated_exercise = uhm::models::Exercise::get_by_id(conn, exercise.id)?.unwrap();

            println!("  New calories: {:.1}", updated_exercise.cached_calories_burned);
            println!("  Difference: {:.1}", updated_exercise.cached_calories_burned - exercise.cached_calories_burned);

            for seg in &updated {
                println!("  Updated Segment {}: {:.1} cal",
                    seg.segment_order,
                    seg.calories_burned
                );
            }
        }

        Ok(())
    })?;

    Ok(())
}
