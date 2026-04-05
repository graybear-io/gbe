//! Collect URL imprint — matches akasha.collect.url events.
//!
//! Shape: { url: string, title?: string, tags: [string], gather: bool }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct CollectImprint;

impl Imprint for CollectImprint {
    fn name(&self) -> &str {
        "collect-url"
    }

    fn matches(&self, subject: &str, data: &serde_json::Value) -> bool {
        subject.contains("collect") && data.get("url").and_then(|u| u.as_str()).is_some()
    }

    fn specificity(&self) -> u32 {
        15
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let url = data
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("?");

        let title = data
            .get("title")
            .and_then(|t| t.as_str());

        let gather = data
            .get("gather")
            .and_then(|g| g.as_bool())
            .unwrap_or(true);

        let tags = data
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let action = if gather { "collect+gather" } else { "collect" };
        let display = title.unwrap_or(url);

        // Heading: "collect+gather example.com"
        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("{action} {display}")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        // Summary: URL + gather flag.
        let mut summary_items = vec![
            TypedContent::pair("url", TypedContent::text(url)),
        ];
        if !gather {
            summary_items.push(TypedContent::pair("gather", TypedContent::text("false")));
        }
        let summary = Cell::new(Role::Summary, TypedContent::list(summary_items))
            .link_to(heading_id, LinkKind::DetailOf);

        // Detail: tags if present.
        let mut cells = vec![];
        if !tags.is_empty() {
            let tag_items: Vec<TypedContent> = tags.iter().map(|t| TypedContent::text(t)).collect();
            let detail = Cell::new(
                Role::Detail,
                TypedContent::pair("tags", TypedContent::list(tag_items)),
            )
            .link_to(heading_id, LinkKind::DetailOf);
            cells.push(detail);
        }

        heading = heading.link_to(summary.id, LinkKind::SummaryOf);
        for c in &cells {
            heading = heading.link_to(c.id, LinkKind::SummaryOf);
        }

        let mut all_cells = vec![heading, summary];
        all_cells.extend(cells);

        Page::new(all_cells, "collect-url")
            .with_source(subject)
    }
}
