use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Mode for vocal separation pipelines.
///
/// Strictly serialized to and deserialized from JSON as `"chorus"` and `"instrumental"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VocalSeparationMode {
    #[serde(rename = "chorus")]
    Coros,
    #[serde(rename = "instrumental")]
    Instrumental,
}

impl VocalSeparationMode {
    /// Returns the canonical public string identifier for this separation mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Coros => "chorus",
            Self::Instrumental => "instrumental",
        }
    }

    /// Returns the human-facing title suffix appended to separated tracks.
    pub fn title_suffix(&self) -> &'static str {
        match self {
            Self::Coros => " (Coros)",
            Self::Instrumental => " (Instrumental)",
        }
    }

    /// Strict parser rejecting any aliases or deviations from `"chorus"` and `"instrumental"`.
    pub fn from_str_strict(s: &str) -> Option<Self> {
        match s {
            "chorus" => Some(Self::Coros),
            "instrumental" => Some(Self::Instrumental),
            _ => None,
        }
    }
}

impl fmt::Display for VocalSeparationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for VocalSeparationMode {
    type Err = VocalSeparationModeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_strict(s).ok_or(VocalSeparationModeParseError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid vocal separation mode: expected 'chorus' or 'instrumental'")]
pub struct VocalSeparationModeParseError;
