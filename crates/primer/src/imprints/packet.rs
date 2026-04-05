//! Packet imprint — matches bus messages that are packets with frame stacks.
//!
//! Shape: { frames: [{ kind, node: { name, domain }, timestamp, metadata }], payload: string }

use crate::cell::{Cell, LinkKind, Role};
use crate::content::TypedContent;
use crate::imprint::Imprint;
use crate::page::Page;

pub struct PacketImprint;

impl Imprint for PacketImprint {
    fn name(&self) -> &str {
        "packet"
    }

    fn matches(&self, _subject: &str, data: &serde_json::Value) -> bool {
        data.get("frames")
            .and_then(|f| f.as_array())
            .is_some_and(|a| !a.is_empty())
    }

    fn specificity(&self) -> u32 {
        20
    }

    fn compile(&self, subject: &str, data: &serde_json::Value) -> Page {
        let frames = data
            .get("frames")
            .and_then(|f| f.as_array())
            .expect("matches guarantees frames exist");

        let origin_node = frames
            .first()
            .and_then(|f| f.get("node"))
            .and_then(|n| n.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("?");

        let frame_count = frames.len();

        // Heading: "packet from thalamus [3 frames]"
        let mut heading = Cell::new(
            Role::Heading,
            TypedContent::text(format!("packet from {origin_node} [{frame_count} frames]")),
        )
        .with_priority(10);
        let heading_id = heading.id;

        // Summary: origin + frame count.
        let summary = Cell::new(
            Role::Summary,
            TypedContent::list(vec![
                TypedContent::pair("origin", TypedContent::text(origin_node)),
                TypedContent::pair("frames", TypedContent::number(frame_count as f64)),
            ]),
        )
        .link_to(heading_id, LinkKind::DetailOf);

        // One detail cell per frame in the stack — linked as a sequence.
        let mut frame_cells: Vec<Cell> = Vec::new();

        for (i, frame) in frames.iter().enumerate() {
            let kind = frame
                .get("kind")
                .and_then(|k| {
                    // kind can be a string ("origin") or an object ({"barrier": {...}})
                    k.as_str()
                        .map(String::from)
                        .or_else(|| {
                            k.as_object().and_then(|obj| {
                                obj.keys().next().map(|key| key.clone())
                            })
                        })
                })
                .unwrap_or_else(|| "?".to_string());

            let node = frame
                .get("node")
                .and_then(|n| n.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("?");

            let domain = frame
                .get("node")
                .and_then(|n| n.get("domain"))
                .and_then(|d| d.as_str())
                .unwrap_or("?");

            let ts = frame
                .get("timestamp")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);

            let mut items = vec![
                TypedContent::pair("kind", TypedContent::text(&kind)),
                TypedContent::pair("node", TypedContent::text(format!("{node} ({domain})"))),
                TypedContent::pair("time", TypedContent::timestamp(ts)),
            ];

            // Include metadata keys.
            if let Some(meta) = frame.get("metadata").and_then(|m| m.as_object()) {
                for (key, val) in meta {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    items.push(TypedContent::pair(key.as_str(), TypedContent::text(val_str)));
                }
            }

            let mut cell = Cell::new(
                Role::Detail,
                TypedContent::list(items),
            )
            .with_priority(5 - i as i32)
            .link_to(heading_id, LinkKind::DetailOf);

            // Link frames in sequence.
            if let Some(prev) = frame_cells.last() {
                cell = cell.link_to(prev.id, LinkKind::Sequence);
            }

            frame_cells.push(cell);
        }

        // Link heading to its children.
        heading = heading.link_to(summary.id, LinkKind::SummaryOf);
        for fc in &frame_cells {
            heading = heading.link_to(fc.id, LinkKind::SummaryOf);
        }

        let mut cells = vec![heading, summary];
        cells.extend(frame_cells);

        Page::new(cells, "packet")
            .with_source(subject)
    }
}
