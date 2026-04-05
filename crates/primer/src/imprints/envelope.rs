//! Envelope imprint — matches DomainPayload-wrapped messages that didn't
//! match a more specific imprint.
//!
//! Shape: has a "node" field with { name, domain } — typical lifecycle events.

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct EnvelopeImprint;

impl Imprint for EnvelopeImprint {
    fn name(&self) -> &str {
        "envelope"
    }

    fn matches(&self, _subject: &str, data: &serde_json::Value) -> bool {
        // Matches anything with a node identity — lifecycle events, etc.
        data.get("node")
            .and_then(|n| n.get("name"))
            .is_some()
    }

    fn specificity(&self) -> u32 {
        5
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let node = data
            .get("node")
            .and_then(|n| n.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("?");

        let domain = data
            .get("node")
            .and_then(|n| n.get("domain"))
            .and_then(|d| d.as_str())
            .unwrap_or("?");

        let event_type = subject.rsplit('.').next().unwrap_or("?");

        // Heading: "sentinel-01 heartbeat"
        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("{node} {event_type}")),
        )
        .with_priority(5);
        let heading_id = heading.id;

        // Summary: node + domain.
        let summary = Cell::new(
            Role::Summary,
            TypedContent::list(vec![
                TypedContent::pair("node", TypedContent::text(node)),
                TypedContent::pair("domain", TypedContent::text(domain)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        // Detail: full JSON.
        let detail_text = serde_json::to_string_pretty(data).unwrap_or_default();
        let detail = Cell::new(Role::Detail, TypedContent::text(detail_text))
            .link_to(heading_id, LinkKind::DetailOf);

        heading = heading
            .link_to(summary.id, LinkKind::SummaryOf)
            .link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, summary, detail], "lifecycle")
            .with_source(subject)
    }
}
