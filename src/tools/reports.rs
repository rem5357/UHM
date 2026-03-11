//! Report generation tools
//!
//! Generate PDF reports for blood pressure and heart rate data with charts and statistics.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use chrono::{Datelike, NaiveDate, Weekday};
use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};
use printpdf::image_crate::{DynamicImage, RgbImage, ImageFormat};
use serde::Serialize;

use crate::db::Database;
use crate::models::{Day, Exercise, MedType, Medication, PatientInfo, Vital, VitalType};

// ============================================================================
// Color Constants (RGB 0-255)
// ============================================================================

const COLOR_BP_TITLE: (u8, u8, u8) = (192, 0, 0);       // Red for BP title
const COLOR_HR_TITLE: (u8, u8, u8) = (112, 48, 160);    // Purple for HR title
const COLOR_NORMAL: (u8, u8, u8) = (0, 176, 80);        // Green
const COLOR_ELEVATED: (u8, u8, u8) = (255, 165, 0);     // Orange
const COLOR_HIGH: (u8, u8, u8) = (255, 0, 0);           // Red
const COLOR_BRADYCARDIA: (u8, u8, u8) = (0, 112, 192);  // Blue
const COLOR_MED_TITLE: (u8, u8, u8) = (0, 112, 192);   // Blue for medication headers
const COLOR_BLACK: (u8, u8, u8) = (0, 0, 0);
const COLOR_GRAY: (u8, u8, u8) = (128, 128, 128);
const COLOR_LIGHT_GRAY: (u8, u8, u8) = (220, 220, 220);

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct GenerateReportResponse {
    pub success: bool,
    pub file_path: String,
    pub total_readings: i64,
    pub days_analyzed: i64,
    pub date_range: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateMedicationsReportResponse {
    pub success: bool,
    pub file_path: String,
    pub medication_count: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GeneratePillOrganizerReportResponse {
    pub success: bool,
    pub file_path: String,
    pub medication_count: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateExerciseReportResponse {
    pub file_path: String,
    pub sessions: i64,
    pub days_with_exercise: i64,
    pub total_duration_minutes: f64,
    pub total_distance_miles: f64,
    pub total_calories_burned: f64,
}

// ============================================================================
// Daily Statistics Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DailyBPStats {
    pub date: String,
    pub day_of_week: String,
    pub count: i64,
    pub systolic_avg: f64,
    pub systolic_sd: f64,
    pub systolic_min: f64,
    pub systolic_max: f64,
    pub diastolic_avg: f64,
    pub diastolic_sd: f64,
    pub diastolic_min: f64,
    pub diastolic_max: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyHRStats {
    pub date: String,
    pub day_of_week: String,
    pub count: i64,
    pub hr_avg: f64,
    pub hr_sd: f64,
    pub hr_min: f64,
    pub hr_max: f64,
}

// ============================================================================
// Classification Functions
// ============================================================================

/// Classify blood pressure based on systolic and diastolic values
pub fn classify_bp(systolic: f64, diastolic: f64) -> (&'static str, (u8, u8, u8)) {
    if systolic >= 140.0 || diastolic >= 90.0 {
        ("Stage 2 HTN", COLOR_HIGH)
    } else if systolic >= 130.0 || diastolic >= 80.0 {
        ("Stage 1 HTN", COLOR_HIGH)
    } else if systolic >= 120.0 {
        ("Elevated", COLOR_ELEVATED)
    } else {
        ("Normal", COLOR_NORMAL)
    }
}

/// Classify heart rate
pub fn classify_hr(bpm: f64) -> (&'static str, (u8, u8, u8)) {
    if bpm < 50.0 {
        ("Bradycardia", COLOR_BRADYCARDIA)
    } else if bpm < 60.0 {
        ("Low Normal", COLOR_NORMAL)
    } else if bpm <= 100.0 {
        ("Normal", COLOR_NORMAL)
    } else {
        ("Elevated", COLOR_ELEVATED)
    }
}

// ============================================================================
// Statistics Aggregation
// ============================================================================

fn day_of_week_abbrev(date: &NaiveDate) -> &'static str {
    match date.weekday() {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn calculate_std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance = values.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

/// Aggregate daily BP statistics from vitals
pub fn aggregate_daily_bp_stats(vitals: &[Vital]) -> Vec<DailyBPStats> {
    // Group by date
    let mut by_date: BTreeMap<String, Vec<&Vital>> = BTreeMap::new();

    for vital in vitals {
        if vital.vital_type != VitalType::BloodPressure {
            continue;
        }
        // Extract date portion from timestamp
        let date = vital.timestamp.split('T').next().unwrap_or(&vital.timestamp);
        by_date.entry(date.to_string()).or_default().push(vital);
    }

    let mut result = Vec::new();

    for (date, readings) in by_date {
        let systolic_values: Vec<f64> = readings.iter().map(|v| v.value1).collect();
        let diastolic_values: Vec<f64> = readings.iter()
            .filter_map(|v| v.value2)
            .collect();

        if systolic_values.is_empty() {
            continue;
        }

        let systolic_avg = systolic_values.iter().sum::<f64>() / systolic_values.len() as f64;
        let diastolic_avg = if diastolic_values.is_empty() {
            0.0
        } else {
            diastolic_values.iter().sum::<f64>() / diastolic_values.len() as f64
        };

        let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
        let day_of_week = parsed_date.map(|d| day_of_week_abbrev(&d)).unwrap_or("---");

        result.push(DailyBPStats {
            date: date.clone(),
            day_of_week: day_of_week.to_string(),
            count: readings.len() as i64,
            systolic_avg,
            systolic_sd: calculate_std_dev(&systolic_values, systolic_avg),
            systolic_min: systolic_values.iter().cloned().fold(f64::INFINITY, f64::min),
            systolic_max: systolic_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            diastolic_avg,
            diastolic_sd: calculate_std_dev(&diastolic_values, diastolic_avg),
            diastolic_min: diastolic_values.iter().cloned().fold(f64::INFINITY, f64::min),
            diastolic_max: diastolic_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        });
    }

    result
}

/// Aggregate daily HR statistics from vitals
pub fn aggregate_daily_hr_stats(vitals: &[Vital]) -> Vec<DailyHRStats> {
    // Group by date
    let mut by_date: BTreeMap<String, Vec<&Vital>> = BTreeMap::new();

    for vital in vitals {
        if vital.vital_type != VitalType::HeartRate {
            continue;
        }
        let date = vital.timestamp.split('T').next().unwrap_or(&vital.timestamp);
        by_date.entry(date.to_string()).or_default().push(vital);
    }

    let mut result = Vec::new();

    for (date, readings) in by_date {
        let hr_values: Vec<f64> = readings.iter().map(|v| v.value1).collect();

        if hr_values.is_empty() {
            continue;
        }

        let hr_avg = hr_values.iter().sum::<f64>() / hr_values.len() as f64;

        let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
        let day_of_week = parsed_date.map(|d| day_of_week_abbrev(&d)).unwrap_or("---");

        result.push(DailyHRStats {
            date: date.clone(),
            day_of_week: day_of_week.to_string(),
            count: readings.len() as i64,
            hr_avg,
            hr_sd: calculate_std_dev(&hr_values, hr_avg),
            hr_min: hr_values.iter().cloned().fold(f64::INFINITY, f64::min),
            hr_max: hr_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        });
    }

    result
}

// ============================================================================
// Chart Generation (plotters)
// ============================================================================

/// Generate BP trend chart as PNG bytes
pub fn generate_bp_chart(daily_stats: &[DailyBPStats], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use plotters::prelude::*;

    if daily_stats.is_empty() {
        return Err("No data to chart".to_string());
    }

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        // Calculate Y axis range
        let y_min = daily_stats.iter()
            .flat_map(|s| vec![s.diastolic_min, s.systolic_min])
            .fold(f64::INFINITY, f64::min)
            .max(40.0) - 10.0;
        let y_max = daily_stats.iter()
            .flat_map(|s| vec![s.diastolic_max, s.systolic_max])
            .fold(f64::NEG_INFINITY, f64::max)
            .min(200.0) + 10.0;

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(
                0..(daily_stats.len() as i32),
                y_min..y_max
            )
            .map_err(|e| e.to_string())?;

        // Pre-compute date labels to avoid closure issues with large datasets
        let date_labels: Vec<String> = daily_stats.iter()
            .map(|s| s.date.split('-').skip(1).collect::<Vec<_>>().join("/"))
            .collect();
        let labels_len = date_labels.len();

        chart.configure_mesh()
            .x_labels(daily_stats.len().min(10))
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < labels_len {
                    date_labels[idx].clone()
                } else {
                    String::new()
                }
            })
            .y_desc("mmHg")
            .draw()
            .map_err(|e| e.to_string())?;

        // Reference lines
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(0, 140.0), (daily_stats.len() as i32, 140.0)],
            ShapeStyle::from(&RGBColor(255, 0, 0).mix(0.5)).stroke_width(1),
        ))).map_err(|e| e.to_string())?;

        chart.draw_series(std::iter::once(PathElement::new(
            vec![(0, 130.0), (daily_stats.len() as i32, 130.0)],
            ShapeStyle::from(&RGBColor(255, 165, 0).mix(0.5)).stroke_width(1),
        ))).map_err(|e| e.to_string())?;

        // Systolic min-max band (red cloud)
        // Create polygon: go along max values, then back along min values
        let mut systolic_polygon: Vec<(i32, f64)> = Vec::new();
        for (i, s) in daily_stats.iter().enumerate() {
            systolic_polygon.push((i as i32, s.systolic_max));
        }
        for (i, s) in daily_stats.iter().enumerate().rev() {
            systolic_polygon.push((i as i32, s.systolic_min));
        }
        if !systolic_polygon.is_empty() {
            systolic_polygon.push(systolic_polygon[0]); // Close the polygon
        }
        chart.draw_series(std::iter::once(Polygon::new(
            systolic_polygon,
            RGBColor(255, 0, 0).mix(0.15),
        ))).map_err(|e| e.to_string())?;

        // Diastolic min-max band (blue cloud)
        let mut diastolic_polygon: Vec<(i32, f64)> = Vec::new();
        for (i, s) in daily_stats.iter().enumerate() {
            diastolic_polygon.push((i as i32, s.diastolic_max));
        }
        for (i, s) in daily_stats.iter().enumerate().rev() {
            diastolic_polygon.push((i as i32, s.diastolic_min));
        }
        if !diastolic_polygon.is_empty() {
            diastolic_polygon.push(diastolic_polygon[0]);
        }
        chart.draw_series(std::iter::once(Polygon::new(
            diastolic_polygon,
            RGBColor(0, 0, 255).mix(0.15),
        ))).map_err(|e| e.to_string())?;

        // Systolic average line
        let systolic_points: Vec<(i32, f64)> = daily_stats.iter()
            .enumerate()
            .map(|(i, s)| (i as i32, s.systolic_avg))
            .collect();

        chart.draw_series(LineSeries::new(
            systolic_points.clone(),
            RED.stroke_width(2),
        ))
        .map_err(|e| e.to_string())?
        .label("Systolic (avg)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(2)));

        // Only show data point markers for reports of 31 days or less
        if daily_stats.len() <= 31 {
            chart.draw_series(systolic_points.iter().map(|(x, y)| {
                Circle::new((*x, *y), 3, RED.filled())
            })).map_err(|e| e.to_string())?;
        }

        // Diastolic average line
        let diastolic_points: Vec<(i32, f64)> = daily_stats.iter()
            .enumerate()
            .map(|(i, s)| (i as i32, s.diastolic_avg))
            .collect();

        chart.draw_series(LineSeries::new(
            diastolic_points.clone(),
            BLUE.stroke_width(2),
        ))
        .map_err(|e| e.to_string())?
        .label("Diastolic (avg)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(2)));

        // Only show data point markers for reports of 31 days or less
        if daily_stats.len() <= 31 {
            chart.draw_series(diastolic_points.iter().map(|(x, y)| {
                Circle::new((*x, *y), 3, BLUE.filled())
            })).map_err(|e| e.to_string())?;
        }

        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()
            .map_err(|e| e.to_string())?;

        root.present().map_err(|e| e.to_string())?;
    }

    // Convert RGB buffer to PNG
    let img = RgbImage::from_raw(width, height, buffer)
        .ok_or("Failed to create image from buffer")?;

    let mut png_bytes = Vec::new();
    let dyn_img = DynamicImage::ImageRgb8(img);
    dyn_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(png_bytes)
}

// ============================================================================
// BP Time-of-Day Types and Chart Generation
// ============================================================================

/// Time-of-day bucket labels (3-hour windows)
const TIME_BUCKET_LABELS: [&str; 8] = [
    "12a-3a", "3a-6a", "6a-9a", "9a-12p",
    "12p-3p", "3p-6p", "6p-9p", "9p-12a",
];

/// Statistics for a single time-of-day bucket
#[derive(Debug, Clone)]
struct TimeBucketStats {
    label: &'static str,
    systolic_avg: Option<f64>,
    diastolic_avg: Option<f64>,
    count: usize,
}

/// Aggregate BP readings into 8 three-hour time-of-day buckets
fn aggregate_time_of_day_bp(vitals: &[Vital]) -> Vec<TimeBucketStats> {
    let mut buckets: Vec<(Vec<f64>, Vec<f64>)> = vec![(Vec::new(), Vec::new()); 8];

    for vital in vitals {
        if vital.vital_type != VitalType::BloodPressure {
            continue;
        }
        // Parse hour from timestamp "YYYY-MM-DDTHH:MM:SS"
        let hour = vital.timestamp
            .split('T')
            .nth(1)
            .and_then(|t| t.split(':').next())
            .and_then(|h| h.parse::<usize>().ok());

        if let Some(h) = hour {
            let bucket = h / 3; // 0..7
            if bucket < 8 {
                buckets[bucket].0.push(vital.value1);
                if let Some(dia) = vital.value2 {
                    buckets[bucket].1.push(dia);
                }
            }
        }
    }

    buckets.iter().enumerate().map(|(i, (sys, dia))| {
        TimeBucketStats {
            label: TIME_BUCKET_LABELS[i],
            systolic_avg: if sys.is_empty() { None } else {
                Some(sys.iter().sum::<f64>() / sys.len() as f64)
            },
            diastolic_avg: if dia.is_empty() { None } else {
                Some(dia.iter().sum::<f64>() / dia.len() as f64)
            },
            count: sys.len(),
        }
    }).collect()
}

/// Generate BP time-of-day chart as PNG bytes
fn generate_bp_time_of_day_chart(buckets: &[TimeBucketStats], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use plotters::prelude::*;

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        // Y axis range
        let mut y_min: f64 = 200.0;
        let mut y_max: f64 = 0.0;
        for b in buckets {
            if let Some(s) = b.systolic_avg {
                y_max = y_max.max(s);
            }
            if let Some(d) = b.diastolic_avg {
                y_min = y_min.min(d);
            }
        }
        y_min = (y_min - 15.0).max(40.0);
        y_max = (y_max + 15.0).min(200.0);

        let labels: Vec<String> = buckets.iter().map(|b| b.label.to_string()).collect();
        let labels_len = labels.len();

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(35)
            .y_label_area_size(50)
            .build_cartesian_2d(0i32..7i32, y_min..y_max)
            .map_err(|e| e.to_string())?;

        chart.configure_mesh()
            .x_labels(8)
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < labels_len { labels[idx].clone() } else { String::new() }
            })
            .y_desc("mmHg")
            .draw()
            .map_err(|e| e.to_string())?;

        // Reference lines: 140 (red), 120 (orange), 80 (blue)
        for (threshold, color, opacity) in [
            (140.0, RGBColor(255, 0, 0), 0.4),
            (120.0, RGBColor(255, 165, 0), 0.4),
            (80.0, RGBColor(0, 112, 192), 0.3),
        ] {
            if threshold >= y_min && threshold <= y_max {
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(0i32, threshold), (7i32, threshold)],
                    ShapeStyle::from(&color.mix(opacity)).stroke_width(1),
                ))).map_err(|e| e.to_string())?;
            }
        }

        // Systolic line — segments between non-empty adjacent buckets
        let sys_points: Vec<(i32, f64)> = buckets.iter().enumerate()
            .filter_map(|(i, b)| b.systolic_avg.map(|v| (i as i32, v)))
            .collect();

        if sys_points.len() >= 2 {
            // Draw connected segments only between adjacent buckets with data
            for window in sys_points.windows(2) {
                chart.draw_series(LineSeries::new(
                    vec![window[0], window[1]],
                    RED.stroke_width(3),
                )).map_err(|e| e.to_string())?;
            }
        }
        // Systolic data points
        chart.draw_series(sys_points.iter().map(|(x, y)| {
            Circle::new((*x, *y), 5, RED.filled())
        })).map_err(|e| e.to_string())?;

        // Diastolic line
        let dia_points: Vec<(i32, f64)> = buckets.iter().enumerate()
            .filter_map(|(i, b)| b.diastolic_avg.map(|v| (i as i32, v)))
            .collect();

        if dia_points.len() >= 2 {
            for window in dia_points.windows(2) {
                chart.draw_series(LineSeries::new(
                    vec![window[0], window[1]],
                    BLUE.stroke_width(3),
                )).map_err(|e| e.to_string())?;
            }
        }
        // Diastolic data points
        chart.draw_series(dia_points.iter().map(|(x, y)| {
            Circle::new((*x, *y), 5, BLUE.filled())
        })).map_err(|e| e.to_string())?;

        // Legend
        // Add dummy series for legend labels
        chart.draw_series(LineSeries::new(
            vec![(-10i32, 0.0)], RED.stroke_width(3),
        )).map_err(|e| e.to_string())?
        .label("Systolic (avg)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(3)));

        chart.draw_series(LineSeries::new(
            vec![(-10i32, 0.0)], BLUE.stroke_width(3),
        )).map_err(|e| e.to_string())?
        .label("Diastolic (avg)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(3)));

        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()
            .map_err(|e| e.to_string())?;

        root.present().map_err(|e| e.to_string())?;
    }

    // Convert RGB buffer to PNG
    let img = RgbImage::from_raw(width, height, buffer)
        .ok_or("Failed to create image from buffer")?;

    let mut png_bytes = Vec::new();
    let dyn_img = DynamicImage::ImageRgb8(img);
    dyn_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(png_bytes)
}

