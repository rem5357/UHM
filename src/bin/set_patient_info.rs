//! Utility to set patient info in the database

use std::path::PathBuf;

fn get_database_path() -> PathBuf {
    std::env::var("UHM_DATABASE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));

            // Go up from target/release or target/debug to project root
            if path.ends_with("release") || path.ends_with("debug") {
                if let Some(parent) = path.parent() {
                    if let Some(grandparent) = parent.parent() {
                        path = grandparent.to_path_buf();
                    }
                }
            }

            path.push("data");
            std::fs::create_dir_all(&path).ok();
            path.push("uhm.db");
            path
        })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_database_path();
    println!("Database path: {}", db_path.display());

    let database = uhm::db::Database::new(&db_path)?;

    // Run migrations
    database.with_conn(|conn| {
        uhm::db::migrations::run_migrations(conn)?;
        Ok(())
    })?;

    // Set patient info
    database.with_conn(|conn| {
        let patient = uhm::models::PatientInfo::set(conn, "Robert Myers", "1961-10-22")?;
        println!("Patient info set:");
        println!("  Name: {}", patient.name);
        println!("  DOB: {}", patient.dob);
        println!("  Updated: {}", patient.updated_at);
        Ok(())
    })?;

    Ok(())
}
