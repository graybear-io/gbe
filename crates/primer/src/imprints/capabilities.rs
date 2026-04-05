//! Capabilities imprint — matches lifecycle capability announcements.
//!
//! Shape: { capabilities: [{ name, description, params }], node: { name, domain } }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct CapabilitiesImprint;

impl Imprint for CapabilitiesImprint {
    fn name(&self) -> &str {
        "capabilities"
    }

    fn matches(&self, _subject: &str, data: &serde_json::Value) -> bool {
        data.get("capabilities").and_then(|c| c.as_array()).is_some()
            && data.get("node").and_then(|n| n.get("name")).is_some()
    }

    fn specificity(&self) -> u32 {
        20
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

        let caps = data
            .get("capabilities")
            .and_then(|c| c.as_array())
            .expect("matches guarantees capabilities exist");

        // Heading: "herald registered 1 capability"
        let cap_word = if caps.len() == 1 { "capability" } else { "capabilities" };
        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("{node} registered {} {cap_word}", caps.len())),
        )
        .with_priority(10);
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

        // One detail cell per capability, linked in sequence.
        let mut cap_cells: Vec<Cell> = Vec::new();

        for cap in caps {
            let name = cap
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("?");

            let description = cap
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            let authority = cap
                .get("authority_required")
                .and_then(|a| a.as_str())
                .unwrap_or("?");

            let mut items = vec![
                TypedContent::pair("name", TypedContent::text(name)),
                TypedContent::pair("requires", TypedContent::text(authority)),
            ];

            if !description.is_empty() {
                items.push(TypedContent::pair("description", TypedContent::text(description)));
            }

            // Include params if present.
            if let Some(params) = cap.get("params").and_then(|p| p.as_array()) {
                let param_items: Vec<TypedContent> = params
                    .iter()
                    .filter_map(|p| {
                        let pname = p.get("name").and_then(|n| n.as_str())?;
                        let kind = p.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
                        let required = p.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
                        let req_marker = if required { " (required)" } else { "" };
                        Some(TypedContent::text(format!("{pname}: {kind}{req_marker}")))
                    })
                    .collect();

                if !param_items.is_empty() {
                    items.push(TypedContent::pair("params", TypedContent::list(param_items)));
                }
            }

            let mut cell = Cell::new(Role::Detail, TypedContent::list(items))
                .link_to(heading_id, LinkKind::DetailOf);

            // Link capabilities in sequence.
            if let Some(prev) = cap_cells.last() {
                cell = cell.link_to(prev.id, LinkKind::Sequence);
            }

            cap_cells.push(cell);
        }

        // Link heading to children.
        heading = heading.link_to(summary.id, LinkKind::SummaryOf);
        for c in &cap_cells {
            heading = heading.link_to(c.id, LinkKind::SummaryOf);
        }

        let mut cells = vec![heading, summary];
        cells.extend(cap_cells);

        Page::new(cells, "capabilities")
            .with_source(subject)
    }
}