/// Generate HR trend chart as PNG bytes
pub fn generate_hr_chart(daily_stats: &[DailyHRStats], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use plotters::prelude::*;

    if daily_stats.is_empty() {
        return Err("No data to chart".to_string());
    }

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        // Calculate Y axis range
        let y_min = daily_stats.iter()
            .map(|s| s.hr_min)
            .fold(f64::INFINITY, f64::min)
            .max(30.0) - 10.0;
        let y_max = daily_stats.iter()
            .map(|s| s.hr_max)
            .fold(f64::NEG_INFINITY, f64::max)
            .min(150.0) + 10.0;

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(
                0..(daily_stats.len() as i32),
                y_min..y_max
            )
            .map_err(|e| e.to_string())?;

        // Pre-compute date labels to avoid closure issues with large datasets
        let date_labels: Vec<String> = daily_stats.iter()
            .map(|s| s.date.split('-').skip(1).collect::<Vec<_>>().join("/"))
            .collect();
        let labels_len = date_labels.len();

        chart.configure_mesh()
            .x_labels(daily_stats.len().min(10))
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < labels_len {
                    date_labels[idx].clone()
                } else {
                    String::new()
                }
            })
            .y_desc("BPM")
            .draw()
            .map_err(|e| e.to_string())?;

        // Reference lines
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(0, 100.0), (daily_stats.len() as i32, 100.0)],
            RGBColor(255, 165, 0).stroke_width(1),
        ))).map_err(|e| e.to_string())?;

        chart.draw_series(std::iter::once(PathElement::new(
            vec![(0, 60.0), (daily_stats.len() as i32, 60.0)],
            RGBColor(0, 176, 80).stroke_width(1),
        ))).map_err(|e| e.to_string())?;

        chart.draw_series(std::iter::once(PathElement::new(
            vec![(0, 50.0), (daily_stats.len() as i32, 50.0)],
            RGBColor(0, 112, 192).stroke_width(1),
        ))).map_err(|e| e.to_string())?;

        // HR min-max band (purple cloud)
        // Create polygon: go along max values, then back along min values
        let mut hr_polygon: Vec<(i32, f64)> = Vec::new();
        for (i, s) in daily_stats.iter().enumerate() {
            hr_polygon.push((i as i32, s.hr_max));
        }
        for (i, s) in daily_stats.iter().enumerate().rev() {
            hr_polygon.push((i as i32, s.hr_min));
        }
        if !hr_polygon.is_empty() {
            hr_polygon.push(hr_polygon[0]); // Close the polygon
        }
        chart.draw_series(std::iter::once(Polygon::new(
            hr_polygon,
            RGBColor(112, 48, 160).mix(0.15),
        ))).map_err(|e| e.to_string())?;

        // HR average line
        let hr_points: Vec<(i32, f64)> = daily_stats.iter()
            .enumerate()
            .map(|(i, s)| (i as i32, s.hr_avg))
            .collect();

        chart.draw_series(LineSeries::new(
            hr_points.clone(),
            RGBColor(112, 48, 160).stroke_width(2),
        ))
        .map_err(|e| e.to_string())?
        .label("Heart Rate (avg)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RGBColor(112, 48, 160).stroke_width(2)));

        // Only show data point markers for reports of 31 days or less
        if daily_stats.len() <= 31 {
            chart.draw_series(hr_points.iter().map(|(x, y)| {
                Circle::new((*x, *y), 4, RGBColor(112, 48, 160).filled())
            })).map_err(|e| e.to_string())?;
        }

        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(WHITE)
            .border_style(BLACK)
            .draw()
            .map_err(|e| e.to_string())?;

        root.present().map_err(|e| e.to_string())?;
    }

    // Convert RGB buffer to PNG
    let img = RgbImage::from_raw(width, height, buffer)
        .ok_or("Failed to create image from buffer")?;

    let mut png_bytes = Vec::new();
    let dyn_img = DynamicImage::ImageRgb8(img);
    dyn_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(png_bytes)
}

// ============================================================================
// PDF Generation Helper Functions
// ============================================================================

fn mm_to_pt(mm: f32) -> Pt {
    Pt(mm * 2.834645669)
}

fn rgb_to_printpdf(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(Rgb::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        None,
    ))
}

fn add_text(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    text: &str,
    x: Mm,
    y: Mm,
    size: f32,
    color: (u8, u8, u8),
) {
    layer.set_fill_color(rgb_to_printpdf(color.0, color.1, color.2));
    layer.use_text(text, size, x, y, font);
}

fn add_line(
    layer: &PdfLayerReference,
    x1: Mm,
    y1: Mm,
    x2: Mm,
    y2: Mm,
    color: (u8, u8, u8),
    width: f32,
) {
    layer.set_outline_color(rgb_to_printpdf(color.0, color.1, color.2));
    layer.set_outline_thickness(width);

    let line = Line {
        points: vec![
            (Point::new(x1, y1), false),
            (Point::new(x2, y2), false),
        ],
        is_closed: false,
    };
    layer.add_line(line);
}

// ============================================================================
// BP Report Generation
// ============================================================================

