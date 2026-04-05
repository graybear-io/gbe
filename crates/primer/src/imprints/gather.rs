//! Gather completed/failed imprints — matches akasha.gathered.* events.
//!
//! Completed shape: { key: string, url: string, title?: string, status: string }
//! Failed shape: { key: string, url: string, error: string }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct GatherCompletedImprint;

impl Imprint for GatherCompletedImprint {
    fn name(&self) -> &str {
        "gather-completed"
    }

    fn matches(&self, subject: &str, data: &serde_json::Value) -> bool {
        subject.contains("gathered.completed")
            && data.get("key").is_some()
            && data.get("url").is_some()
    }

    fn specificity(&self) -> u32 {
        20
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let key = data.get("key").and_then(|k| k.as_str()).unwrap_or("?");
        let url = data.get("url").and_then(|u| u.as_str()).unwrap_or("?");
        let title = data.get("title").and_then(|t| t.as_str());
        let status = data.get("status").and_then(|s| s.as_str()).unwrap_or("completed");

        let display = title.unwrap_or(url);

        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("gathered {display}")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        let status_cell = Cell::new(
            Role::Status,
            TypedContent::text(status),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        let detail = Cell::new(
            Role::Detail,
            TypedContent::list(vec![
                TypedContent::pair("key", TypedContent::text(key)),
                TypedContent::pair("url", TypedContent::text(url)),
                TypedContent::pair("status", TypedContent::text(status)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        heading = heading
            .link_to(status_cell.id, LinkKind::SummaryOf)
            .link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, status_cell, detail], "gather-completed")
            .with_source(subject)
    }
}

pub struct GatherFailedImprint;

impl Imprint for GatherFailedImprint {
    fn name(&self) -> &str {
        "gather-failed"
    }

    fn matches(&self, subject: &str, data: &serde_json::Value) -> bool {
        subject.contains("gathered.failed")
            && data.get("key").is_some()
            && data.get("error").is_some()
    }

    fn specificity(&self) -> u32 {
        20
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let key = data.get("key").and_then(|k| k.as_str()).unwrap_or("?");
        let url = data.get("url").and_then(|u| u.as_str()).unwrap_or("?");
        let error = data.get("error").and_then(|e| e.as_str()).unwrap_or("?");

        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("gather failed {url}")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        let status_cell = Cell::new(
            Role::Status,
            TypedContent::text("error"),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        let detail = Cell::new(
            Role::Detail,
            TypedContent::list(vec![
                TypedContent::pair("key", TypedContent::text(key)),
                TypedContent::pair("url", TypedContent::text(url)),
                TypedContent::pair("error", TypedContent::text(error)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        heading = heading
            .link_to(status_cell.id, LinkKind::SummaryOf)
            .link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, status_cell, detail], "gather-failed")
            .with_source(subject)
    }
}
