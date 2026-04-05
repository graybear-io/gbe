//! Generic imprint — catch-all for any data that didn't match a
//! more specific imprint.
//!
//! Produces a heading from the subject and a detail cell with the full JSON.

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct GenericImprint;

impl Imprint for GenericImprint {
    fn name(&self) -> &str {
        "generic"
    }

    fn matches(&self, _subject: &str, _data: &serde_json::Value) -> bool {
        true // Catch-all.
    }

    fn specificity(&self) -> u32 {
        0
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let event_type = subject.rsplit('.').next().unwrap_or("event");

        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(event_type),
        )
        .with_priority(1);
        let heading_id = heading.id;

        let detail_text = serde_json::to_string_pretty(data).unwrap_or_default();
        let detail = Cell::new(Role::Detail, TypedContent::text(detail_text))
            .link_to(heading_id, LinkKind::DetailOf);

        heading = heading.link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, detail], "generic")
            .with_source(subject)
    }
}