/// Generate a Blood Pressure PDF report
pub fn generate_bp_report(
    db: &Database,
    start_date: &str,
    end_date: &str,
    output_path: &str,
    notes: Option<Vec<String>>,
) -> Result<GenerateReportResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get patient info
    let patient = PatientInfo::get(&conn)
        .map_err(|e| e.to_string())?
        .ok_or("Patient info not set. Please call set_patient_info first.")?;

    // Fetch BP vitals for date range
    let start_ts = format!("{}T00:00:00", start_date);
    let end_ts = format!("{}T23:59:59", end_date);

    let vitals = Vital::list_by_date_range(&conn, &start_ts, &end_ts, Some(VitalType::BloodPressure))
        .map_err(|e| e.to_string())?;

    if vitals.is_empty() {
        return Err(format!("No blood pressure readings found between {} and {}", start_date, end_date));
    }

    // Calculate statistics
    let daily_stats = aggregate_daily_bp_stats(&vitals);
    let total_readings = vitals.len() as i64;
    let days_analyzed = daily_stats.len() as i64;

    // Overall averages
    let overall_systolic: f64 = vitals.iter().map(|v| v.value1).sum::<f64>() / total_readings as f64;
    let overall_diastolic: f64 = vitals.iter()
        .filter_map(|v| v.value2)
        .sum::<f64>() / total_readings as f64;

    let (classification, class_color) = classify_bp(overall_systolic, overall_diastolic);

    // Create PDF - Page 1 Portrait
    let (doc, page1, layer1) = PdfDocument::new(
        "Blood Pressure Report",
        Mm(215.9),  // Letter width
        Mm(279.4),  // Letter height
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let layer = doc.get_page(page1).get_layer(layer1);

    // Page 1 dimensions (Portrait)
    let page_height = 279.4;
    let margin_left = 15.0;
    let mut y = page_height - 20.0;

    // Title
    add_text(&layer, &font_bold, "Blood Pressure Report", Mm(margin_left), Mm(y), 18.0, COLOR_BP_TITLE);
    y -= 10.0;

    // Patient info
    add_text(&layer, &font, &format!("Patient: {}", patient.name), Mm(margin_left), Mm(y), 11.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("DOB: {}", patient.dob), Mm(120.0), Mm(y), 11.0, COLOR_BLACK);
    y -= 6.0;

    add_text(&layer, &font, &format!("Report Period: {} to {}", start_date, end_date), Mm(margin_left), Mm(y), 11.0, COLOR_BLACK);
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
    add_text(&layer, &font, &format!("Generated: {}", now), Mm(120.0), Mm(y), 11.0, COLOR_BLACK);
    y -= 10.0;

    // Horizontal line
    add_line(&layer, Mm(margin_left), Mm(y), Mm(200.0), Mm(y), COLOR_GRAY, 0.5);
    y -= 8.0;

    // Summary section
    add_text(&layer, &font_bold, "Summary", Mm(margin_left), Mm(y), 12.0, COLOR_BLACK);
    y -= 7.0;

    add_text(&layer, &font, &format!("Total Readings: {}", total_readings), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Days Monitored: {}", days_analyzed), Mm(80.0), Mm(y), 10.0, COLOR_BLACK);
    y -= 6.0;

    add_text(&layer, &font, &format!("Overall Average: {:.0}/{:.0} mmHg", overall_systolic, overall_diastolic), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Classification: {}", classification), Mm(80.0), Mm(y), 10.0, class_color);
    y -= 6.0;

    // Systolic range
    let sys_min = vitals.iter().map(|v| v.value1).fold(f64::INFINITY, f64::min);
    let sys_max = vitals.iter().map(|v| v.value1).fold(f64::NEG_INFINITY, f64::max);
    add_text(&layer, &font, &format!("Systolic Range: {:.0} - {:.0} mmHg", sys_min, sys_max), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);

    // Diastolic range
    let dia_min = vitals.iter().filter_map(|v| v.value2).fold(f64::INFINITY, f64::min);
    let dia_max = vitals.iter().filter_map(|v| v.value2).fold(f64::NEG_INFINITY, f64::max);
    add_text(&layer, &font, &format!("Diastolic Range: {:.0} - {:.0} mmHg", dia_min, dia_max), Mm(80.0), Mm(y), 10.0, COLOR_BLACK);
    y -= 12.0;

    // Daily statistics table
    add_text(&layer, &font_bold, "Daily Statistics", Mm(margin_left), Mm(y), 12.0, COLOR_BLACK);
    y -= 7.0;

    // Table header
    let col_widths = [20.0, 12.0, 10.0, 22.0, 14.0, 14.0, 14.0, 22.0, 14.0, 14.0, 14.0];
    let headers = ["Date", "Day", "N", "Sys Avg", "SD", "Low", "High", "Dia Avg", "SD", "Low", "High"];

    let mut col_x = margin_left;
    for (i, header) in headers.iter().enumerate() {
        add_text(&layer, &font_bold, header, Mm(col_x), Mm(y), 8.0, COLOR_BLACK);
        col_x += col_widths[i];
    }
    y -= 5.0;

    // Table rows - ALL days (no limit)
    for stats in daily_stats.iter() {
        col_x = margin_left;

        // Determine row color based on systolic avg
        let (_, row_color) = classify_bp(stats.systolic_avg, stats.diastolic_avg);

        let values = [
            stats.date.clone(),
            stats.day_of_week.clone(),
            stats.count.to_string(),
            format!("{:.0}", stats.systolic_avg),
            format!("{:.1}", stats.systolic_sd),
            format!("{:.0}", stats.systolic_min),
            format!("{:.0}", stats.systolic_max),
            format!("{:.0}", stats.diastolic_avg),
            format!("{:.1}", stats.diastolic_sd),
            format!("{:.0}", stats.diastolic_min),
            format!("{:.0}", stats.diastolic_max),
        ];

        for (i, value) in values.iter().enumerate() {
            let color = if i >= 3 && i <= 6 { row_color } else if i >= 7 { row_color } else { COLOR_BLACK };
            add_text(&layer, &font, value, Mm(col_x), Mm(y), 7.0, color);
            col_x += col_widths[i];
        }
        y -= 4.5;
    }

    // ========================================================================
    // Page 2 - Landscape for Chart
    // ========================================================================
    let (page2, layer2) = doc.add_page(Mm(279.4), Mm(215.9), "Chart Page");  // Landscape
    let layer2 = doc.get_page(page2).get_layer(layer2);

    let landscape_width = 279.4;
    let landscape_height = 215.9;
    let margin_left_p2 = 15.0;
    let mut y2 = landscape_height - 20.0;

    // Chart title
    add_text(&layer2, &font_bold, "Blood Pressure Trend", Mm(margin_left_p2), Mm(y2), 16.0, COLOR_BP_TITLE);
    add_text(&layer2, &font, &format!("{} - {}", start_date, end_date), Mm(120.0), Mm(y2), 11.0, COLOR_BLACK);
    y2 -= 10.0;

    // Generate and embed chart (larger for landscape)
    match generate_bp_chart(&daily_stats, 1000, 400) {
        Ok(png_bytes) => {
            let dynamic_image = printpdf::image_crate::load_from_memory(&png_bytes)
                .map_err(|e| e.to_string())?;
            let pdf_image = Image::from_dynamic_image(&dynamic_image);

            // 1000x400 pixels at 120 DPI = ~212mm x 85mm - fits well on landscape
            let transform = ImageTransform {
                translate_x: Some(Mm(margin_left_p2)),
                translate_y: Some(Mm(y2 - 90.0)),
                dpi: Some(120.0),
                ..Default::default()
            };

            pdf_image.add_to_layer(layer2.clone(), transform);
            y2 -= 95.0;
        }
        Err(e) => {
            add_text(&layer2, &font, &format!("Chart generation error: {}", e), Mm(margin_left_p2), Mm(y2 - 10.0), 9.0, COLOR_HIGH);
            y2 -= 15.0;
        }
    }

    // Legend
    y2 -= 5.0;
    add_text(&layer2, &font_bold, "Legend:", Mm(margin_left_p2), Mm(y2), 10.0, COLOR_BLACK);
    add_text(&layer2, &font, "Normal (<120)", Mm(45.0), Mm(y2), 10.0, COLOR_NORMAL);
    add_text(&layer2, &font, "Elevated (120-129)", Mm(90.0), Mm(y2), 10.0, COLOR_ELEVATED);
    add_text(&layer2, &font, "Stage 1 HTN (130-139)", Mm(150.0), Mm(y2), 10.0, COLOR_HIGH);
    add_text(&layer2, &font, "Stage 2 HTN (>=140)", Mm(215.0), Mm(y2), 10.0, COLOR_HIGH);
    y2 -= 10.0;

    // Clinical notes on page 2
    if let Some(ref notes_list) = notes {
        if !notes_list.is_empty() {
            add_text(&layer2, &font_bold, "Clinical Notes", Mm(margin_left_p2), Mm(y2), 12.0, COLOR_BLACK);
            y2 -= 6.0;

            for note in notes_list {
                add_text(&layer2, &font, &format!("- {}", note), Mm(margin_left_p2), Mm(y2), 9.0, COLOR_BLACK);
                y2 -= 5.0;
            }
        }
    }

    // ========================================================================
    // Page 3 - Landscape: BP Time-of-Day Analysis
    // ========================================================================
    let tod_buckets = aggregate_time_of_day_bp(&vitals);
    let buckets_with_data = tod_buckets.iter().filter(|b| b.count > 0).count();

    // Only add page if data spans at least 2 time windows
    if buckets_with_data >= 2 {
        let (page3, layer3) = doc.add_page(Mm(279.4), Mm(215.9), "Time-of-Day Page");
        let layer3 = doc.get_page(page3).get_layer(layer3);

        let landscape_height_p3 = 215.9_f32;
        let margin_left_p3 = 15.0_f32;
        let mut y3 = landscape_height_p3 - 20.0;

        // Header
        add_text(&layer3, &font_bold, "Blood Pressure by Time of Day", Mm(margin_left_p3), Mm(y3), 16.0, COLOR_BP_TITLE);
        add_text(&layer3, &font, &format!("{} - {}", start_date, end_date), Mm(140.0), Mm(y3), 11.0, COLOR_BLACK);
        y3 -= 7.0;
        add_text(&layer3, &font, &format!("Patient: {} | {} readings in {} time windows",
            patient.name, total_readings, buckets_with_data), Mm(margin_left_p3), Mm(y3), 10.0, COLOR_GRAY);
        y3 -= 8.0;

        // Chart
        match generate_bp_time_of_day_chart(&tod_buckets, 1000, 380) {
            Ok(png_bytes) => {
                let dynamic_image = printpdf::image_crate::load_from_memory(&png_bytes)
                    .map_err(|e| e.to_string())?;
                let pdf_image = Image::from_dynamic_image(&dynamic_image);

                let transform = ImageTransform {
                    translate_x: Some(Mm(margin_left_p3)),
                    translate_y: Some(Mm(y3 - 85.0)),
                    dpi: Some(120.0),
                    ..Default::default()
                };

                pdf_image.add_to_layer(layer3.clone(), transform);
                y3 -= 90.0;
            }
            Err(e) => {
                add_text(&layer3, &font, &format!("Chart error: {}", e), Mm(margin_left_p3), Mm(y3 - 10.0), 9.0, COLOR_HIGH);
                y3 -= 15.0;
            }
        }

        // Reference line legend
        y3 -= 5.0;
        add_text(&layer3, &font_bold, "Reference Lines:", Mm(margin_left_p3), Mm(y3), 9.0, COLOR_BLACK);
        add_text(&layer3, &font, "120 mmHg (Normal SYS ceiling)", Mm(55.0), Mm(y3), 9.0, COLOR_ELEVATED);
        add_text(&layer3, &font, "140 mmHg (Stage 1 HTN)", Mm(130.0), Mm(y3), 9.0, COLOR_HIGH);
        add_text(&layer3, &font, "80 mmHg (Normal DIA ceiling)", Mm(200.0), Mm(y3), 9.0, COLOR_BRADYCARDIA);
        y3 -= 10.0;

        // Stats table
        add_text(&layer3, &font_bold, "Time Window Statistics", Mm(margin_left_p3), Mm(y3), 12.0, COLOR_BLACK);
        y3 -= 7.0;

        // Table header — two columns of 4 windows each, side by side
        let tod_col_widths: [f32; 4] = [22.0, 24.0, 24.0, 16.0];
        let tod_headers = ["Window", "Sys Avg", "Dia Avg", "N"];
        let col_offset_right: f32 = 100.0;

        // Left header
        let mut col_x = margin_left_p3;
        for (i, header) in tod_headers.iter().enumerate() {
            add_text(&layer3, &font_bold, header, Mm(col_x), Mm(y3), 9.0, COLOR_BLACK);
            col_x += tod_col_widths[i];
        }
        // Right header
        col_x = margin_left_p3 + col_offset_right;
        for (i, header) in tod_headers.iter().enumerate() {
            add_text(&layer3, &font_bold, header, Mm(col_x), Mm(y3), 9.0, COLOR_BLACK);
            col_x += tod_col_widths[i];
        }
        y3 -= 1.5;
        add_line(&layer3, Mm(margin_left_p3), Mm(y3), Mm(margin_left_p3 + 85.0), Mm(y3), COLOR_LIGHT_GRAY, 0.3);
        add_line(&layer3, Mm(margin_left_p3 + col_offset_right), Mm(y3), Mm(margin_left_p3 + col_offset_right + 85.0), Mm(y3), COLOR_LIGHT_GRAY, 0.3);
        y3 -= 4.0;

        // Table rows — 4 per column
        for row in 0..4usize {
            for col_side in 0..2usize {
                let bucket_idx = row + col_side * 4;
                let b = &tod_buckets[bucket_idx];
                let base_x = margin_left_p3 + (col_side as f32) * col_offset_right;

                add_text(&layer3, &font, b.label, Mm(base_x), Mm(y3), 9.0, COLOR_BLACK);

                match b.systolic_avg {
                    Some(sys) => {
                        let sys_color = if sys >= 140.0 { COLOR_HIGH }
                            else if sys >= 120.0 { COLOR_ELEVATED }
                            else { COLOR_NORMAL };
                        add_text(&layer3, &font_bold, &format!("{:.0}", sys), Mm(base_x + tod_col_widths[0]), Mm(y3), 9.0, sys_color);
                    }
                    None => {
                        add_text(&layer3, &font, "\u{2014}", Mm(base_x + tod_col_widths[0]), Mm(y3), 9.0, COLOR_GRAY);
                    }
                }

                match b.diastolic_avg {
                    Some(dia) => {
                        let dia_color = if dia >= 90.0 { COLOR_HIGH }
                            else if dia >= 80.0 { COLOR_ELEVATED }
                            else { COLOR_NORMAL };
                        add_text(&layer3, &font_bold, &format!("{:.0}", dia), Mm(base_x + tod_col_widths[0] + tod_col_widths[1]), Mm(y3), 9.0, dia_color);
                    }
                    None => {
                        add_text(&layer3, &font, "\u{2014}", Mm(base_x + tod_col_widths[0] + tod_col_widths[1]), Mm(y3), 9.0, COLOR_GRAY);
                    }
                }

                let count_str = if b.count > 0 { b.count.to_string() } else { "\u{2014}".to_string() };
                add_text(&layer3, &font, &count_str, Mm(base_x + tod_col_widths[0] + tod_col_widths[1] + tod_col_widths[2]), Mm(y3), 9.0, COLOR_BLACK);
            }
            y3 -= 5.0;
        }
    }

    // Save PDF
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| e.to_string())?;

    Ok(GenerateReportResponse {
        success: true,
        file_path: output_path.to_string(),
        total_readings,
        days_analyzed,
        date_range: format!("{} to {}", start_date, end_date),
        message: format!("BP report generated successfully with {} readings over {} days", total_readings, days_analyzed),
    })
}

// ============================================================================
// HR Report Generation
// ============================================================================

/// Generate a Heart Rate PDF report
pub fn generate_hr_report(
    db: &Database,
    start_date: &str,
    end_date: &str,
    output_path: &str,
    notes: Option<Vec<String>>,
) -> Result<GenerateReportResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get patient info
    let patient = PatientInfo::get(&conn)
        .map_err(|e| e.to_string())?
        .ok_or("Patient info not set. Please call set_patient_info first.")?;

    // Fetch HR vitals for date range
    let start_ts = format!("{}T00:00:00", start_date);
    let end_ts = format!("{}T23:59:59", end_date);

    let vitals = Vital::list_by_date_range(&conn, &start_ts, &end_ts, Some(VitalType::HeartRate))
        .map_err(|e| e.to_string())?;

    if vitals.is_empty() {
        return Err(format!("No heart rate readings found between {} and {}", start_date, end_date));
    }

    // Calculate statistics
    let daily_stats = aggregate_daily_hr_stats(&vitals);
    let total_readings = vitals.len() as i64;
    let days_analyzed = daily_stats.len() as i64;

    // Overall averages
    let overall_hr: f64 = vitals.iter().map(|v| v.value1).sum::<f64>() / total_readings as f64;
    let (classification, class_color) = classify_hr(overall_hr);

    // Count days with readings below 50 bpm
    let days_with_bradycardia = daily_stats.iter()
        .filter(|s| s.hr_min < 50.0)
        .count();

    // Create PDF - Page 1 Portrait
    let (doc, page1, layer1) = PdfDocument::new(
        "Heart Rate Report",
        Mm(215.9),  // Letter width
        Mm(279.4),  // Letter height
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let layer = doc.get_page(page1).get_layer(layer1);

    // Page 1 dimensions (Portrait)
    let page_height = 279.4;
    let margin_left = 15.0;
    let mut y = page_height - 20.0;

    // Title
    add_text(&layer, &font_bold, "Heart Rate Report", Mm(margin_left), Mm(y), 18.0, COLOR_HR_TITLE);
    y -= 10.0;

    // Patient info
    add_text(&layer, &font, &format!("Patient: {}", patient.name), Mm(margin_left), Mm(y), 11.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("DOB: {}", patient.dob), Mm(120.0), Mm(y), 11.0, COLOR_BLACK);
    y -= 6.0;

    add_text(&layer, &font, &format!("Report Period: {} to {}", start_date, end_date), Mm(margin_left), Mm(y), 11.0, COLOR_BLACK);
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
    add_text(&layer, &font, &format!("Generated: {}", now), Mm(120.0), Mm(y), 11.0, COLOR_BLACK);
    y -= 10.0;

    // Horizontal line
    add_line(&layer, Mm(margin_left), Mm(y), Mm(200.0), Mm(y), COLOR_GRAY, 0.5);
    y -= 8.0;

    // Summary section
    add_text(&layer, &font_bold, "Summary", Mm(margin_left), Mm(y), 12.0, COLOR_BLACK);
    y -= 7.0;

    add_text(&layer, &font, &format!("Total Readings: {}", total_readings), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Days Monitored: {}", days_analyzed), Mm(80.0), Mm(y), 10.0, COLOR_BLACK);
    y -= 6.0;

    add_text(&layer, &font, &format!("Overall Average: {:.0} bpm", overall_hr), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Classification: {}", classification), Mm(80.0), Mm(y), 10.0, class_color);
    y -= 6.0;

    // HR range
    let hr_min = vitals.iter().map(|v| v.value1).fold(f64::INFINITY, f64::min);
    let hr_max = vitals.iter().map(|v| v.value1).fold(f64::NEG_INFINITY, f64::max);
    add_text(&layer, &font, &format!("Heart Rate Range: {:.0} - {:.0} bpm", hr_min, hr_max), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);

    if days_with_bradycardia > 0 {
        add_text(&layer, &font, &format!("Days with <50 bpm: {}", days_with_bradycardia), Mm(100.0), Mm(y), 10.0, COLOR_BRADYCARDIA);
    }
    y -= 12.0;

    // Daily statistics table
    add_text(&layer, &font_bold, "Daily Statistics", Mm(margin_left), Mm(y), 12.0, COLOR_BLACK);
    y -= 7.0;

    // Table header
    let col_widths = [28.0, 16.0, 12.0, 28.0, 18.0, 22.0, 22.0];
    let headers = ["Date", "Day", "N", "HR Avg", "SD", "Low", "High"];

    let mut col_x = margin_left;
    for (i, header) in headers.iter().enumerate() {
        add_text(&layer, &font_bold, header, Mm(col_x), Mm(y), 9.0, COLOR_BLACK);
        col_x += col_widths[i];
    }
    y -= 5.0;

    // Table rows - ALL days (no limit)
    for stats in daily_stats.iter() {
        col_x = margin_left;

        // Determine row color based on HR avg
        let (_, row_color) = classify_hr(stats.hr_avg);

        let values = [
            stats.date.clone(),
            stats.day_of_week.clone(),
            stats.count.to_string(),
            format!("{:.0}", stats.hr_avg),
            format!("{:.1}", stats.hr_sd),
            format!("{:.0}", stats.hr_min),
            format!("{:.0}", stats.hr_max),
        ];

        for (i, value) in values.iter().enumerate() {
            let color = if i >= 3 { row_color } else { COLOR_BLACK };
            add_text(&layer, &font, value, Mm(col_x), Mm(y), 8.0, color);
            col_x += col_widths[i];
        }
        y -= 4.5;
    }

    // ========================================================================
    // Page 2 - Landscape for Chart
    // ========================================================================
    let (page2, layer2) = doc.add_page(Mm(279.4), Mm(215.9), "Chart Page");  // Landscape
    let layer2 = doc.get_page(page2).get_layer(layer2);

    let landscape_height = 215.9;
    let margin_left_p2 = 15.0;
    let mut y2 = landscape_height - 20.0;

    // Chart title
    add_text(&layer2, &font_bold, "Heart Rate Trend", Mm(margin_left_p2), Mm(y2), 16.0, COLOR_HR_TITLE);
    add_text(&layer2, &font, &format!("{} - {}", start_date, end_date), Mm(100.0), Mm(y2), 11.0, COLOR_BLACK);
    y2 -= 10.0;

    // Generate and embed chart (larger for landscape)
    match generate_hr_chart(&daily_stats, 1000, 400) {
        Ok(png_bytes) => {
            let dynamic_image = printpdf::image_crate::load_from_memory(&png_bytes)
                .map_err(|e| e.to_string())?;
            let pdf_image = Image::from_dynamic_image(&dynamic_image);

            // 1000x400 pixels at 120 DPI = ~212mm x 85mm - fits well on landscape
            let transform = ImageTransform {
                translate_x: Some(Mm(margin_left_p2)),
                translate_y: Some(Mm(y2 - 90.0)),
                dpi: Some(120.0),
                ..Default::default()
            };

            pdf_image.add_to_layer(layer2.clone(), transform);
            y2 -= 95.0;
        }
        Err(e) => {
            add_text(&layer2, &font, &format!("Chart generation error: {}", e), Mm(margin_left_p2), Mm(y2 - 10.0), 9.0, COLOR_HIGH);
            y2 -= 15.0;
        }
    }

    // Legend
    y2 -= 5.0;
    add_text(&layer2, &font_bold, "Legend:", Mm(margin_left_p2), Mm(y2), 10.0, COLOR_BLACK);
    add_text(&layer2, &font, "Bradycardia (<50)", Mm(45.0), Mm(y2), 10.0, COLOR_BRADYCARDIA);
    add_text(&layer2, &font, "Low Normal (50-59)", Mm(105.0), Mm(y2), 10.0, COLOR_NORMAL);
    add_text(&layer2, &font, "Normal (60-100)", Mm(170.0), Mm(y2), 10.0, COLOR_NORMAL);
    add_text(&layer2, &font, "Elevated (>100)", Mm(230.0), Mm(y2), 10.0, COLOR_ELEVATED);
    y2 -= 10.0;

    // Clinical notes on page 2
    if let Some(ref notes_list) = notes {
        if !notes_list.is_empty() {
            add_text(&layer2, &font_bold, "Clinical Notes", Mm(margin_left_p2), Mm(y2), 12.0, COLOR_BLACK);
            y2 -= 6.0;

            for note in notes_list {
                add_text(&layer2, &font, &format!("- {}", note), Mm(margin_left_p2), Mm(y2), 9.0, COLOR_BLACK);
                y2 -= 5.0;
            }
        }
    }

    // Save PDF
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| e.to_string())?;

    Ok(GenerateReportResponse {
        success: true,
        file_path: output_path.to_string(),
        total_readings,
        days_analyzed,
        date_range: format!("{} to {}", start_date, end_date),
        message: format!("HR report generated successfully with {} readings over {} days", total_readings, days_analyzed),
    })
}

// ============================================================================
// Weight Report Generation
// ============================================================================

/// Daily weight data point
#[derive(Debug, Clone)]
struct DailyWeight {
    date: String,
    weight: f64,
}

/// Generate Weight trend chart as PNG bytes
pub fn generate_weight_chart(daily_weights: &[DailyWeight], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use plotters::prelude::*;

    if daily_weights.is_empty() {
        return Err("No data to chart".to_string());
    }

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        // Calculate Y axis range with some padding
        let weight_min = daily_weights.iter()
            .map(|w| w.weight)
            .fold(f64::INFINITY, f64::min);
        let weight_max = daily_weights.iter()
            .map(|w| w.weight)
            .fold(f64::NEG_INFINITY, f64::max);

        // Add 5 lbs padding on each side, round to nearest 5
        let y_min = ((weight_min - 5.0) / 5.0).floor() * 5.0;
        let y_max = ((weight_max + 5.0) / 5.0).ceil() * 5.0;

        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0..(daily_weights.len() as i32),
                y_min..y_max
            )
            .map_err(|e| e.to_string())?;

        // Limit labels to avoid crowding, max 10 for readability
        let num_labels = daily_weights.len().min(10);

        // Pre-compute date labels to avoid closure issues with large datasets
        let date_labels: Vec<String> = daily_weights.iter().map(|w| {
            let parts: Vec<&str> = w.date.split('-').collect();
            if parts.len() == 3 {
                format!("{}/{}", parts[1], parts[2])
            } else {
                w.date.clone()
            }
        }).collect();
        let labels_len = date_labels.len();

        chart.configure_mesh()
            .x_labels(num_labels)
            .x_label_formatter(&|x| {
                let idx = *x as usize;
                if idx < labels_len {
                    date_labels[idx].clone()
                } else {
                    String::new()
                }
            })
            .y_desc("Weight (lbs)")
            .y_label_formatter(&|y| format!("{:.0}", y))
            .draw()
            .map_err(|e| format!("Chart mesh error: {}", e))?;

        // Weight line - green color
        let weight_points: Vec<(i32, f64)> = daily_weights.iter()
            .enumerate()
            .map(|(i, w)| (i as i32, w.weight))
            .collect();

        chart.draw_series(LineSeries::new(
            weight_points.clone(),
            RGBColor(0, 128, 0).stroke_width(2),
        ))
        .map_err(|e| e.to_string())?
        .label("Weight")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RGBColor(0, 128, 0).stroke_width(2)));

        // Only show data point markers for reports of 31 days or less
        if daily_weights.len() <= 31 {
            chart.draw_series(weight_points.iter().map(|(x, y)| {
                Circle::new((*x, *y), 4, RGBColor(0, 128, 0).filled())
            })).map_err(|e| e.to_string())?;
        }

        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()
            .map_err(|e| e.to_string())?;

        root.present().map_err(|e| e.to_string())?;
    }

    // Convert RGB buffer to PNG
    let img = RgbImage::from_raw(width, height, buffer)
        .ok_or("Failed to create image from buffer")?;

    let mut png_bytes = Vec::new();
    let dyn_img = DynamicImage::ImageRgb8(img);
    dyn_img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(png_bytes)
}

