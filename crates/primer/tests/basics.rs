use primer::*;

#[test]
fn build_a_page_from_cells() {
    // Simulate what a gather-result imprint would produce.
    let mut title = Cell::new(Role::Heading, TypedContent::text("example.com gather"));
    let title_id = title.id;

    let stats = Cell::new(
        Role::Summary,
        TypedContent::list(vec![
            TypedContent::pair("pages", TypedContent::number(42.0)),
            TypedContent::pair("duration", TypedContent::number_with_unit(1200.0, "ms")),
        ]),
    )
    .with_priority(5)
    .link_to(title_id, LinkKind::DetailOf);

    let discoveries = Cell::new(
        Role::Detail,
        TypedContent::list(vec![
            TypedContent::text("Found RSS feed"),
            TypedContent::text("Discovered sitemap.xml"),
        ]),
    )
    .link_to(title_id, LinkKind::DetailOf);

    // Title links back as summary-of its details.
    title = title
        .link_to(stats.id, LinkKind::SummaryOf)
        .link_to(discoveries.id, LinkKind::SummaryOf);

    let page = Page::new(vec![title, stats, discoveries], "gather-result")
        .with_source("akasha.>")
        .with_min_width(40);

    assert_eq!(page.cells.len(), 3);
    assert_eq!(page.index.content_type, "gather-result");
    assert_eq!(page.index.source.as_deref(), Some("akasha.>"));
    assert_eq!(page.cells_by_role(Role::Heading).len(), 1);
    assert_eq!(page.cells_by_role(Role::Detail).len(), 1);
}

#[test]
fn feed_sequence_navigation() {
    let mut feed = Feed::new(100);

    // Empty feed.
    assert_eq!(feed.next(), NavResult::Empty);
    assert_eq!(feed.prev(), NavResult::Empty);

    // Add some pages.
    feed.push(Page::new(vec![Cell::new(Role::Heading, TypedContent::text("event 1"))], "lifecycle"));
    feed.push(Page::new(vec![Cell::new(Role::Heading, TypedContent::text("event 2"))], "lifecycle"));
    feed.push(Page::new(vec![Cell::new(Role::Heading, TypedContent::text("event 3"))], "lifecycle"));

    // Follow mode: cursor is at the end.
    assert_eq!(feed.cursor_position(), Some(2));
    assert!(feed.is_following());

    // Navigate back.
    assert_eq!(feed.prev(), NavResult::Moved);
    assert_eq!(feed.cursor_position(), Some(1));
    assert!(!feed.is_following());

    // Navigate forward.
    assert_eq!(feed.next(), NavResult::Moved);
    assert_eq!(feed.cursor_position(), Some(2));

    // Top/bottom.
    assert_eq!(feed.top(), NavResult::Moved);
    assert_eq!(feed.cursor_position(), Some(0));
    assert_eq!(feed.top(), NavResult::AtBoundary);

    assert_eq!(feed.bottom(), NavResult::Moved);
    assert_eq!(feed.cursor_position(), Some(2));
    assert!(feed.is_following());
}

#[test]
fn feed_capacity_eviction() {
    let mut feed = Feed::new(3);

    for i in 0..5 {
        feed.push(Page::new(
            vec![Cell::new(Role::Heading, TypedContent::text(format!("event {i}")))],
            "lifecycle",
        ));
    }

    // Should have retained only the last 3.
    assert_eq!(feed.len(), 3);

    // Content should be events 2, 3, 4.
    let headings: Vec<_> = feed
        .pages()
        .iter()
        .map(|p| match &p.cells[0].content {
            TypedContent::Text(s) => s.as_str(),
            _ => panic!("expected text"),
        })
        .collect();
    assert_eq!(headings, vec!["event 2", "event 3", "event 4"]);
}

#[test]
fn cell_navigation_drill() {
    let mut heading = Cell::new(Role::Heading, TypedContent::text("deploy result"));
    let heading_id = heading.id;

    let detail = Cell::new(Role::Detail, TypedContent::text("full deployment log here"))
        .link_to(heading_id, LinkKind::DetailOf);
    let detail_id = detail.id;

    heading = heading.link_to(detail_id, LinkKind::SummaryOf);

    let page = Page::new(vec![heading, detail], "deploy");

    let mut nav = CellNavigator::new(&page);

    // Starts on heading.
    assert_eq!(nav.focused().unwrap().role, Role::Heading);

    // Drill down to detail.
    assert_eq!(nav.drill_down(), NavResult::Moved);
    assert_eq!(nav.focused().unwrap().role, Role::Detail);

    // Drill up back to heading.
    assert_eq!(nav.drill_up(), NavResult::Moved);
    assert_eq!(nav.focused().unwrap().role, Role::Heading);

    // Can't drill up further.
    assert_eq!(nav.drill_up(), NavResult::AtBoundary);
}

#[test]
fn imprint_registry_specificity_ordering() {
    use primer::imprint::ImprintRegistry;

    struct DefaultImprint;
    impl Imprint for DefaultImprint {
        fn name(&self) -> &str { "default" }
        fn matches(&self, _: &str, data: &serde_json::Value) -> bool {
            data.get("title").is_some()
        }
        fn specificity(&self) -> u32 { 0 }
        fn compile(&self, _: &str, data: &serde_json::Value) -> Page {
            let title = data["title"].as_str().unwrap_or("untitled");
            Page::new(vec![Cell::new(Role::Heading, TypedContent::text(title))], "generic")
        }
    }

    struct GatherImprint;
    impl Imprint for GatherImprint {
        fn name(&self) -> &str { "gather-result" }
        fn matches(&self, _: &str, data: &serde_json::Value) -> bool {
            data.get("title").is_some() && data.get("page_count").is_some()
        }
        fn specificity(&self) -> u32 { 10 }
        fn compile(&self, _: &str, data: &serde_json::Value) -> Page {
            let title = data["title"].as_str().unwrap_or("untitled");
            let count = data["page_count"].as_u64().unwrap_or(0);
            Page::new(
                vec![
                    Cell::new(Role::Heading, TypedContent::text(title)),
                    Cell::new(Role::Summary, TypedContent::pair("pages", TypedContent::number(count as f64))),
                ],
                "gather-result",
            )
        }
    }

    let mut registry = ImprintRegistry::new();
    // Register in wrong order — registry should sort by specificity.
    registry.register(Box::new(DefaultImprint));
    registry.register(Box::new(GatherImprint));

    // Data with both title and page_count: gather imprint should win.
    let data = serde_json::json!({ "title": "example.com", "page_count": 42 });
    let page = registry.compile("akasha.gathered.completed", &data).unwrap();
    assert_eq!(page.index.content_type, "gather-result");
    assert_eq!(page.cells.len(), 2);

    // Data with only title: default imprint should match.
    let data = serde_json::json!({ "title": "some event" });
    let page = registry.compile("lifecycle.started", &data).unwrap();
    assert_eq!(page.index.content_type, "generic");
    assert_eq!(page.cells.len(), 1);
}

#[test]
fn page_serializes_to_json() {
    let page = Page::new(
        vec![
            Cell::new(Role::Heading, TypedContent::text("test")),
            Cell::new(Role::Status, TypedContent::timestamp(1711584000000)),
        ],
        "test",
    );

    let json = serde_json::to_string_pretty(&page).unwrap();
    let roundtrip: Page = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtrip.cells.len(), 2);
    assert_eq!(roundtrip.index.content_type, "test");
}
