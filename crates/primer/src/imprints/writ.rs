//! Writ imprint — matches bus messages that are writs.
//!
//! Shape: { capability: string, authority: { level, scope, issuer }, target: ... }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct WritImprint;

impl Imprint for WritImprint {
    fn name(&self) -> &str {
        "writ"
    }

    fn matches(&self, _subject: &str, data: &serde_json::Value) -> bool {
        data.get("capability").is_some() && data.get("authority").is_some()
    }

    fn specificity(&self) -> u32 {
        20
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let capability = data
            .get("capability")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        // WritTarget serializes with snake_case: {"node": {...}} or {"domain": "..."}
        let target_name = data
            .get("target")
            .and_then(|t| {
                // Try snake_case (from frame's serde config)
                t.get("node")
                    .and_then(|n| n.get("name"))
                    .and_then(|n| n.as_str())
                    .or_else(|| t.get("domain").and_then(|d| d.as_str()))
                    // Also try PascalCase (in case of older serialization)
                    .or_else(|| {
                        t.get("Node")
                            .and_then(|n| n.get("name"))
                            .and_then(|n| n.as_str())
                    })
                    .or_else(|| t.get("Domain").and_then(|d| d.as_str()))
            })
            .unwrap_or("?");

        let level = data
            .get("authority")
            .and_then(|a| a.get("level"))
            .and_then(|l| l.as_str())
            .unwrap_or("?");

        let scope = data
            .get("authority")
            .and_then(|a| a.get("scope"))
            .and_then(|s| s.as_str())
            .unwrap_or("*");

        let issuer = data
            .get("authority")
            .and_then(|a| a.get("issuer"))
            .and_then(|i| i.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("?");

        // Heading: "writ gather → thalamus"
        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("writ {capability} → {target_name}")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        // Summary: authority level
        let summary = Cell::new(
            Role::Summary,
            TypedContent::list(vec![
                TypedContent::pair("authority", TypedContent::text(level)),
                TypedContent::pair("issuer", TypedContent::text(issuer)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        // Detail: full writ info
        let mut detail_items = vec![
            TypedContent::pair("capability", TypedContent::text(capability)),
            TypedContent::pair("target", TypedContent::text(target_name)),
            TypedContent::pair("authority", TypedContent::text(level)),
            TypedContent::pair("scope", TypedContent::text(scope)),
            TypedContent::pair("issuer", TypedContent::text(issuer)),
            TypedContent::pair("subject", TypedContent::text(subject)),
        ];

        // Include params if present.
        if let Some(params) = data.get("params") {
            if let Ok(pretty) = serde_json::to_string(params) {
                detail_items.push(TypedContent::pair("params", TypedContent::text(pretty)));
            }
        }

        let detail = Cell::new(Role::Detail, TypedContent::list(detail_items))
            .link_to(heading_id, LinkKind::DetailOf);

        heading = heading
            .link_to(summary.id, LinkKind::SummaryOf)
            .link_to(detail.id, LinkKind::SummaryOf);

        Page::new(vec![heading, summary, detail], "writ")
            .with_source(subject)
    }
}