/// Generate a Weight PDF report (single landscape page with chart)
pub fn generate_weight_report(
    db: &Database,
    start_date: &str,
    end_date: &str,
    output_path: &str,
) -> Result<GenerateReportResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get patient info
    let patient = PatientInfo::get(&conn)
        .map_err(|e| e.to_string())?
        .ok_or("Patient info not set. Please call set_patient_info first.")?;

    // Fetch weight vitals for date range
    let start_ts = format!("{}T00:00:00", start_date);
    let end_ts = format!("{}T23:59:59", end_date);

    let vitals = Vital::list_by_date_range(&conn, &start_ts, &end_ts, Some(VitalType::Weight))
        .map_err(|e| e.to_string())?;

    if vitals.is_empty() {
        return Err(format!("No weight readings found between {} and {}", start_date, end_date));
    }

    // Group by date and take the average if multiple readings per day
    let mut by_date: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    for vital in &vitals {
        let date = vital.timestamp.split('T').next().unwrap_or(&vital.timestamp);
        by_date.entry(date.to_string()).or_default().push(vital.value1);
    }

    let daily_weights: Vec<DailyWeight> = by_date.iter()
        .map(|(date, weights)| {
            let avg = weights.iter().sum::<f64>() / weights.len() as f64;
            DailyWeight {
                date: date.clone(),
                weight: avg,
            }
        })
        .collect();

    let total_readings = vitals.len() as i64;
    let days_analyzed = daily_weights.len() as i64;

    // Calculate stats
    let weight_min = daily_weights.iter().map(|w| w.weight).fold(f64::INFINITY, f64::min);
    let weight_max = daily_weights.iter().map(|w| w.weight).fold(f64::NEG_INFINITY, f64::max);
    let weight_avg = daily_weights.iter().map(|w| w.weight).sum::<f64>() / days_analyzed as f64;
    let weight_change = daily_weights.last().map(|w| w.weight).unwrap_or(0.0)
                      - daily_weights.first().map(|w| w.weight).unwrap_or(0.0);

    // Create PDF - Single Landscape Page
    let (doc, page1, layer1) = PdfDocument::new(
        "Weight Report",
        Mm(279.4),  // Landscape width
        Mm(215.9),  // Landscape height
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let layer = doc.get_page(page1).get_layer(layer1);

    let page_height = 215.9;
    let margin_left = 15.0;
    let mut y = page_height - 15.0;

    // Title
    add_text(&layer, &font_bold, "Weight Report", Mm(margin_left), Mm(y), 18.0, (0, 128, 0));
    add_text(&layer, &font, &format!("{} - {}", start_date, end_date), Mm(100.0), Mm(y), 11.0, COLOR_BLACK);
    y -= 8.0;

    // Patient info line
    add_text(&layer, &font, &format!("Patient: {}", patient.name), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("DOB: {}", patient.dob), Mm(100.0), Mm(y), 10.0, COLOR_BLACK);
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
    add_text(&layer, &font, &format!("Generated: {}", now), Mm(180.0), Mm(y), 10.0, COLOR_BLACK);
    y -= 6.0;

    // Summary stats on one line
    add_text(&layer, &font, &format!("Readings: {}", total_readings), Mm(margin_left), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Days: {}", days_analyzed), Mm(55.0), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Avg: {:.1} lbs", weight_avg), Mm(90.0), Mm(y), 10.0, COLOR_BLACK);
    add_text(&layer, &font, &format!("Range: {:.1} - {:.1} lbs", weight_min, weight_max), Mm(140.0), Mm(y), 10.0, COLOR_BLACK);

    let change_color = if weight_change < 0.0 { COLOR_NORMAL } else if weight_change > 0.0 { COLOR_HIGH } else { COLOR_BLACK };
    let change_str = if weight_change >= 0.0 { format!("+{:.1}", weight_change) } else { format!("{:.1}", weight_change) };
    add_text(&layer, &font, &format!("Change: {} lbs", change_str), Mm(220.0), Mm(y), 10.0, change_color);
    y -= 8.0;

    // Generate and embed chart
    match generate_weight_chart(&daily_weights, 1100, 450) {
        Ok(png_bytes) => {
            let dynamic_image = printpdf::image_crate::load_from_memory(&png_bytes)
                .map_err(|e| e.to_string())?;
            let pdf_image = Image::from_dynamic_image(&dynamic_image);

            let transform = ImageTransform {
                translate_x: Some(Mm(margin_left)),
                translate_y: Some(Mm(y - 145.0)),
                dpi: Some(110.0),
                ..Default::default()
            };

            pdf_image.add_to_layer(layer.clone(), transform);
        }
        Err(e) => {
            add_text(&layer, &font, &format!("Chart generation error: {}", e), Mm(margin_left), Mm(y - 10.0), 9.0, COLOR_HIGH);
        }
    }

    // Save PDF
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| e.to_string())?;

    Ok(GenerateReportResponse {
        success: true,
        file_path: output_path.to_string(),
        total_readings,
        days_analyzed,
        date_range: format!("{} to {}", start_date, end_date),
        message: format!("Weight report generated successfully with {} readings over {} days. Change: {} lbs",
                        total_readings, days_analyzed, change_str),
    })
}

// ============================================================================
// Exercise Report Generation (Python-based)
// ============================================================================

/// Generate an exercise performance PDF report with charts and BP recovery analysis.
///
/// Uses Python (matplotlib + reportlab) for chart generation and PDF assembly.
/// Rust handles data collection from SQLite and generates a Python script with
/// embedded data literals, then shells out to Python for rendering.
pub fn generate_exercise_report(
    db: &Database,
    start_date: &str,
    end_date: &str,
    output_path: &str,
    notes: Option<Vec<String>>,
) -> Result<GenerateExerciseReportResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get patient info
    let patient = PatientInfo::get(&conn)
        .map_err(|e| e.to_string())?
        .ok_or("Patient info not set. Please call set_patient_info first.")?;

    // Query exercises in date range
    let mut exercises = Exercise::list_by_date_range(&conn, start_date, end_date)
        .map_err(|e| e.to_string())?;

    if exercises.is_empty() {
        return Err(format!("No exercises found between {} and {}", start_date, end_date));
    }

    // Sort by timestamp ascending for chart display
    exercises.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // Build date labels and collect exercise data
    // We need to get the date from the day table for each exercise
    let mut exercise_data: Vec<ExerciseDataPoint> = Vec::new();
    let mut date_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for ex in &exercises {
        let day = Day::get_by_id(&conn, ex.day_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Day not found for exercise {}", ex.id))?;

        let count = date_counts.entry(day.date.clone()).or_insert(0);
        *count += 1;

        exercise_data.push(ExerciseDataPoint {
            date: day.date.clone(),
            date_occurrence: *count,
            dur: ex.cached_duration_minutes,
            dist: ex.cached_distance_miles,
            cal: ex.cached_calories_burned,
            timestamp: ex.timestamp.clone(),
            post_vital_group_id: ex.post_vital_group_id,
        });
    }

    // Build date labels — handle multiple sessions per day
    let date_max_counts: std::collections::HashMap<String, usize> = {
        let mut m = std::collections::HashMap::new();
        for dp in &exercise_data {
            let entry = m.entry(dp.date.clone()).or_insert(0usize);
            if dp.date_occurrence > *entry {
                *entry = dp.date_occurrence;
            }
        }
        m
    };

    let exercise_labels: Vec<String> = exercise_data.iter().map(|dp| {
        let parsed = NaiveDate::parse_from_str(&dp.date, "%Y-%m-%d");
        let base_label = match parsed {
            Ok(d) => format!("{} {:02}", month_abbrev(d.month()), d.day()),
            Err(_) => dp.date.clone(),
        };
        if date_max_counts.get(&dp.date).copied().unwrap_or(1) > 1 {
            // Multiple sessions on same day — use occurrence number
            format!("{} ({})", base_label, dp.date_occurrence)
        } else {
            base_label
        }
    }).collect();

    // Collect post-exercise BP recovery pairs.
    // Readings may be split across multiple vital groups (e.g., 1st reading in
    // the linked group, 2nd reading in the next group). Collect all vitals from
    // all groups within 0-20 min after exercise end, then pair by timestamp.
    let mut bp_pairs: Vec<BpPair> = Vec::new();

    for dp in &exercise_data {
        if let Some(pair) = collect_post_exercise_bp(&conn, &dp.timestamp, dp.dur, dp.post_vital_group_id) {
            bp_pairs.push(pair);
        }
    }

    // Compute summary stats
    let total_sessions = exercise_data.len() as i64;
    let unique_dates: std::collections::HashSet<&str> = exercise_data.iter()
        .map(|d| d.date.as_str()).collect();
    let days_with_exercise = unique_dates.len() as i64;
    let total_dur: f64 = exercise_data.iter().map(|d| d.dur).sum();
    let total_dist: f64 = exercise_data.iter().map(|d| d.dist).sum();
    let total_cal: f64 = exercise_data.iter().map(|d| d.cal).sum();

    // Determine chart mode based on date range span
    let d1 = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start_date: {}", e))?;
    let d2 = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end_date: {}", e))?;
    let span_days = (d2 - d1).num_days();
    let chart_mode = if span_days <= 31 {
        "bars"
    } else {
        "line_only"
    };

    // Generate Python script
    let python_script = build_exercise_report_python(
        &exercise_data,
        &exercise_labels,
        &bp_pairs,
        &patient.name,
        &patient.dob,
        start_date,
        end_date,
        output_path,
        chart_mode,
        &notes,
    );

    // Write and execute Python script
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("uhm_exercise_report.py");
    std::fs::write(&script_path, &python_script)
        .map_err(|e| format!("Failed to write Python script: {}", e))?;

    let output = std::process::Command::new("python")
        .arg(&script_path)
        .output()
        .map_err(|e| format!("Failed to execute Python: {}. Is Python installed?", e))?;

    // Clean up script
    let _ = std::fs::remove_file(&script_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Python script failed (exit code {:?}):\nstderr: {}\nstdout: {}",
            output.status.code(), stderr, stdout
        ));
    }

    Ok(GenerateExerciseReportResponse {
        file_path: output_path.to_string(),
        sessions: total_sessions,
        days_with_exercise,
        total_duration_minutes: total_dur,
        total_distance_miles: total_dist,
        total_calories_burned: total_cal,
    })
}

/// Internal data point for exercise chart rendering
struct ExerciseDataPoint {
    date: String,
    date_occurrence: usize,
    dur: f64,
    dist: f64,
    cal: f64,
    timestamp: String,
    post_vital_group_id: Option<i64>,
}

/// BP recovery pair data
struct BpPair {
    min1: f64,
    sys1: f64,
    dia1: f64,
    hr1: f64,
    min2: f64,
    sys2: f64,
    dia2: f64,
    hr2: f64,
}

fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "???",
    }
}

/// Parse an ISO timestamp into a NaiveDateTime
fn parse_timestamp(ts: &str) -> Option<chrono::NaiveDateTime> {
    // Try common formats
    chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%SZ")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S"))
        .ok()
}

/// Collect post-exercise BP recovery pair by searching all vital groups
/// in the 0-20 min window after exercise end. Handles both single-group
/// and split-group patterns (readings across consecutive groups).
fn collect_post_exercise_bp(
    conn: &rusqlite::Connection,
    exercise_timestamp: &str,
    duration_minutes: f64,
    linked_group_id: Option<i64>,
) -> Option<BpPair> {
    let exercise_start = parse_timestamp(exercise_timestamp)?;
    let exercise_end = exercise_start + chrono::Duration::seconds((duration_minutes * 60.0) as i64);
    let window_end = exercise_end + chrono::Duration::minutes(20);

    let end_ts = exercise_end.format("%Y-%m-%dT%H:%M:%S").to_string();
    let window_end_ts = window_end.format("%Y-%m-%dT%H:%M:%S").to_string();

    // Find all vital groups with timestamps in the post-exercise window.
    // Also include the explicitly linked group (it may have a timestamp
    // slightly outside the window due to rounding).
    let mut group_ids: Vec<i64> = Vec::new();

    // Query groups by timestamp range
    let mut stmt = conn.prepare(
        "SELECT id FROM vital_groups WHERE timestamp >= ?1 AND timestamp <= ?2"
    ).ok()?;
    let rows = stmt.query_map(rusqlite::params![end_ts, window_end_ts], |row| {
        row.get::<_, i64>(0)
    }).ok()?;
    for row in rows {
        if let Ok(id) = row {
            group_ids.push(id);
        }
    }

    // Also include the linked group if not already found
    if let Some(gid) = linked_group_id {
        if !group_ids.contains(&gid) {
            group_ids.push(gid);
        }
    }

    if group_ids.is_empty() {
        return None;
    }

    // Collect all vitals from these groups
    let mut all_vitals: Vec<Vital> = Vec::new();
    for gid in &group_ids {
        if let Ok(vitals) = Vital::list_by_group(conn, *gid) {
            all_vitals.extend(vitals);
        }
    }

    // Filter BP readings to 0-20 min after exercise end
    let mut bp_readings: Vec<(Vital, f64)> = all_vitals.iter()
        .filter(|v| v.vital_type == VitalType::BloodPressure)
        .filter_map(|v| {
            let ts = parse_timestamp(&v.timestamp)?;
            let offset_min = (ts - exercise_end).num_seconds() as f64 / 60.0;
            if offset_min >= -1.0 && offset_min <= 20.0 {
                Some((v.clone(), offset_min.max(0.0)))
            } else {
                None
            }
        })
        .collect();
    bp_readings.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if bp_readings.len() < 2 {
        return None;
    }

    let (bp1, min1) = &bp_readings[0];
    let (bp2, min2) = &bp_readings[1];

    // Collect all HR readings for closest-match
    let hr_vitals: Vec<&Vital> = all_vitals.iter()
        .filter(|v| v.vital_type == VitalType::HeartRate)
        .collect();

    let hr1 = find_closest_hr(&hr_vitals, &bp1.timestamp);
    let hr2 = find_closest_hr(&hr_vitals, &bp2.timestamp);

    Some(BpPair {
        min1: *min1,
        sys1: bp1.value1,
        dia1: bp1.value2.unwrap_or(0.0),
        hr1,
        min2: *min2,
        sys2: bp2.value1,
        dia2: bp2.value2.unwrap_or(0.0),
        hr2,
    })
}

/// Find the HR reading closest in time to a given timestamp
fn find_closest_hr(hr_readings: &[&Vital], target_ts: &str) -> f64 {
    let target = match parse_timestamp(target_ts) {
        Some(t) => t,
        None => return 0.0,
    };

    hr_readings.iter()
        .filter_map(|hr| {
            parse_timestamp(&hr.timestamp).map(|t| {
                let diff = (t - target).num_seconds().unsigned_abs();
                (diff, hr.value1)
            })
        })
        .min_by_key(|(diff, _)| *diff)
        .map(|(_, val)| val)
        .unwrap_or(0.0)
}

