//! Typed content — structured data that consumers render according to their capabilities.
//!
//! A cell carries typed content, not pre-rendered strings. A 20-column OLED
//! and an 80-column terminal render the same content differently because
//! they know their own constraints.

use serde::{Deserialize, Serialize};

/// Structured content within a cell.
///
/// The consumer (mediatronic) decides how to render each variant
/// based on its capabilities and layout imprint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum TypedContent {
    /// Plain text.
    Text(String),

    /// A numeric value with an optional unit.
    Number {
        value: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
    },

    /// An ordered list of content items.
    List(Vec<TypedContent>),

    /// A labeled value — "pages: 42", "duration: 1200ms".
    Pair {
        label: String,
        value: Box<TypedContent>,
    },

    /// A timestamp in milliseconds since Unix epoch.
    Timestamp(u64),
}

impl TypedContent {
    /// Create a text content.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    /// Create a number without a unit.
    pub fn number(value: f64) -> Self {
        Self::Number { value, unit: None }
    }

    /// Create a number with a unit.
    pub fn number_with_unit(value: f64, unit: impl Into<String>) -> Self {
        Self::Number {
            value,
            unit: Some(unit.into()),
        }
    }

    /// Create a labeled pair.
    pub fn pair(label: impl Into<String>, value: TypedContent) -> Self {
        Self::Pair {
            label: label.into(),
            value: Box::new(value),
        }
    }

    /// Create a list.
    pub fn list(items: Vec<TypedContent>) -> Self {
        Self::List(items)
    }

    /// Create a timestamp.
    pub fn timestamp(ms: u64) -> Self {
        Self::Timestamp(ms)
    }
}
