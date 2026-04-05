//! WritResponse imprint — matches bus messages that are writ responses.
//!
//! Shape: { writ_id: string, status: string, responder: { name, domain }, data: ... }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct WritResponseImprint;

impl Imprint for WritResponseImprint {
    fn name(&self) -> &str {
        "writ-response"
    }

    fn matches(&self, _subject: &str, data: &serde_json::Value) -> bool {
        data.get("writ_id").is_some() && data.get("status").is_some()
    }

    fn specificity(&self) -> u32 {
        20
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let status = data
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        let responder = data
            .get("responder")
            .and_then(|r| r.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("?");

        let responder_domain = data
            .get("responder")
            .and_then(|r| r.get("domain"))
            .and_then(|d| d.as_str())
            .unwrap_or("?");

        let writ_id = data
            .get("writ_id")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        // Heading: "response Ok from thalamus"
        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("response {status} from {responder}")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        // Status cell.
        let status_cell = Cell::new(
            Role::Status,
            TypedContent::text(status),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        // Detail: full response info.
        let mut detail_items = vec![
            TypedContent::pair("status", TypedContent::text(status)),
            TypedContent::pair("writ_id", TypedContent::text(writ_id)),
            TypedContent::pair("responder", TypedContent::text(format!("{responder} ({responder_domain})"))),
            TypedContent::pair("subject", TypedContent::text(subject)),
        ];

        // Include response data if present.
        if let Some(resp_data) = data.get("data") {
            if let Ok(pretty) = serde_json::to_string_pretty(resp_data) {
                detail_items.push(TypedContent::pair("data", TypedContent::text(pretty)));
            }
        }

        let detail = Cell::new(Role::Detail, TypedContent::list(detail_items))
            .link_to(heading_id, LinkKind::DetailOf);

        heading = heading
            .link_to(status_cell.id, LinkKind::SummaryOf)
            .link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, status_cell, detail], "writ-response")
            .with_source(subject)
    }
}
