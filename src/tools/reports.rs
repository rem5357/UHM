//! Report generation tools
//!
//! Generate PDF reports for blood pressure and heart rate data with charts and statistics.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use chrono::{Datelike, NaiveDate, Weekday};
use printpdf::*;
use printpdf::image_crate::{DynamicImage, RgbImage, ImageFormat};
use serde::Serialize;

use crate::db::Database;
use crate::models::{PatientInfo, Vital, VitalType};

// ============================================================================
// Color Constants (RGB 0-255)
// ============================================================================

const COLOR_BP_TITLE: (u8, u8, u8) = (192, 0, 0);       // Red for BP title
const COLOR_HR_TITLE: (u8, u8, u8) = (112, 48, 160);    // Purple for HR title
const COLOR_NORMAL: (u8, u8, u8) = (0, 176, 80);        // Green
const COLOR_ELEVATED: (u8, u8, u8) = (255, 165, 0);     // Orange
const COLOR_HIGH: (u8, u8, u8) = (255, 0, 0);           // Red
const COLOR_BRADYCARDIA: (u8, u8, u8) = (0, 112, 192);  // Blue
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

        chart.configure_mesh()
            .x_labels(daily_stats.len().min(10))
            .x_label_formatter(&|x| {
                if *x >= 0 && (*x as usize) < daily_stats.len() {
                    let date = &daily_stats[*x as usize].date;
                    date.split('-').skip(1).collect::<Vec<_>>().join("/")
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

        chart.draw_series(systolic_points.iter().map(|(x, y)| {
            Circle::new((*x, *y), 3, RED.filled())
        })).map_err(|e| e.to_string())?;

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

        chart.draw_series(diastolic_points.iter().map(|(x, y)| {
            Circle::new((*x, *y), 3, BLUE.filled())
        })).map_err(|e| e.to_string())?;

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

        chart.configure_mesh()
            .x_labels(daily_stats.len().min(10))
            .x_label_formatter(&|x| {
                if *x >= 0 && (*x as usize) < daily_stats.len() {
                    let date = &daily_stats[*x as usize].date;
                    date.split('-').skip(1).collect::<Vec<_>>().join("/")
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

        chart.draw_series(hr_points.iter().map(|(x, y)| {
            Circle::new((*x, *y), 4, RGBColor(112, 48, 160).filled())
        })).map_err(|e| e.to_string())?;

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
