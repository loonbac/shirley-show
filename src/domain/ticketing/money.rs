use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Money {
    cents: i64,
}

impl Money {
    pub const fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    pub fn from_soles(soles: f64) -> Result<Self, AppError> {
        if soles.is_nan() || soles.is_infinite() {
            return Err(AppError::validation(
                "invalid_amount",
                "Monto debe ser un número finito",
                "monto",
            ));
        }
        if soles < 0.0 {
            return Err(AppError::validation(
                "negative_amount",
                "Monto no puede ser negativo",
                "monto",
            ));
        }
        let rounded = (soles * 100.0).round();
        if (soles * 100.0 - rounded).abs() > 1e-4 {
            return Err(AppError::validation(
                "too_many_decimals",
                "Monto no puede tener más de 2 decimales",
                "monto",
            ));
        }
        Ok(Self {
            cents: rounded as i64,
        })
    }

    pub const fn cents(&self) -> i64 {
        self.cents
    }

    pub fn to_soles(&self) -> f64 {
        self.cents as f64 / 100.0
    }

    pub const fn zero() -> Self {
        Self { cents: 0 }
    }

    pub const fn add(&self, other: &Self) -> Self {
        Self {
            cents: self.cents + other.cents,
        }
    }

    pub const fn sub_signed(&self, other: &Self) -> i64 {
        self.cents - other.cents
    }
}