/// Build the Python script string with all data embedded
fn build_exercise_report_python(
    exercises: &[ExerciseDataPoint],
    labels: &[String],
    bp_pairs: &[BpPair],
    patient_name: &str,
    patient_dob: &str,
    start_date: &str,
    end_date: &str,
    output_path: &str,
    chart_mode: &str,
    notes: &Option<Vec<String>>,
) -> String {
    // Build exercise data as Python list of dicts
    let exercises_py: Vec<String> = exercises.iter().zip(labels.iter()).map(|(ex, label)| {
        format!(
            r#"    {{"date_label": "{}", "dur": {:.4}, "dist": {:.6}, "cal": {:.4}}}"#,
            escape_python_str(label),
            ex.dur,
            ex.dist,
            ex.cal,
        )
    }).collect();

    // Build bp_pairs as Python list of dicts
    let bp_pairs_py: Vec<String> = bp_pairs.iter().map(|p| {
        format!(
            r#"    {{"min1": {:.2}, "sys1": {:.1}, "dia1": {:.1}, "hr1": {:.1}, "min2": {:.2}, "sys2": {:.1}, "dia2": {:.1}, "hr2": {:.1}}}"#,
            p.min1, p.sys1, p.dia1, p.hr1, p.min2, p.sys2, p.dia2, p.hr2,
        )
    }).collect();

    // Build notes as Python list or None
    let notes_py = match notes {
        Some(n) if !n.is_empty() => {
            let items: Vec<String> = n.iter()
                .map(|s| format!(r#"    "{}""#, escape_python_str(s)))
                .collect();
            format!("[\n{}\n]", items.join(",\n"))
        }
        _ => "None".to_string(),
    };

    // Escape the output path for Windows
    let output_path_escaped = output_path.replace('\\', "\\\\");

    format!(
        r##"# Auto-generated by UHM Exercise Report tool
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np
import tempfile
import os

# ============================================================================
# Data
# ============================================================================

exercises = [
{}
]

bp_pairs = [
{}
]

patient_name = "{}"
patient_dob = "{}"
start_date = "{}"
end_date = "{}"
output_path = "{}"
chart_mode = "{}"
notes = {}

# ============================================================================
# Colors
# ============================================================================

CHART_BG = "#0d1b2a"
SOFT_BLUE = "#4fc3f7"
GREEN = "#00c853"
AMBER = "#ffd54f"
HIGHLIGHT = "#e94560"
TEXT_WHITE = "#ffffff"
TEXT_GRAY = "#a0a0b0"
CARD_BG = "#16213e"
ACCENT_BLUE = "#0f3460"

# ============================================================================
# Chart: Exercise Overview (3-panel)
# ============================================================================

def render_exercise_overview(exercises, chart_mode, output_path):
    fig, axes = plt.subplots(1, 3, figsize=(10, 3.2), facecolor=CHART_BG)
    x = np.arange(len(exercises))
    dates = [e["date_label"] for e in exercises]

    panels = [
        (axes[0], [e["dur"] for e in exercises], "Duration", "Minutes", SOFT_BLUE),
        (axes[1], [e["dist"] for e in exercises], "Distance", "Miles", GREEN),
        (axes[2], [e["cal"] for e in exercises], "Calories Burned", "Calories", AMBER),
    ]

    for ax, values, title, ylabel, color in panels:
        ax.set_facecolor(CHART_BG)
        avg = np.mean(values)

        if chart_mode == "bars":
            ax.bar(x, values, color=color, alpha=0.85, width=0.7)
        else:
            ax.plot(x, values, color=color, linewidth=2, alpha=0.85)

        ax.axhline(y=avg, color=HIGHLIGHT, linestyle='--', linewidth=1.5,
                    alpha=0.8, label=f'Avg: {{avg:.1f}}')
        ax.set_ylabel(ylabel, color=TEXT_GRAY, fontsize=8)
        ax.set_title(title, color=TEXT_WHITE, fontsize=10, fontweight='bold')
        ax.set_xticks(x)

        rotation = 55 if len(exercises) > 10 else 45
        fontsize = 5.5 if len(exercises) > 15 else 7
        ax.set_xticklabels(dates, rotation=rotation, ha='right',
                           fontsize=fontsize, color=TEXT_GRAY)
        ax.tick_params(axis='y', colors=TEXT_GRAY, labelsize=7)
        ax.legend(fontsize=7, loc='lower right', facecolor=CARD_BG,
                  edgecolor=TEXT_GRAY, labelcolor=TEXT_WHITE)
        for spine in ['top', 'right']:
            ax.spines[spine].set_visible(False)
        for spine in ['left', 'bottom']:
            ax.spines[spine].set_color(TEXT_GRAY)

    fig.suptitle("Exercise Sessions Overview", color=TEXT_WHITE,
                 fontsize=12, fontweight='bold', y=1.02)
    fig.tight_layout()
    fig.savefig(output_path, dpi=200, bbox_inches='tight',
                facecolor=CHART_BG, edgecolor='none')
    plt.close(fig)

# ============================================================================
# Chart: Speed Trend
# ============================================================================

def render_speed_trend(exercises, chart_mode, output_path):
    speeds = [e["dist"] / (e["dur"] / 60) if e["dur"] > 0 else 0 for e in exercises]
    avg_speed = np.mean(speeds) if speeds else 0
    x = np.arange(len(exercises))
    dates = [e["date_label"] for e in exercises]

    fig, ax = plt.subplots(figsize=(9, 2.8), facecolor=CHART_BG)
    ax.set_facecolor(CHART_BG)

    if chart_mode == "bars":
        ax.plot(x, speeds, color=HIGHLIGHT, linewidth=2.5, marker='o',
                markersize=5, markerfacecolor='white',
                markeredgecolor=HIGHLIGHT, zorder=5)
    else:
        ax.plot(x, speeds, color=HIGHLIGHT, linewidth=2, zorder=5)

    ax.axhline(y=avg_speed, color=AMBER, linestyle='--', linewidth=1.5,
               alpha=0.8, label=f'Avg: {{avg_speed:.2f}} mph')
    ax.fill_between(x, speeds, avg_speed,
                    where=[s >= avg_speed for s in speeds],
                    color=GREEN, alpha=0.15)
    ax.fill_between(x, speeds, avg_speed,
                    where=[s < avg_speed for s in speeds],
                    color=HIGHLIGHT, alpha=0.15)

    ax.set_ylabel("Speed (mph)", color=TEXT_GRAY, fontsize=9)
    ax.set_title("Pace Trend", color=TEXT_WHITE, fontsize=11, fontweight='bold')
    ax.set_xticks(x)
    fontsize = 6 if len(exercises) > 15 else 7
    ax.set_xticklabels(dates, rotation=55, ha='right',
                       fontsize=fontsize, color=TEXT_GRAY)
    ax.tick_params(axis='y', colors=TEXT_GRAY, labelsize=8)
    ax.legend(fontsize=8, loc='lower right', facecolor=CARD_BG,
              edgecolor=TEXT_GRAY, labelcolor=TEXT_WHITE)
    for spine in ['top', 'right']:
        ax.spines[spine].set_visible(False)
    for spine in ['left', 'bottom']:
        ax.spines[spine].set_color(TEXT_GRAY)
    ax.grid(axis='y', color=TEXT_GRAY, alpha=0.15, linewidth=0.5)

    fig.tight_layout()
    fig.savefig(output_path, dpi=200, bbox_inches='tight',
                facecolor=CHART_BG, edgecolor='none')
    plt.close(fig)

# ============================================================================
# Chart: BP Recovery Curve
# ============================================================================

def render_bp_recovery(bp_pairs, output_path):
    if not bp_pairs:
        return

    try:
        from scipy.interpolate import make_interp_spline
        has_scipy = True
    except ImportError:
        has_scipy = False

    avg_early_min = np.mean([p["min1"] for p in bp_pairs])
    avg_late_min = np.mean([p["min2"] for p in bp_pairs])
    avg_early_sys = np.mean([p["sys1"] for p in bp_pairs])
    avg_late_sys = np.mean([p["sys2"] for p in bp_pairs])
    avg_early_dia = np.mean([p["dia1"] for p in bp_pairs])
    avg_late_dia = np.mean([p["dia2"] for p in bp_pairs])
    avg_sys_drop = np.mean([p["sys1"] - p["sys2"] for p in bp_pairs])
    avg_dia_drop = np.mean([p["dia1"] - p["dia2"] for p in bp_pairs])

    # Extrapolate to t=0 and t=15
    if avg_late_min == avg_early_min:
        sys_slope = 0
        dia_slope = 0
    else:
        sys_slope = (avg_late_sys - avg_early_sys) / (avg_late_min - avg_early_min)
        dia_slope = (avg_late_dia - avg_early_dia) / (avg_late_min - avg_early_min)
    sys_at_0 = avg_early_sys - sys_slope * avg_early_min
    sys_at_15 = avg_late_sys + sys_slope * (15 - avg_late_min)
    dia_at_0 = avg_early_dia - dia_slope * avg_early_min
    dia_at_15 = avg_late_dia + dia_slope * (15 - avg_late_min)

    t_points = np.array([0, avg_early_min, avg_late_min, 15])
    sys_points = np.array([sys_at_0, avg_early_sys, avg_late_sys, sys_at_15])
    dia_points = np.array([dia_at_0, avg_early_dia, avg_late_dia, dia_at_15])

    t_smooth = np.linspace(0, 15, 100)
    try:
        if has_scipy:
            sys_smooth = make_interp_spline(t_points, sys_points, k=2)(t_smooth)
            dia_smooth = make_interp_spline(t_points, dia_points, k=2)(t_smooth)
        else:
            sys_smooth = np.interp(t_smooth, t_points, sys_points)
            dia_smooth = np.interp(t_smooth, t_points, dia_points)
    except:
        sys_smooth = np.interp(t_smooth, t_points, sys_points)
        dia_smooth = np.interp(t_smooth, t_points, dia_points)

    fig, ax = plt.subplots(figsize=(9, 4.5), facecolor=CHART_BG)
    ax.set_facecolor(CHART_BG)

    # Individual traces (only if <= 20 sessions)
    if len(bp_pairs) <= 20:
        for p in bp_pairs:
            ax.plot([p["min1"], p["min2"]], [p["sys1"], p["sys2"]],
                    color=SOFT_BLUE, alpha=0.15, linewidth=1)
            ax.plot([p["min1"], p["min2"]], [p["dia1"], p["dia2"]],
                    color=GREEN, alpha=0.15, linewidth=1)

    # Average curves
    ax.plot(t_smooth, sys_smooth, color=SOFT_BLUE, linewidth=3,
            label='Avg Systolic', zorder=5)
    ax.plot(t_smooth, dia_smooth, color=GREEN, linewidth=3,
            label='Avg Diastolic', zorder=5)
    ax.fill_between(t_smooth, dia_smooth, sys_smooth,
                    color=SOFT_BLUE, alpha=0.1)

    # Data points at measurement averages
    ax.scatter([avg_early_min, avg_late_min],
              [avg_early_sys, avg_late_sys],
              color=SOFT_BLUE, s=80, zorder=6,
              edgecolors='white', linewidths=1.5)
    ax.scatter([avg_early_min, avg_late_min],
              [avg_early_dia, avg_late_dia],
              color=GREEN, s=80, zorder=6,
              edgecolors='white', linewidths=1.5)

    # Annotations
    ax.annotate(f'{{avg_early_sys:.0f}}/{{avg_early_dia:.0f}}',
                xy=(avg_early_min, avg_early_sys),
                xytext=(avg_early_min+0.5, avg_early_sys+4),
                color='white', fontsize=9, fontweight='bold',
                arrowprops=dict(arrowstyle='->', color=TEXT_GRAY, lw=0.8))
    ax.annotate(f'{{avg_late_sys:.0f}}/{{avg_late_dia:.0f}}',
                xy=(avg_late_min, avg_late_sys),
                xytext=(avg_late_min+0.5, avg_late_sys+4),
                color='white', fontsize=9, fontweight='bold',
                arrowprops=dict(arrowstyle='->', color=TEXT_GRAY, lw=0.8))

    # Reference lines
    ax.axhline(y=120, color=HIGHLIGHT, linestyle=':', linewidth=1,
               alpha=0.5, label='Systolic 120 ref')
    ax.axhline(y=80, color=AMBER, linestyle=':', linewidth=1,
               alpha=0.5, label='Diastolic 80 ref')

    # Summary text box
    textbox = (f"Avg Systolic Drop: {{avg_sys_drop:.0f}} mmHg\n"
               f"Avg Diastolic Drop: {{avg_dia_drop:.0f}} mmHg\n"
               f"Recovery Window: ~{{avg_early_min:.0f}} to "
               f"~{{avg_late_min:.0f}} min")
    props = dict(boxstyle='round,pad=0.5', facecolor=ACCENT_BLUE,
                 alpha=0.8, edgecolor=TEXT_GRAY)
    ax.text(0.02, 0.25, textbox, transform=ax.transAxes, fontsize=8.5,
            verticalalignment='top', color='white', bbox=props)

    # Styling
    ax.set_xlabel("Minutes Post-Exercise", color='white', fontsize=11)
    ax.set_ylabel("mmHg", color='white', fontsize=11)
    ax.set_title("Post-Exercise BP Recovery — Average Trend",
                 color='white', fontsize=13, fontweight='bold', pad=15)
    ax.set_xlim(-0.5, 16)
    ax.set_ylim(45, 145)
    ax.legend(fontsize=8, loc='upper right', facecolor=CARD_BG,
              edgecolor=TEXT_GRAY, labelcolor='white')
    ax.tick_params(colors=TEXT_GRAY, labelsize=9)
    for spine in ['top', 'right']:
        ax.spines[spine].set_visible(False)
    for spine in ['left', 'bottom']:
        ax.spines[spine].set_color(TEXT_GRAY)
    ax.grid(axis='y', color=TEXT_GRAY, alpha=0.15, linewidth=0.5)

    fig.tight_layout()
    fig.savefig(output_path, dpi=200, bbox_inches='tight',
                facecolor=CHART_BG, edgecolor='none')
    plt.close(fig)

# ============================================================================
# PDF Assembly
# ============================================================================

def build_pdf():
    from reportlab.lib.pagesizes import letter
    from reportlab.lib.units import inch
    from reportlab.lib.colors import HexColor
    from reportlab.platypus import (SimpleDocTemplate, Paragraph, Spacer,
                                     Table, TableStyle, Image, PageBreak)
    from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
    from reportlab.lib.enums import TA_CENTER, TA_LEFT
    from datetime import date

    tmp = tempfile.gettempdir()
    overview_png = os.path.join(tmp, "uhm_ex_overview.png")
    speed_png = os.path.join(tmp, "uhm_ex_speed.png")
    bp_png = os.path.join(tmp, "uhm_ex_bp.png")

    # Render charts
    render_exercise_overview(exercises, chart_mode, overview_png)
    render_speed_trend(exercises, chart_mode, speed_png)
    if bp_pairs:
        render_bp_recovery(bp_pairs, bp_png)

    # Build PDF
    doc = SimpleDocTemplate(output_path, pagesize=letter,
                            topMargin=0.5*inch, bottomMargin=0.5*inch,
                            leftMargin=0.6*inch, rightMargin=0.6*inch)
    styles = getSampleStyleSheet()

    title_style = ParagraphStyle('T', parent=styles['Title'],
        fontSize=22, textColor=HexColor('#1a1a2e'),
        spaceAfter=4, fontName='Helvetica-Bold')
    subtitle_style = ParagraphStyle('ST', parent=styles['Normal'],
        fontSize=11, textColor=HexColor('#666666'),
        spaceAfter=16, alignment=TA_CENTER)
    heading_style = ParagraphStyle('H', parent=styles['Heading1'],
        fontSize=14, textColor=HexColor('#0f3460'),
        spaceBefore=14, spaceAfter=8, fontName='Helvetica-Bold')
    body_style = ParagraphStyle('B', parent=styles['Normal'],
        fontSize=10, textColor=HexColor('#333333'),
        spaceAfter=6, leading=14)
    metric_label = ParagraphStyle('ML', parent=styles['Normal'],
        fontSize=8, textColor=HexColor('#888888'), alignment=TA_CENTER)
    metric_value = ParagraphStyle('MV', parent=styles['Normal'],
        fontSize=20, textColor=HexColor('#1a1a2e'),
        alignment=TA_CENTER, fontName='Helvetica-Bold')
    metric_unit = ParagraphStyle('MU', parent=styles['Normal'],
        fontSize=8, textColor=HexColor('#0f3460'), alignment=TA_CENTER)

    story = []

    # --- HEADER ---
    story.append(Paragraph("Exercise Performance Report", title_style))
    story.append(Paragraph(
        f"{{patient_name}} — {{start_date}} to {{end_date}}", subtitle_style))

    # --- METRICS BAR ---
    total = len(exercises)
    total_dur = sum(e["dur"] for e in exercises)
    total_dist = sum(e["dist"] for e in exercises)
    total_cal = sum(e["cal"] for e in exercises)
    avg_dur = total_dur / total if total > 0 else 0
    avg_dist = total_dist / total if total > 0 else 0
    avg_speed = total_dist / (total_dur / 60) if total_dur > 0 else 0
    avg_cal = total_cal / total if total > 0 else 0

    story.append(Paragraph("Performance Summary", heading_style))
    col_w = doc.width / 5
    metrics_data = [
        [Paragraph("SESSIONS", metric_label),
         Paragraph("AVG DURATION", metric_label),
         Paragraph("AVG DISTANCE", metric_label),
         Paragraph("AVG SPEED", metric_label),
         Paragraph("AVG CALORIES", metric_label)],
        [Paragraph(f"<b>{{total}}</b>", metric_value),
         Paragraph(f"<b>{{avg_dur:.1f}}</b>", metric_value),
         Paragraph(f"<b>{{avg_dist:.2f}}</b>", metric_value),
         Paragraph(f"<b>{{avg_speed:.2f}}</b>", metric_value),
         Paragraph(f"<b>{{avg_cal:.0f}}</b>", metric_value)],
        [Paragraph("sessions", metric_unit),
         Paragraph("minutes", metric_unit),
         Paragraph("miles", metric_unit),
         Paragraph("mph", metric_unit),
         Paragraph("kcal/session", metric_unit)],
    ]
    t = Table(metrics_data, colWidths=[col_w]*5)
    t.setStyle(TableStyle([
        ('ALIGN', (0,0), (-1,-1), 'CENTER'),
        ('VALIGN', (0,0), (-1,-1), 'MIDDLE'),
        ('BACKGROUND', (0,0), (-1,-1), HexColor('#f0f4f8')),
        ('BOX', (0,0), (-1,-1), 0.5, HexColor('#dde3ea')),
        ('LINEBELOW', (0,0), (-1,0), 0.5, HexColor('#ccd3da')),
        ('TOPPADDING', (0,0), (-1,-1), 6),
        ('BOTTOMPADDING', (0,0), (-1,-1), 6),
    ]))
    story.append(t)
    story.append(Spacer(1, 6))

    # Totals strip
    totals_text = (f"<b>Totals:</b>  {{total}} sessions  |  "
                   f"{{total_dur:.0f}} min ({{total_dur/60:.1f}} hrs)  |  "
                   f"{{total_dist:.1f}} miles  |  "
                   f"{{total_cal:.0f}} calories burned")
    totals_style = ParagraphStyle('TS', parent=body_style,
        alignment=TA_CENTER, textColor=HexColor('#0f3460'))
    tt = Table([[Paragraph(totals_text, totals_style)]],
               colWidths=[doc.width])
    tt.setStyle(TableStyle([
        ('BACKGROUND', (0,0), (-1,-1), HexColor('#e8edf3')),
        ('ALIGN', (0,0), (-1,-1), 'CENTER'),
        ('TOPPADDING', (0,0), (-1,-1), 8),
        ('BOTTOMPADDING', (0,0), (-1,-1), 8),
        ('BOX', (0,0), (-1,-1), 0.5, HexColor('#ccd3da')),
    ]))
    story.append(tt)
    story.append(Spacer(1, 10))

    # --- CHARTS ---
    story.append(Paragraph("Session Details", heading_style))
    story.append(Image(overview_png, width=doc.width, height=2.8*inch))
    story.append(Spacer(1, 8))

    story.append(Paragraph("Pace Trend", heading_style))
    story.append(Image(speed_png, width=doc.width, height=2.4*inch))

    speeds = [e["dist"] / (e["dur"] / 60) if e["dur"] > 0 else 0 for e in exercises]
    story.append(Spacer(1, 4))
    note_style = ParagraphStyle('N', parent=body_style,
        fontSize=9, textColor=HexColor('#555555'), leftIndent=10)
    story.append(Paragraph(
        f"Speed range: {{min(speeds):.2f}} — {{max(speeds):.2f}} mph "
        f"over {{total}} sessions.", note_style))

    # --- PAGE 2: BP RECOVERY ---
    if bp_pairs and os.path.exists(bp_png):
        story.append(PageBreak())
        story.append(Paragraph("Post-Exercise BP Recovery", heading_style))
        story.append(Image(bp_png, width=doc.width, height=3.8*inch))
        story.append(Spacer(1, 8))

        # Recovery stats table
        story.append(Paragraph("Recovery Statistics", heading_style))
        avg_e_min = np.mean([p["min1"] for p in bp_pairs])
        avg_l_min = np.mean([p["min2"] for p in bp_pairs])
        avg_e_sys = np.mean([p["sys1"] for p in bp_pairs])
        avg_l_sys = np.mean([p["sys2"] for p in bp_pairs])
        avg_e_dia = np.mean([p["dia1"] for p in bp_pairs])
        avg_l_dia = np.mean([p["dia2"] for p in bp_pairs])
        avg_e_hr = np.mean([p["hr1"] for p in bp_pairs])
        late_hrs = [p["hr2"] for p in bp_pairs if p.get("hr2")]
        avg_l_hr = np.mean(late_hrs) if late_hrs else 0

        bp_table_data = [
            ["Metric",
             f"1st Reading (~{{avg_e_min:.0f}} min)",
             f"2nd Reading (~{{avg_l_min:.0f}} min)",
             "Avg Drop"],
            ["Systolic (mmHg)",
             f"{{avg_e_sys:.0f}}", f"{{avg_l_sys:.0f}}",
             f"\u25bc {{avg_e_sys - avg_l_sys:.0f}}"],
            ["Diastolic (mmHg)",
             f"{{avg_e_dia:.0f}}", f"{{avg_l_dia:.0f}}",
             f"\u25bc {{avg_e_dia - avg_l_dia:.0f}}"],
            ["Heart Rate (bpm)",
             f"{{avg_e_hr:.0f}}", f"{{avg_l_hr:.0f}}",
             f"\u25bc {{avg_e_hr - avg_l_hr:.0f}}"],
        ]
        w = doc.width
        bp_t = Table(bp_table_data,
                     colWidths=[w*0.3, w*0.23, w*0.23, w*0.24])
        bp_t.setStyle(TableStyle([
            ('BACKGROUND', (0,0), (-1,0), HexColor('#1a1a2e')),
            ('TEXTCOLOR', (0,0), (-1,0), HexColor('#ffffff')),
            ('FONTNAME', (0,0), (-1,0), 'Helvetica-Bold'),
            ('FONTSIZE', (0,0), (-1,-1), 10),
            ('ALIGN', (1,0), (-1,-1), 'CENTER'),
            ('BACKGROUND', (0,1), (-1,-1), HexColor('#f0f4f8')),
            ('ROWBACKGROUNDS', (0,1), (-1,-1),
             [HexColor('#f0f4f8'), HexColor('#e4e9f0')]),
            ('BOX', (0,0), (-1,-1), 1, HexColor('#1a1a2e')),
            ('LINEBELOW', (0,0), (-1,0), 1, HexColor('#0f3460')),
            ('INNERGRID', (0,0), (-1,-1), 0.5, HexColor('#ccd3da')),
            ('TOPPADDING', (0,0), (-1,-1), 8),
            ('BOTTOMPADDING', (0,0), (-1,-1), 8),
            ('TEXTCOLOR', (3,1), (3,-1), HexColor('#00796b')),
            ('FONTNAME', (3,1), (3,-1), 'Helvetica-Bold'),
        ]))
        story.append(bp_t)
        story.append(Spacer(1, 12))

        # Auto-generated analysis
        story.append(Paragraph("Analysis", heading_style))
        quality = ("excellent" if avg_l_sys < 120 and avg_l_dia < 80
                   else "good" if avg_l_sys < 130 and avg_l_dia < 85
                   else "moderate")
        classification = ("well below 120/80" if avg_l_sys < 120 and avg_l_dia < 80
                          else "near normal" if avg_l_sys < 130 and avg_l_dia < 85
                          else "mildly elevated")
        story.append(Paragraph(
            f"Across {{len(bp_pairs)}} post-exercise measurement sessions, "
            f"blood pressure consistently shows {{quality}} recovery behavior. "
            f"The average systolic reading drops {{avg_e_sys - avg_l_sys:.0f}} mmHg "
            f"between the first and second post-exercise measurements "
            f"(from ~{{avg_e_sys:.0f}} to ~{{avg_l_sys:.0f}} mmHg), typically "
            f"within a {{avg_l_min - avg_e_min:.0f}}-minute window.",
            body_style))
        if late_hrs:
            story.append(Paragraph(
                f"Second readings average {{avg_l_sys:.0f}}/{{avg_l_dia:.0f}} mmHg "
                f"— {{classification}}. Heart rate recovery settles into the "
                f"{{min(late_hrs):.0f}}-{{max(late_hrs):.0f}} bpm range within minutes.",
                body_style))
        else:
            story.append(Paragraph(
                f"Second readings average {{avg_l_sys:.0f}}/{{avg_l_dia:.0f}} mmHg "
                f"— {{classification}}.",
                body_style))

    # --- CLINICAL NOTES ---
    if notes:
        story.append(Spacer(1, 12))
        story.append(Paragraph("Clinical Notes", heading_style))
        for note in notes:
            story.append(Paragraph(f"\u2022 {{note}}", body_style))

    # --- FOOTER ---
    story.append(Spacer(1, 20))
    footer_style = ParagraphStyle('F', parent=body_style,
        fontSize=8, textColor=HexColor('#999999'), alignment=TA_CENTER)
    story.append(Paragraph(
        f"<i>Generated {{date.today().isoformat()}} — "
        f"UHM (Universal Health Manager)</i>", footer_style))

    doc.build(story)

    # Cleanup temp chart files
    for f in [overview_png, speed_png, bp_png]:
        if os.path.exists(f):
            os.remove(f)

# ============================================================================
# Main
# ============================================================================

build_pdf()
print("OK")
"##,
        exercises_py.join(",\n"),
        bp_pairs_py.join(",\n"),
        escape_python_str(patient_name),
        escape_python_str(patient_dob),
        escape_python_str(start_date),
        escape_python_str(end_date),
        output_path_escaped,
        chart_mode,
        notes_py,
    )
}

/// Escape a string for use in a Python string literal
fn escape_python_str(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
}

// ============================================================================
// Day Summary Report Generation
// ============================================================================

use crate::models::{
    ExerciseSegment, FoodItem, MealEntry, MealType, Nutrition,
    Recipe, RecipeIngredient,
};
use crate::nutrition::calculate_nutrition_multiplier;

/// Response for day summary generation
#[derive(Debug, Serialize)]
pub struct DaySummaryResponse {
    pub file_path: String,
    pub summary: DaySummary,
}

/// Summary statistics for the day
#[derive(Debug, Serialize)]
pub struct DaySummary {
    pub date: String,
    pub weight: Option<f64>,
    pub weight_change: Option<f64>,
    pub gross_calories: f64,
    pub net_calories: f64,
    pub protein: f64,
    pub sodium: f64,
    pub exercise_calories: f64,
    pub tier: String,
    pub protein_status: String,
}

/// Ingredient nutrition breakdown for the report
#[derive(Debug)]
struct IngredientNutrition {
    name: String,
    amount: String,
    nutrition: Nutrition,
}

/// Get calorie tier classification
fn get_calorie_tier(gross: f64, net: f64, protein: f64) -> &'static str {
    if net <= 1500.0 && protein >= 140.0 {
        "MEGA Win"
    } else if gross < 2000.0 {
        "Super Win"
    } else if gross < 3000.0 {
        "Win"
    } else {
        "Over Budget"
    }
}

/// Get status emoji and text for a target
fn get_status(target_type: &str, value: f64) -> (String, String) {
    match target_type {
        "gross_calories" => {
            if value < 2000.0 {
                ("✅".to_string(), "On target".to_string())
            } else {
                ("❌".to_string(), "Over".to_string())
            }
        }
        "net_calories" => {
            if value <= 1500.0 {
                ("✅".to_string(), "On target".to_string())
            } else {
                ("❌".to_string(), "Over".to_string())
            }
        }
        "protein" => {
            if value >= 140.0 {
                ("✅".to_string(), "On target".to_string())
            } else {
                ("⚠️".to_string(), "Low".to_string())
            }
        }
        "sodium" => {
            if value < 1800.0 {
                ("✅".to_string(), "On target".to_string())
            } else {
                ("⚠️".to_string(), "High".to_string())
            }
        }
        _ => ("".to_string(), "".to_string()),
    }
}

/// Format a single meal type header for display
fn format_meal_type(meal_type: &MealType) -> &'static str {
    match meal_type {
        MealType::Breakfast => "Breakfast",
        MealType::Lunch => "Lunch",
        MealType::Dinner => "Dinner",
        MealType::Snack => "Snack",
        MealType::Unspecified => "Unspecified",
    }
}

