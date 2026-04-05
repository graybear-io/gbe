//! Triage completed imprint — matches akasha.triage.completed events.
//!
//! Shape: { key: string, url: string, title?: string, promoted: integer, skipped: integer }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct TriageCompletedImprint;

impl Imprint for TriageCompletedImprint {
    fn name(&self) -> &str {
        "triage-completed"
    }

    fn matches(&self, subject: &str, data: &serde_json::Value) -> bool {
        subject.contains("triage.completed")
            && data.get("key").is_some()
            && data.get("promoted").is_some()
    }

    fn specificity(&self) -> u32 {
        20
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let key = data.get("key").and_then(|k| k.as_str()).unwrap_or("?");
        let url = data.get("url").and_then(|u| u.as_str()).unwrap_or("?");
        let title = data.get("title").and_then(|t| t.as_str());
        let promoted = data.get("promoted").and_then(|p| p.as_u64()).unwrap_or(0);
        let skipped = data.get("skipped").and_then(|s| s.as_u64()).unwrap_or(0);

        let display = title.unwrap_or(url);

        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("triaged {display}")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        let summary = Cell::new(
            Role::Summary,
            TypedContent::list(vec![
                TypedContent::pair("promoted", TypedContent::number(promoted as f64)),
                TypedContent::pair("skipped", TypedContent::number(skipped as f64)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        let detail = Cell::new(
            Role::Detail,
            TypedContent::list(vec![
                TypedContent::pair("key", TypedContent::text(key)),
                TypedContent::pair("url", TypedContent::text(url)),
                TypedContent::pair("promoted", TypedContent::number(promoted as f64)),
                TypedContent::pair("skipped", TypedContent::number(skipped as f64)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        heading = heading
            .link_to(summary.id, LinkKind::SummaryOf)
            .link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, summary, detail], "triage-completed")
            .with_source(subject)
    }
}
