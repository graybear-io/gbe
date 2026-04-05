//! Compact content rendering for diagnostics and traces.
//!
//! Not a mediatronic — just a plain text renderer for TypedContent,
//! used by pipeline traces and debug output.

use crate::content::TypedContent;

/// Render typed content to a single compact string.
pub fn render_compact(content: &TypedContent) -> String {
    match content {
        TypedContent::Text(s) => s.clone(),
        TypedContent::Number { value, unit } => {
            if let Some(u) = unit {
                format!("{value}{u}")
            } else if *value == (*value as i64) as f64 {
                format!("{}", *value as i64)
            } else {
                format!("{value:.1}")
            }
        }
        TypedContent::Timestamp(ms) => {
            let secs = ms / 1000;
            let h = (secs / 3600) % 24;
            let m = (secs / 60) % 60;
            let s = secs % 60;
            format!("{h:02}:{m:02}:{s:02}.{:03}", ms % 1000)
        }
        TypedContent::Pair { label, value } => {
            format!("{}: {}", label, render_compact(value))
        }
        TypedContent::List(items) => items
            .iter()
            .map(render_compact)
            .collect::<Vec<_>>()
            .join(", "),
    }
}
