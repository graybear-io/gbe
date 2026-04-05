use primer::imprints::default_registry;
use primer::Role;

#[test]
fn writ_imprint_matches_and_compiles() {
    let registry = default_registry();

    let data = serde_json::json!({
        "capability": "gather",
        "authority": {
            "level": "Pilgrim",
            "scope": "*",
            "issuer": {
                "name": "bear",
                "domain": "allthing"
            }
        },
        "target": {
            "Node": {
                "name": "thalamus",
                "domain": "akasha"
            }
        },
        "params": {
            "key": "collect:example.com"
        }
    });

    let page = registry.compile("writs.thalamus", &data).unwrap();
    assert_eq!(page.index.content_type, "writ");
    assert_eq!(page.cells_by_role(Role::Heading).len(), 1);
    assert_eq!(page.cells_by_role(Role::Summary).len(), 1);
    assert_eq!(page.cells_by_role(Role::Detail).len(), 1);

    // Heading should mention capability and target.
    let heading = &page.cells_by_role(Role::Heading)[0];
    match &heading.content {
        primer::TypedContent::Text(s) => {
            assert!(s.contains("gather"), "heading should contain capability");
            assert!(s.contains("thalamus"), "heading should contain target");
        }
        _ => panic!("heading should be text"),
    }
}

#[test]
fn writ_response_imprint_matches() {
    let registry = default_registry();

    let data = serde_json::json!({
        "writ_id": "01HXYZ123",
        "status": "Ok",
        "responder": {
            "name": "thalamus",
            "domain": "akasha"
        },
        "data": {
            "gathered": 42
        }
    });

    let page = registry.compile("writs.responses", &data).unwrap();
    assert_eq!(page.index.content_type, "writ-response");

    let heading = &page.cells_by_role(Role::Heading)[0];
    match &heading.content {
        primer::TypedContent::Text(s) => {
            assert!(s.contains("Ok"));
            assert!(s.contains("thalamus"));
        }
        _ => panic!("heading should be text"),
    }
}

#[test]
fn packet_imprint_matches_frames() {
    let registry = default_registry();

    let data = serde_json::json!({
        "id": "01HXYZ456",
        "payload": "aGVsbG8=",
        "frames": [
            {
                "kind": "origin",
                "node": { "name": "herald", "domain": "allthing" },
                "timestamp": 1711584000000_u64,
                "metadata": {}
            },
            {
                "kind": { "barrier": { "from_domain": "allthing", "to_domain": "akasha" } },
                "node": { "name": "thalamus", "domain": "akasha" },
                "timestamp": 1711584001000_u64,
                "metadata": { "discord_message_id": "123456" }
            }
        ]
    });

    let page = registry.compile("akasha.collected", &data).unwrap();
    assert_eq!(page.index.content_type, "packet");

    // Should have: heading, summary, 2 detail cells (one per frame).
    assert_eq!(page.cells_by_role(Role::Heading).len(), 1);
    assert_eq!(page.cells_by_role(Role::Summary).len(), 1);
    assert_eq!(page.cells_by_role(Role::Detail).len(), 2);

    // Detail cells should be linked in sequence.
    let details = page.cells_by_role(Role::Detail);
    let second_detail = &details[1];
    let has_sequence_link = second_detail.links.iter().any(|l| {
        l.kind == primer::cell::LinkKind::Sequence && l.target == details[0].id
    });
    assert!(has_sequence_link, "frame cells should be linked in sequence");
}

#[test]
fn lifecycle_event_matches_envelope() {
    let registry = default_registry();

    let data = serde_json::json!({
        "node": {
            "name": "sentinel-01",
            "domain": "gbe",
            "kind": "Service"
        },
        "uptime_ms": 42000
    });

    let page = registry.compile("lifecycle.sentinel.heartbeat", &data).unwrap();
    assert_eq!(page.index.content_type, "lifecycle");

    let heading = &page.cells_by_role(Role::Heading)[0];
    match &heading.content {
        primer::TypedContent::Text(s) => {
            assert!(s.contains("sentinel-01"));
            assert!(s.contains("heartbeat"));
        }
        _ => panic!("heading should be text"),
    }
}

#[test]
fn unknown_data_falls_through_to_generic() {
    let registry = default_registry();

    let data = serde_json::json!({
        "something": "unexpected",
        "count": 7
    });

    let page = registry.compile("some.unknown.subject", &data).unwrap();
    assert_eq!(page.index.content_type, "generic");
    assert_eq!(page.cells_by_role(Role::Heading).len(), 1);
    assert_eq!(page.cells_by_role(Role::Detail).len(), 1);
}

#[test]
fn specificity_ordering_writ_over_envelope() {
    // A writ also has a "node"-like structure in authority.issuer,
    // but the writ imprint should win because it's more specific.
    let registry = default_registry();

    let data = serde_json::json!({
        "capability": "drain-host",
        "authority": {
            "level": "Consul",
            "scope": "*",
            "issuer": {
                "name": "bear",
                "domain": "allthing"
            }
        },
        "target": {
            "Node": {
                "name": "sentinel-01",
                "domain": "gbe"
            }
        },
        "node": {
            "name": "overseer",
            "domain": "gbe"
        }
    });

    let page = registry.compile("writs.sentinel", &data).unwrap();
    // Writ imprint should win over envelope imprint.
    assert_eq!(page.index.content_type, "writ");
}

#[test]
fn feed_with_compiled_pages() {
    use primer::Feed;

    let registry = default_registry();
    let mut feed = Feed::new(1000);

    // Simulate a stream of bus events arriving.
    let events = vec![
        ("lifecycle.sentinel.started", serde_json::json!({
            "node": { "name": "sentinel-01", "domain": "gbe" }
        })),
        ("writs.thalamus", serde_json::json!({
            "capability": "gather",
            "authority": { "level": "Pilgrim", "scope": "*", "issuer": { "name": "bear", "domain": "allthing" } },
            "target": { "Node": { "name": "thalamus", "domain": "akasha" } }
        })),
        ("writs.responses", serde_json::json!({
            "writ_id": "01ABC", "status": "Ok",
            "responder": { "name": "thalamus", "domain": "akasha" }
        })),
    ];

    for (subject, data) in &events {
        if let Some(page) = registry.compile(subject, data) {
            feed.push(page);
        }
    }

    assert_eq!(feed.len(), 3);

    // Content types should reflect what was compiled.
    let types: Vec<&str> = feed.pages().iter().map(|p| p.index.content_type.as_str()).collect();
    assert_eq!(types, vec!["lifecycle", "writ", "writ-response"]);

    // Navigate: should start at the end (follow mode).
    assert_eq!(feed.cursor_position(), Some(2));
    assert_eq!(feed.current().unwrap().index.content_type, "writ-response");

    // Go back.
    feed.prev();
    assert_eq!(feed.current().unwrap().index.content_type, "writ");

    feed.prev();
    assert_eq!(feed.current().unwrap().index.content_type, "lifecycle");
}
