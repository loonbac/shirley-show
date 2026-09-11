use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveDirection {
    Up,
    Down,
}

impl FromStr for MoveDirection {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "up" => Ok(MoveDirection::Up),
            "down" => Ok(MoveDirection::Down),
            _ => Err(AppError::validation(
                "invalid_direction",
                "Direction must be 'up' or 'down'",
                "direction",
            )),
        }
    }
}
