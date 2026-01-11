//! Shared nutrition data structure
//!
//! Used across food items, recipes, meal entries, and days.

use serde::{Deserialize, Serialize};

/// Nutritional information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Nutrition {
    pub calories: f64,
    pub protein: f64,      // grams
    pub carbs: f64,        // grams
    pub fat: f64,          // grams
    pub fiber: f64,        // grams
    pub sodium: f64,       // milligrams
    pub sugar: f64,        // grams
    pub saturated_fat: f64, // grams
    pub cholesterol: f64,  // milligrams
}

impl Nutrition {
    /// Create a new Nutrition with all zeros
    pub fn zero() -> Self {
        Self::default()
    }

    /// Scale nutrition values by a multiplier
    pub fn scale(&self, multiplier: f64) -> Self {
        Self {
            calories: self.calories * multiplier,
            protein: self.protein * multiplier,
            carbs: self.carbs * multiplier,
            fat: self.fat * multiplier,
            fiber: self.fiber * multiplier,
            sodium: self.sodium * multiplier,
            sugar: self.sugar * multiplier,
            saturated_fat: self.saturated_fat * multiplier,
            cholesterol: self.cholesterol * multiplier,
        }
    }

    /// Add another nutrition to this one
    pub fn add(&self, other: &Nutrition) -> Self {
        Self {
            calories: self.calories + other.calories,
            protein: self.protein + other.protein,
            carbs: self.carbs + other.carbs,
            fat: self.fat + other.fat,
            fiber: self.fiber + other.fiber,
            sodium: self.sodium + other.sodium,
            sugar: self.sugar + other.sugar,
            saturated_fat: self.saturated_fat + other.saturated_fat,
            cholesterol: self.cholesterol + other.cholesterol,
        }
    }
}

impl std::ops::Add for Nutrition {
    type Output = Nutrition;

    fn add(self, other: Nutrition) -> Nutrition {
        Nutrition::add(&self, &other)
    }
}

impl std::ops::Mul<f64> for Nutrition {
    type Output = Nutrition;

    fn mul(self, multiplier: f64) -> Nutrition {
        self.scale(multiplier)
    }
}

impl std::iter::Sum for Nutrition {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Nutrition::zero(), |acc, n| acc + n)
    }
}