/// Generate a comprehensive markdown day summary
pub fn generate_day_summary(
    db: &Database,
    date: &str,
    output_path: &str,
    include_ingredients: bool,
) -> Result<DaySummaryResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get day by date
    let day = Day::get_by_date(&conn, date)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No data found for date {}", date))?;

    // Get weight for today
    let today_start = format!("{}T00:00:00", date);
    let today_end = format!("{}T23:59:59", date);
    let today_weights = Vital::list_by_date_range(&conn, &today_start, &today_end, Some(VitalType::Weight))
        .map_err(|e| e.to_string())?;
    let today_weight = today_weights.first().map(|v| v.value1);

    // Get weight for yesterday
    let yesterday = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| e.to_string())?
        .pred_opt()
        .ok_or("Invalid date")?;
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let yesterday_start = format!("{}T00:00:00", yesterday_str);
    let yesterday_end = format!("{}T23:59:59", yesterday_str);
    let yesterday_weights = Vital::list_by_date_range(&conn, &yesterday_start, &yesterday_end, Some(VitalType::Weight))
        .map_err(|e| e.to_string())?;
    let yesterday_weight = yesterday_weights.first().map(|v| v.value1);

    let weight_change = match (today_weight, yesterday_weight) {
        (Some(t), Some(y)) => Some(t - y),
        _ => None,
    };

    // Get meal entries for the day
    let meal_entries = MealEntry::get_for_day(&conn, day.id)
        .map_err(|e| e.to_string())?;

    // Get exercises for the day
    let exercises = Exercise::list_for_day(&conn, day.id)
        .map_err(|e| e.to_string())?;

    // Calculate exercise calories
    let exercise_calories: f64 = exercises.iter()
        .map(|e| e.cached_calories_burned)
        .sum();

    // Calculate gross calories and nutrition from day's cached values
    let gross_calories = day.cached_nutrition.calories;
    let net_calories = gross_calories - exercise_calories;
    let protein = day.cached_nutrition.protein;
    let sodium = day.cached_nutrition.sodium;

    // Determine tier
    let tier = get_calorie_tier(gross_calories, net_calories, protein);
    let (protein_emoji, protein_text) = get_status("protein", protein);
    let protein_status = format!("{} {}", protein_emoji, protein_text);

    // Calculate days to birthday (Oct 22, 2026)
    let today_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let birthday = chrono::NaiveDate::from_ymd_opt(2026, 10, 22)
        .ok_or("Invalid birthday date")?;
    let days_to_birthday = (birthday - today_date).num_days();

    // Format the date for display
    let formatted_date = today_date.format("%A, %B %d, %Y").to_string();

    // Build markdown content
    let mut md = String::new();
    md.push_str(&format!("# Food & Exercise Log — {}\n\n", formatted_date));

    // Weight section
    if let Some(w) = today_weight {
        let change_str = match weight_change {
            Some(c) if c > 0.0 => format!("+{:.1}", c),
            Some(c) => format!("{:.1}", c),
            None => "n/a".to_string(),
        };
        md.push_str(&format!("**Morning Weight:** {:.1} lbs ({} from yesterday)\n", w, change_str));
    } else {
        md.push_str("**Morning Weight:** Not recorded\n");
    }

    if days_to_birthday > 0 {
        md.push_str(&format!("**{} days to 65 (Oct 22, 2026)**\n", days_to_birthday));
    }
    md.push_str("\n---\n\n");

    // Group meal entries by meal type
    let mut meals_by_type: std::collections::BTreeMap<String, Vec<&MealEntry>> = std::collections::BTreeMap::new();
    for entry in &meal_entries {
        let key = format_meal_type(&entry.meal_type).to_string();
        meals_by_type.entry(key).or_default().push(entry);
    }

    // Meal sections
    let mut meal_totals: Vec<(String, Nutrition)> = Vec::new();

    for (meal_type_name, entries) in &meals_by_type {
        for entry in entries {
            // Get source name
            let source_name = if let Some(recipe_id) = entry.recipe_id {
                Recipe::get_by_id(&conn, recipe_id)
                    .map_err(|e| e.to_string())?
                    .map(|r| r.name)
                    .unwrap_or_else(|| "Unknown Recipe".to_string())
            } else if let Some(food_item_id) = entry.food_item_id {
                FoodItem::get_by_id(&conn, food_item_id)
                    .map_err(|e| e.to_string())?
                    .map(|f| f.name)
                    .unwrap_or_else(|| "Unknown Food".to_string())
            } else {
                "Unknown".to_string()
            };

            md.push_str(&format!("## {}: {}\n\n", meal_type_name, source_name));

            // Include ingredients breakdown for recipes
            if include_ingredients {
                if let Some(recipe_id) = entry.recipe_id {
                    let ingredients = RecipeIngredient::get_for_recipe(&conn, recipe_id)
                        .map_err(|e| e.to_string())?;

                    if !ingredients.is_empty() {
                        // Build ingredient nutrition list
                        let mut ingredient_data: Vec<IngredientNutrition> = Vec::new();

                        for ing in &ingredients {
                            if let Some(food_item) = FoodItem::get_by_id(&conn, ing.food_item_id)
                                .map_err(|e| e.to_string())?
                            {
                                let multiplier = calculate_nutrition_multiplier(
                                    ing.quantity,
                                    &ing.unit,
                                    food_item.serving_size,
                                    &food_item.serving_unit,
                                    food_item.grams_per_serving,
                                    food_item.ml_per_serving,
                                );
                                let scaled_nutrition = food_item.nutrition.scale(multiplier);

                                ingredient_data.push(IngredientNutrition {
                                    name: food_item.name.clone(),
                                    amount: format!("{:.1} {}", ing.quantity, ing.unit),
                                    nutrition: scaled_nutrition,
                                });
                            }
                        }

                        // Table header
                        md.push_str("| Ingredient | Amount | Cal | Protein | Fat | Carbs | Fiber | Sodium |\n");
                        md.push_str("|------------|--------|----:|--------:|----:|------:|------:|-------:|\n");

                        for ing_data in &ingredient_data {
                            md.push_str(&format!(
                                "| {} | {} | {:.0} | {:.1}g | {:.1}g | {:.1}g | {:.1}g | {:.0}mg |\n",
                                ing_data.name,
                                ing_data.amount,
                                ing_data.nutrition.calories,
                                ing_data.nutrition.protein,
                                ing_data.nutrition.fat,
                                ing_data.nutrition.carbs,
                                ing_data.nutrition.fiber,
                                ing_data.nutrition.sodium,
                            ));
                        }

                        // Meal total row
                        md.push_str(&format!(
                            "| **MEAL TOTAL** | | **{:.0}** | **{:.1}g** | **{:.1}g** | **{:.1}g** | **{:.1}g** | **{:.0}mg** |\n",
                            entry.cached_nutrition.calories,
                            entry.cached_nutrition.protein,
                            entry.cached_nutrition.fat,
                            entry.cached_nutrition.carbs,
                            entry.cached_nutrition.fiber,
                            entry.cached_nutrition.sodium,
                        ));
                        md.push('\n');
                    }
                } else {
                    // Food item - show single row
                    md.push_str("| Item | Cal | Protein | Fat | Carbs | Fiber | Sodium |\n");
                    md.push_str("|------|----:|--------:|----:|------:|------:|-------:|\n");
                    md.push_str(&format!(
                        "| {} | {:.0} | {:.1}g | {:.1}g | {:.1}g | {:.1}g | {:.0}mg |\n\n",
                        source_name,
                        entry.cached_nutrition.calories,
                        entry.cached_nutrition.protein,
                        entry.cached_nutrition.fat,
                        entry.cached_nutrition.carbs,
                        entry.cached_nutrition.fiber,
                        entry.cached_nutrition.sodium,
                    ));
                }
            }

            meal_totals.push((format!("{}: {}", meal_type_name, source_name), entry.cached_nutrition.clone()));
        }
    }

    // Exercise sections
    if !exercises.is_empty() {
        for exercise in &exercises {
            md.push_str(&format!("## Exercise: {}\n\n", exercise.exercise_type.display_name()));

            // Get segments
            let segments = ExerciseSegment::list_for_exercise(&conn, exercise.id)
                .map_err(|e| e.to_string())?;

            md.push_str("| Metric | Value |\n");
            md.push_str("|--------|-------|\n");
            md.push_str(&format!("| Duration | {:.1} minutes |\n", exercise.cached_duration_minutes));
            md.push_str(&format!("| Distance | {:.2} miles |\n", exercise.cached_distance_miles));

            // Calculate average speed
            if exercise.cached_duration_minutes > 0.0 {
                let avg_speed = (exercise.cached_distance_miles / exercise.cached_duration_minutes) * 60.0;
                md.push_str(&format!("| Avg Speed | {:.1} mph |\n", avg_speed));
            }

            md.push_str(&format!("| Calories Burned | {:.0} cal |\n", exercise.cached_calories_burned));

            // Show segments if multiple
            if segments.len() > 1 {
                md.push_str("\n### Segments\n\n");
                md.push_str("| # | Duration | Speed | Distance | Incline | Cal |\n");
                md.push_str("|---|----------|-------|----------|---------|-----|\n");
                for seg in &segments {
                    md.push_str(&format!(
                        "| {} | {:.1} min | {:.1} mph | {:.2} mi | {:.0}% | {:.0} |\n",
                        seg.segment_order,
                        seg.duration_minutes.unwrap_or(0.0),
                        seg.speed_mph.unwrap_or(0.0),
                        seg.distance_miles.unwrap_or(0.0),
                        seg.incline_percent,
                        seg.calories_burned,
                    ));
                }
            }

            // Post-exercise BP/HR recovery
            if let Some(post_group_id) = exercise.post_vital_group_id {
                let post_vitals = Vital::list_by_group(&conn, post_group_id)
                    .map_err(|e| e.to_string())?;

                if !post_vitals.is_empty() {
                    md.push_str("\n### Post-Exercise BP/HR Recovery\n\n");
                    md.push_str("| Time | BP | HR |\n");
                    md.push_str("|------|----|----|\n");

                    // Group by timestamp and show BP+HR together
                    for vital in &post_vitals {
                        match vital.vital_type {
                            VitalType::BloodPressure => {
                                let hr_vital = post_vitals.iter()
                                    .find(|v| v.vital_type == VitalType::HeartRate && v.timestamp == vital.timestamp);
                                let hr_str = hr_vital.map(|v| format!("{:.0}", v.value1)).unwrap_or_else(|| "-".to_string());
                                md.push_str(&format!(
                                    "| Post | {:.0}/{:.0} | {} |\n",
                                    vital.value1,
                                    vital.value2.unwrap_or(0.0),
                                    hr_str,
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }

            md.push('\n');
        }
    }

    // Day Summary section
    md.push_str("## Day Summary\n\n");

    // Meals breakdown table
    md.push_str("### Meals Breakdown\n\n");
    md.push_str("| Meal | Cal | Protein | Fat | Carbs | Fiber | Sodium |\n");
    md.push_str("|------|----:|--------:|----:|------:|------:|-------:|\n");

    for (meal_name, nutrition) in &meal_totals {
        md.push_str(&format!(
            "| {} | {:.0} | {:.1}g | {:.1}g | {:.1}g | {:.1}g | {:.0}mg |\n",
            meal_name,
            nutrition.calories,
            nutrition.protein,
            nutrition.fat,
            nutrition.carbs,
            nutrition.fiber,
            nutrition.sodium,
        ));
    }

    md.push_str(&format!(
        "| **GROSS TOTAL** | **{:.0}** | **{:.1}g** | **{:.1}g** | **{:.1}g** | **{:.1}g** | **{:.0}mg** |\n\n",
        gross_calories,
        protein,
        day.cached_nutrition.fat,
        day.cached_nutrition.carbs,
        day.cached_nutrition.fiber,
        sodium,
    ));

    // Net calories table
    md.push_str("### Net Calories\n\n");
    md.push_str("| Gross Intake | Exercise Burned | Net Calories |\n");
    md.push_str("|-------------:|----------------:|-------------:|\n");
    md.push_str(&format!(
        "| {:.0} | {:.0} | {:.0} |\n\n",
        gross_calories,
        exercise_calories,
        net_calories,
    ));

    md.push_str("---\n\n");

    // Status Check section
    md.push_str("## Status Check\n\n");
    md.push_str("| Target | Goal | Actual | Status |\n");
    md.push_str("|--------|------|-------:|--------|\n");

    let (gross_emoji, gross_text) = get_status("gross_calories", gross_calories);
    md.push_str(&format!("| Calories (gross) | <2000 | {:.0} | {} {} |\n", gross_calories, gross_emoji, gross_text));

    let (net_emoji, net_text) = get_status("net_calories", net_calories);
    md.push_str(&format!("| Net Calories | ≤1500 | {:.0} | {} {} |\n", net_calories, net_emoji, net_text));

    let (prot_emoji, prot_text) = get_status("protein", protein);
    md.push_str(&format!("| Protein | ≥140g | {:.1}g | {} {} |\n", protein, prot_emoji, prot_text));

    let (sod_emoji, sod_text) = get_status("sodium", sodium);
    md.push_str(&format!("| Sodium | <1800mg | {:.0}mg | {} {} |\n\n", sodium, sod_emoji, sod_text));

    md.push_str(&format!("### Current Tier: **{}**\n", tier));

    // Write to file
    let path = std::path::Path::new(output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, &md).map_err(|e| e.to_string())?;

    Ok(DaySummaryResponse {
        file_path: output_path.to_string(),
        summary: DaySummary {
            date: date.to_string(),
            weight: today_weight,
            weight_change,
            gross_calories,
            net_calories,
            protein,
            sodium,
            exercise_calories,
            tier: tier.to_string(),
            protein_status,
        },
    })
}

// ============================================================================
// Medications Report Generation
// ============================================================================

/// Generate a Medication List PDF report
pub fn generate_medications_report(
    db: &Database,
    patient_name: &str,
    output_path: &str,
    active_only: bool,
    include_notes: bool,
) -> Result<GenerateMedicationsReportResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get patient info for header
    let patient = PatientInfo::get(&conn)
        .map_err(|e| e.to_string())?
        .ok_or("Patient info not set. Please call set_patient_info first.")?;

    // Fetch medications
    let meds = Medication::list(&conn, active_only, None)
        .map_err(|e| e.to_string())?;

    if meds.is_empty() {
        return Err(if active_only {
            "No active medications found.".to_string()
        } else {
            "No medications found.".to_string()
        });
    }

    let medication_count = meds.len();

    // Group by MedType
    let mut grouped: std::collections::HashMap<MedType, Vec<&Medication>> = std::collections::HashMap::new();
    for med in &meds {
        grouped.entry(med.med_type).or_default().push(med);
    }

    // Sort group keys by sort_order
    let mut type_keys: Vec<MedType> = grouped.keys().cloned().collect();
    type_keys.sort_by_key(|t| t.sort_order());

    // Create PDF
    let (doc, page1, layer1) = PdfDocument::new(
        "Medication List",
        Mm(215.9),
        Mm(279.4),
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let page_height: f32 = 279.4;
    let margin_left: f32 = 15.0;
    let margin_right: f32 = 200.0;
    let footer_y: f32 = 15.0;
    let min_y: f32 = 30.0;

    let mut current_page = doc.get_page(page1).get_layer(layer1);
    let mut y = page_height - 20.0;
    let mut page_num: u32 = 1;

    // --- Helper closure for header ---
    let render_header = |layer: &PdfLayerReference, y: &mut f32, page: u32| {
        *y = page_height - 20.0;

        // Title
        add_text(layer, &font_bold, "Medication List", Mm(margin_left), Mm(*y), 18.0, COLOR_MED_TITLE);
        *y -= 8.0;

        // Patient info
        add_text(layer, &font, &format!("Patient: {}", patient_name), Mm(margin_left), Mm(*y), 11.0, COLOR_BLACK);
        add_text(layer, &font, &format!("DOB: {}", patient.dob), Mm(120.0), Mm(*y), 11.0, COLOR_BLACK);
        *y -= 6.0;

        let now = chrono::Local::now().format("%Y-%m-%d").to_string();
        let status_label = if active_only { "Active medications only" } else { "All medications (active and inactive)" };
        add_text(layer, &font, status_label, Mm(margin_left), Mm(*y), 10.0, COLOR_GRAY);
        add_text(layer, &font, &format!("Generated: {}", now), Mm(120.0), Mm(*y), 10.0, COLOR_GRAY);
        *y -= 6.0;

        // Divider
        add_line(layer, Mm(margin_left), Mm(*y), Mm(margin_right), Mm(*y), COLOR_GRAY, 0.5);
        *y -= 8.0;

        // Page number footer
        let page_str = format!("Page {}", page);
        add_text(layer, &font, &page_str, Mm(185.0), Mm(footer_y), 8.0, COLOR_GRAY);

        // Disclaimer footer
        add_text(
            layer, &font,
            "For informational purposes only. Not a substitute for professional medical advice.",
            Mm(margin_left), Mm(footer_y), 8.0, COLOR_GRAY,
        );
    };

    // Render header on first page
    render_header(&current_page, &mut y, page_num);

    // Render each medication type group
    for med_type in &type_keys {
        let group = &grouped[med_type];

        // Estimate space needed for section header
        let section_header_space: f32 = 12.0;
        if y - section_header_space < min_y {
            // New page
            page_num += 1;
            let (new_page, new_layer) = doc.add_page(Mm(215.9), Mm(279.4), "Layer 1");
            current_page = doc.get_page(new_page).get_layer(new_layer);
            render_header(&current_page, &mut y, page_num);
        }

        // Section header
        add_text(&current_page, &font_bold, med_type.display_name(), Mm(margin_left), Mm(y), 14.0, COLOR_MED_TITLE);
        y -= 2.0;
        add_line(&current_page, Mm(margin_left), Mm(y), Mm(margin_right), Mm(y), COLOR_MED_TITLE, 0.3);
        y -= 6.0;

        for (med_idx, med) in group.iter().enumerate() {
            // Estimate space for this medication (~25mm with all fields)
            let mut est_lines: f32 = 3.0; // name + dosage line + divider
            if med.instructions.is_some() { est_lines += 1.0; }
            if med.med_type == MedType::Prescription {
                if med.prescribing_doctor.is_some() || med.start_date.is_some() { est_lines += 1.0; }
                if med.pharmacy.is_some() || med.rx_number.is_some() { est_lines += 1.0; }
            }
            if include_notes && med.notes.is_some() { est_lines += 1.0; }
            if !med.is_active { est_lines += 1.0; }
            let est_space = est_lines * 5.0 + 5.0;

            if y - est_space < min_y {
                // New page
                page_num += 1;
                let (new_page, new_layer) = doc.add_page(Mm(215.9), Mm(279.4), "Layer 1");
                current_page = doc.get_page(new_page).get_layer(new_layer);
                render_header(&current_page, &mut y, page_num);
            }

            // Medication name
            let name_label = if !med.is_active {
                format!("{} (INACTIVE)", med.name)
            } else {
                med.name.clone()
            };
            add_text(&current_page, &font_bold, &name_label, Mm(margin_left + 2.0), Mm(y), 12.0, COLOR_BLACK);
            y -= 5.0;

            // Dosage + frequency line
            let freq_str = med.frequency.as_deref().unwrap_or("as needed");
            let dosage_line = format!(
                "{} {} — {}",
                format_dosage_amount(med.dosage_amount),
                med.dosage_unit.display_name(),
                freq_str,
            );
            add_text(&current_page, &font, &dosage_line, Mm(margin_left + 4.0), Mm(y), 10.0, COLOR_BLACK);
            y -= 5.0;

            // Instructions
            if let Some(ref instructions) = med.instructions {
                let instr_line = format!("Instructions: {}", instructions);
                let truncated = if instr_line.len() > 120 { format!("{}...", &instr_line[..117]) } else { instr_line };
                add_text(&current_page, &font, &truncated, Mm(margin_left + 4.0), Mm(y), 10.0, COLOR_BLACK);
                y -= 5.0;
            }

            // Prescription-specific fields
            if med.med_type == MedType::Prescription {
                let mut rx_parts: Vec<String> = Vec::new();
                if let Some(ref doctor) = med.prescribing_doctor {
                    rx_parts.push(format!("Prescribed by {}", doctor));
                }
                if let Some(ref start) = med.start_date {
                    rx_parts.push(format!("Started {}", start));
                }
                if !rx_parts.is_empty() {
                    add_text(&current_page, &font, &rx_parts.join(" | "), Mm(margin_left + 4.0), Mm(y), 9.0, COLOR_GRAY);
                    y -= 4.5;
                }

                let mut rx_detail: Vec<String> = Vec::new();
                if let Some(ref pharmacy) = med.pharmacy {
                    rx_detail.push(format!("Pharmacy: {}", pharmacy));
                }
                if let Some(ref rx_num) = med.rx_number {
                    rx_detail.push(format!("Rx# {}", rx_num));
                }
                if let Some(refills) = med.refills_remaining {
                    rx_detail.push(format!("Refills: {}", refills));
                }
                if !rx_detail.is_empty() {
                    add_text(&current_page, &font, &rx_detail.join(" | "), Mm(margin_left + 4.0), Mm(y), 9.0, COLOR_GRAY);
                    y -= 4.5;
                }
            }

            // Inactive reason
            if !med.is_active {
                if let Some(ref reason) = med.discontinue_reason {
                    let reason_line = format!("Discontinued: {}", reason);
                    add_text(&current_page, &font, &reason_line, Mm(margin_left + 4.0), Mm(y), 9.0, COLOR_GRAY);
                    y -= 4.5;
                }
                if let Some(ref end) = med.end_date {
                    let end_line = format!("End date: {}", end);
                    add_text(&current_page, &font, &end_line, Mm(margin_left + 4.0), Mm(y), 9.0, COLOR_GRAY);
                    y -= 4.5;
                }
            }

            // Notes
            if include_notes {
                if let Some(ref notes) = med.notes {
                    let notes_line = format!("Notes: {}", notes);
                    let truncated = if notes_line.len() > 120 { format!("{}...", &notes_line[..117]) } else { notes_line };
                    add_text(&current_page, &font, &truncated, Mm(margin_left + 4.0), Mm(y), 9.0, COLOR_GRAY);
                    y -= 4.5;
                }
            }

            // Light divider between meds (not after last in group)
            if med_idx < group.len() - 1 {
                y -= 1.0;
                add_line(&current_page, Mm(margin_left + 2.0), Mm(y), Mm(margin_right - 10.0), Mm(y), COLOR_LIGHT_GRAY, 0.2);
                y -= 4.0;
            } else {
                y -= 3.0;
            }
        }

        // Extra space before next section
        y -= 5.0;
    }

    // Save PDF
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(path).map_err(|e| e.to_string())?;
    let writer = BufWriter::new(file);
    doc.save(&mut std::io::BufWriter::new(writer)).map_err(|e| e.to_string())?;

    let status = if active_only { "active" } else { "all" };
    Ok(GenerateMedicationsReportResponse {
        success: true,
        file_path: output_path.to_string(),
        medication_count,
        message: format!("Medication list ({} {}) saved to {}", medication_count, status, output_path),
    })
}

// ============================================================================
// Pill Organizer Report
// ============================================================================

/// Time slot definition for pill organizer
struct PillSlot {
    label: &'static str,
    bg_color: (u8, u8, u8),
    label_color: (u8, u8, u8),
}

/// A medication entry assigned to a time slot
#[derive(Clone)]
struct PillEntry {
    name: String,
    dosage: String,
    med_type_label: String,
    pill_description: String,
}

/// Filled rectangle helper for colored backgrounds
fn add_filled_rect(
    layer: &PdfLayerReference,
    x: f32,
    y_top: f32,
    w: f32,
    h: f32,
    color: (u8, u8, u8),
) {
    layer.set_fill_color(rgb_to_printpdf(color.0, color.1, color.2));
    let polygon = Polygon {
        rings: vec![vec![
            (Point::new(Mm(x), Mm(y_top - h)), false),
            (Point::new(Mm(x + w), Mm(y_top - h)), false),
            (Point::new(Mm(x + w), Mm(y_top)), false),
            (Point::new(Mm(x), Mm(y_top)), false),
        ]],
        mode: PaintMode::Fill,
        winding_order: WindingOrder::NonZero,
    };
    layer.add_polygon(polygon);
}

/// Simple word-wrap: splits text into lines of approximately max_chars width
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

/// Assign a medication to time slot indices based on schedule_slot or frequency
/// Returns indices: 0=Morning, 1=Midday, 2=Evening, 3=Bedtime
fn assign_med_to_slots(med: &Medication) -> Vec<usize> {
    // Check schedule_slot field first (user-defined override)
    if let Some(ref slot) = med.schedule_slot {
        let slot_lower = slot.to_lowercase();
        if slot_lower == "all" {
            return vec![0, 1, 2, 3];
        }
        let mut indices = Vec::new();
        for s in slot_lower.split(',') {
            match s.trim() {
                "morning" | "am" => indices.push(0),
                "midday" | "afternoon" => indices.push(1),
                "evening" | "pm" => indices.push(2),
                "bedtime" => indices.push(3),
                _ => {}
            }
        }
        if !indices.is_empty() {
            indices.sort();
            indices.dedup();
            return indices;
        }
    }

    // Parse from frequency string
    if let Some(ref freq) = med.frequency {
        let freq_lower = freq.to_lowercase();

        // Four times daily → all slots
        if freq_lower.contains("four times daily")
            || freq_lower.contains("4 times daily")
            || freq_lower.contains("4x daily")
        {
            return vec![0, 1, 2, 3];
        }

        // Three times daily
        if freq_lower.contains("three times daily") || freq_lower.contains("3 times daily") {
            return vec![0, 1, 2];
        }

        let mut indices = Vec::new();

        if freq_lower.contains("bedtime") {
            indices.push(3);
        }
        if freq_lower.contains("3 pm")
            || freq_lower.contains("3pm")
            || freq_lower.contains("midday")
            || freq_lower.contains("afternoon")
        {
            indices.push(1);
        }
        if freq_lower.contains("am") || freq_lower.contains("morning") {
            indices.push(0);
        }
        // "PM" means evening, but not if it's already tagged as midday (3 PM)
        if freq_lower.contains("pm") || freq_lower.contains("evening") {
            if !indices.contains(&1) {
                indices.push(2);
            }
        }

        if !indices.is_empty() {
            indices.sort();
            indices.dedup();
            return indices;
        }
    }

    // Default: morning
    vec![0]
}

/// Check if a medication should be excluded from the pill organizer
fn is_non_pill_med(med: &Medication) -> bool {
    // Exclude non-pill dosage units
    let non_pill_units = ["spray", "drop", "patch", "injection", "ml", "fl_oz", "puff"];
    if non_pill_units.contains(&med.dosage_unit.as_str()) {
        return true;
    }

    // Exclude Metamucil / fiber powder
    let name_lower = med.name.to_lowercase();
    if name_lower.contains("metamucil") {
        return true;
    }

    // Check instructions/notes for non-pill forms
    let non_pill_keywords = ["gel", "cream", "spray", "liquid", "powder", "solution", "topical"];
    if let Some(ref instructions) = med.instructions {
        let instr_lower = instructions.to_lowercase();
        for kw in &non_pill_keywords {
            if instr_lower.contains(kw) {
                return true;
            }
        }
    }

    false
}

/// Generate a Pill Organizer PDF report — single-page schedule for filling weekly pill containers
pub fn generate_pill_organizer_report(
    db: &Database,
    patient_name: &str,
    output_path: &str,
) -> Result<GeneratePillOrganizerReportResponse, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Get all active medications
    let meds = Medication::list(&conn, true, None).map_err(|e| e.to_string())?;

    if meds.is_empty() {
        return Err("No active medications found.".to_string());
    }

    // Filter: exclude PRN, non-pill forms
    let pill_meds: Vec<&Medication> = meds
        .iter()
        .filter(|m| {
            // Exclude PRN
            if let Some(ref freq) = m.frequency {
                if freq.to_uppercase().contains("PRN") {
                    return false;
                }
            }
            // Exclude non-pill forms
            !is_non_pill_med(m)
        })
        .collect();

    if pill_meds.is_empty() {
        return Err("No pill-form medications found after filtering.".to_string());
    }

    // Define time slots
    let slots = [
        PillSlot {
            label: "Morning \u{2014} 8:00 AM",
            bg_color: (232, 240, 254),   // #e8f0fe
            label_color: (21, 101, 192), // #1565c0
        },
        PillSlot {
            label: "Midday \u{2014} 3:00 PM",
            bg_color: (255, 243, 205),  // #fff3cd
            label_color: (230, 81, 0),  // #e65100
        },
        PillSlot {
            label: "Evening \u{2014} 8:00 PM",
            bg_color: (232, 245, 233),  // #e8f5e9
            label_color: (46, 125, 50), // #2e7d32
        },
        PillSlot {
            label: "Bedtime",
            bg_color: (243, 229, 245),   // #f3e5f5
            label_color: (106, 27, 154), // #6a1b9a
        },
    ];

    // Assign meds to slots
    let mut slot_entries: Vec<Vec<PillEntry>> = vec![Vec::new(); 4];
    let mut total_entries = 0usize;
    let mut has_multi_slot_med = false;

    for med in &pill_meds {
        let indices = assign_med_to_slots(med);
        if indices.len() > 1 {
            has_multi_slot_med = true;
        }

        let type_label = match med.med_type {
            MedType::Prescription => "Rx",
            MedType::Supplement => "Suppl.",
            MedType::Otc => "OTC",
            _ => med.med_type.display_name(),
        };

        let desc = med
            .pill_description
            .clone()
            .unwrap_or_default();

        let dosage_str = format!(
            "{} {}",
            format_dosage_amount(med.dosage_amount),
            med.dosage_unit.display_name()
        );

        for &idx in &indices {
            if idx < 4 {
                slot_entries[idx].push(PillEntry {
                    name: med.name.clone(),
                    dosage: dosage_str.clone(),
                    med_type_label: type_label.to_string(),
                    pill_description: desc.clone(),
                });
                total_entries += 1;
            }
        }
    }

    // Find primary prescriber for subtitle
    let prescriber = pill_meds
        .iter()
        .filter_map(|m| m.prescribing_doctor.as_deref())
        .next()
        .unwrap_or("—");

    // ========================================================================
    // PDF Generation
    // ========================================================================

    let (doc, page1, layer1) = PdfDocument::new(
        "Pill Organizer Schedule",
        Mm(215.9),
        Mm(279.4),
        "Layer 1",
    );

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;
    let font_italic = doc
        .add_builtin_font(BuiltinFont::HelveticaOblique)
        .map_err(|e| e.to_string())?;

    let layer = doc.get_page(page1).get_layer(layer1);

    // Layout constants (mm)
    let margin: f32 = 12.7; // 0.5 inch
    let page_width: f32 = 215.9;
    let content_width: f32 = page_width - 2.0 * margin;

    // Column positions (mm from left edge)
    let col_name: f32 = margin + 3.5;
    let col_dose: f32 = margin + 48.7;
    let col_type: f32 = margin + 98.1;
    let col_desc: f32 = margin + 110.1;

    // Description column max chars (~50 chars at 9pt Helvetica)
    let desc_max_chars: usize = 50;

    // Colors
    let color_banner: (u8, u8, u8) = (26, 54, 93);       // #1a365d
    let color_white: (u8, u8, u8) = (255, 255, 255);
    let color_subtitle: (u8, u8, u8) = (176, 196, 222);   // #b0c4de
    let color_col_header: (u8, u8, u8) = (158, 158, 158); // #9e9e9e
    let color_divider: (u8, u8, u8) = (176, 190, 197);    // #b0bec5
    let color_text_dark: (u8, u8, u8) = (26, 26, 46);     // #1a1a2e
    let color_text_blue: (u8, u8, u8) = (13, 71, 161);    // #0d47a1
    let color_type_gray: (u8, u8, u8) = (117, 117, 117);  // #757575
    let color_desc_gray: (u8, u8, u8) = (85, 85, 85);     // #555555
    let color_row_alt: (u8, u8, u8) = (248, 249, 250);    // #f8f9fa
    let color_note_bg: (u8, u8, u8) = (227, 242, 253);    // #e3f2fd
    let color_note_label: (u8, u8, u8) = (21, 101, 192);  // #1565c0
    let color_tip_bg: (u8, u8, u8) = (255, 248, 225);     // #fff8e1
    let color_tip_label: (u8, u8, u8) = (230, 81, 0);     // #e65100
    let color_text_body: (u8, u8, u8) = (51, 51, 51);     // #333333
    let color_footer: (u8, u8, u8) = (189, 189, 189);     // #bdbdbd

    let mut y: f32 = 279.4 - margin;

    // --- Title Banner ---
    let banner_h: f32 = 18.4; // ~52pt
    add_filled_rect(&layer, margin, y, content_width, banner_h, color_banner);

    // Title text (centered)
    let title = format!("Medications \u{2014} {}", patient_name);
    // Approximate centering: page center = 107.95mm
    add_text(
        &layer,
        &font_bold,
        &title,
        Mm(page_width / 2.0 - 45.0),
        Mm(y - 8.5),
        22.0,
        color_white,
    );

    // Subtitle
    let subtitle = format!(
        "Daily Pill Container Schedule  \u{2022}  Excludes PRN and non-pill items  \u{2022}  Prescriber: {}",
        prescriber
    );
    add_text(
        &layer,
        &font,
        &subtitle,
        Mm(page_width / 2.0 - 72.0),
        Mm(y - 15.0),
        10.5,
        color_subtitle,
    );

    y -= banner_h + 5.6; // gap below banner

    // --- Column Headers ---
    add_text(&layer, &font_bold, "MEDICATION", Mm(col_name), Mm(y), 8.5, color_col_header);
    add_text(&layer, &font_bold, "DOSAGE", Mm(col_dose), Mm(y), 8.5, color_col_header);
    add_text(&layer, &font_bold, "TYPE", Mm(col_type), Mm(y), 8.5, color_col_header);
    add_text(&layer, &font_bold, "PILL DESCRIPTION", Mm(col_desc), Mm(y), 8.5, color_col_header);

    y -= 2.1;
    add_line(
        &layer,
        Mm(margin),
        Mm(y),
        Mm(page_width - margin),
        Mm(y),
        color_divider,
        0.4,
    );
    y -= 2.8;

    // --- Time Blocks ---
    let section_bar_h: f32 = 7.8;  // 22pt
    let row_h: f32 = 9.9;          // 28pt
    let desc_line_h: f32 = 3.9;    // 11pt for wrapped second line

    for (slot_idx, slot) in slots.iter().enumerate() {
        let entries = &slot_entries[slot_idx];
        if entries.is_empty() {
            continue;
        }

        // Section header bar
        add_filled_rect(&layer, margin, y, content_width, section_bar_h, slot.bg_color);
        add_text(
            &layer,
            &font_bold,
            slot.label,
            Mm(margin + 2.8),
            Mm(y - section_bar_h + 2.1),
            13.0,
            slot.label_color,
        );
        y -= section_bar_h + 1.8;

        // Medication rows
        for (i, entry) in entries.iter().enumerate() {
            // Alternating row background
            if i % 2 == 0 {
                add_filled_rect(
                    &layer,
                    margin + 0.7,
                    y + 2.8,
                    content_width - 1.4,
                    row_h,
                    color_row_alt,
                );
            }

            // Medication name
            add_text(
                &layer,
                &font_bold,
                &entry.name,
                Mm(col_name),
                Mm(y),
                11.0,
                color_text_dark,
            );

            // Dosage
            add_text(
                &layer,
                &font,
                &entry.dosage,
                Mm(col_dose),
                Mm(y),
                10.5,
                color_text_blue,
            );

            // Type (italic)
            add_text(
                &layer,
                &font_italic,
                &entry.med_type_label,
                Mm(col_type),
                Mm(y),
                9.5,
                color_type_gray,
            );

            // Pill description (with word wrap, max 2 lines)
            if !entry.pill_description.is_empty() {
                let lines = wrap_text(&entry.pill_description, desc_max_chars);
                let mut desc_y = y;
                for line in lines.iter().take(2) {
                    add_text(
                        &layer,
                        &font,
                        line,
                        Mm(col_desc),
                        Mm(desc_y),
                        9.0,
                        color_desc_gray,
                    );
                    desc_y -= desc_line_h;
                }
            }

            y -= row_h;
        }

        y -= 1.4; // gap between sections
    }

    // --- Note Box (Atenolol / multi-slot med) ---
    if has_multi_slot_med {
        y -= 1.4;
        let note_h: f32 = 7.8;
        add_filled_rect(&layer, margin, y, content_width, note_h, color_note_bg);
        add_text(
            &layer,
            &font_bold,
            "Note:",
            Mm(margin + 2.8),
            Mm(y - note_h + 2.5),
            9.5,
            color_note_label,
        );
        add_text(
            &layer,
            &font,
            "Atenolol total = 200 mg/day (4 \u{00d7} 50 mg). Each dose is half of a scored 100 mg tablet. Imprint varies by manufacturer/refill.",
            Mm(margin + 14.8),
            Mm(y - note_h + 2.5),
            9.5,
            color_text_body,
        );
        y -= note_h;
    }

    // --- Tip Box ---
    y -= 2.8;
    let tip_h: f32 = 7.8;
    add_filled_rect(&layer, margin, y, content_width, tip_h, color_tip_bg);
    add_text(
        &layer,
        &font_bold,
        "Tip:",
        Mm(margin + 2.8),
        Mm(y - tip_h + 2.5),
        9.5,
        color_tip_label,
    );
    add_text(
        &layer,
        &font,
        "Generic pill imprints change between refills as pharmacy suppliers rotate. Verify markings on your current bottles if unsure.",
        Mm(margin + 11.3),
        Mm(y - tip_h + 2.5),
        9.5,
        color_text_body,
    );

    // --- Footer ---
    let now = chrono::Local::now().format("%B %d, %Y").to_string();
    let footer_text = format!(
        "Generated {}  \u{2022}  Review with your physician before making changes",
        now
    );
    add_text(
        &layer,
        &font_italic,
        &footer_text,
        Mm(page_width / 2.0 - 55.0),
        Mm(margin - 2.1),
        9.0,
        color_footer,
    );

    // Save PDF
    let path = Path::new(output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(path).map_err(|e| e.to_string())?;
    let writer = BufWriter::new(file);
    doc.save(&mut std::io::BufWriter::new(writer))
        .map_err(|e| e.to_string())?;

    Ok(GeneratePillOrganizerReportResponse {
        success: true,
        file_path: output_path.to_string(),
        medication_count: pill_meds.len(),
        message: format!(
            "Pill organizer report ({} medications, {} entries across {} time slots) saved to {}",
            pill_meds.len(),
            total_entries,
            slot_entries.iter().filter(|s| !s.is_empty()).count(),
            output_path
        ),
    })
}

/// Format dosage amount: show as integer if whole number, otherwise 1 decimal
fn format_dosage_amount(amount: f64) -> String {
    if (amount - amount.round()).abs() < 0.001 {
        format!("{:.0}", amount)
    } else {
        format!("{:.1}", amount)
    }
}
