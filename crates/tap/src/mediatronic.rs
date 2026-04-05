//! Terminal mediatronic — renders primer pages as ratatui widgets.
//!
//! This is a layout imprint expressed as code. It maps cell roles to
//! ratatui widgets in a three-panel layout: filters | events | detail.
//!
//! The mediatronic doesn't know about writs, lifecycle events, or
//! packets. It knows about cells, roles, and pages.

use primer::{Cell, Feed, Page, Role, TypedContent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

/// Render typed content to a compact string for list views.
/// The mediatronic decides how much space to give based on its constraints.
pub fn render_content_compact(content: &TypedContent) -> String {
    match content {
        TypedContent::Text(s) => s.clone(),
        TypedContent::Number { value, unit } => {
            if let Some(u) = unit {
                format!("{value}{u}")
            } else if *value == (*value as i64) as f64 {
                format!("{}", *value as i64)
            } else {
                format!("{value:.1}")
            }
        }
        TypedContent::Timestamp(ms) => format_ts(*ms),
        TypedContent::Pair { label, value } => {
            format!("{}: {}", label, render_content_compact(value))
        }
        TypedContent::List(items) => items
            .iter()
            .map(render_content_compact)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Render typed content to a multi-line detail string.
pub fn render_content_detail(content: &TypedContent, indent: usize) -> String {
    let pad = " ".repeat(indent);
    match content {
        TypedContent::Text(s) => format!("{pad}{s}"),
        TypedContent::Number { value, unit } => {
            if let Some(u) = unit {
                format!("{pad}{value} {u}")
            } else if *value == (*value as i64) as f64 {
                format!("{pad}{}", *value as i64)
            } else {
                format!("{pad}{value}")
            }
        }
        TypedContent::Timestamp(ms) => format!("{pad}{}", format_ts(*ms)),
        TypedContent::Pair { label, value } => {
            format!("{pad}{}: {}", label, render_content_compact(value))
        }
        TypedContent::List(items) => items
            .iter()
            .map(|item| render_content_detail(item, indent))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Extract a one-line summary from a page: the heading cell's content.
pub fn page_summary(page: &Page) -> String {
    page.cells
        .iter()
        .find(|c| c.role == Role::Heading)
        .map(|c| render_content_compact(&c.content))
        .unwrap_or_else(|| "(no heading)".to_string())
}

/// Extract the domain color from a page's source subject.
pub fn domain_color(page: &Page) -> Color {
    let source = page
        .index
        .source
        .as_deref()
        .unwrap_or("");
    if source.starts_with("gbe.") || source.starts_with("lifecycle.") {
        Color::Cyan
    } else if source.starts_with("akasha.") {
        Color::Magenta
    } else if source.starts_with("allthing.") {
        Color::Green
    } else if source.starts_with("writs.") {
        Color::Yellow
    } else {
        Color::White
    }
}

/// Extract the domain name from a page's source.
fn domain_name(page: &Page) -> &str {
    page.index
        .source
        .as_deref()
        .and_then(|s| s.split('.').next())
        .unwrap_or("?")
}

/// Build a detail view for the selected page by rendering all cells
/// grouped by role.
pub fn page_detail(page: &Page) -> String {
    let mut lines = Vec::new();

    // Page metadata.
    if let Some(source) = &page.index.source {
        lines.push(format!("Subject: {source}"));
    }
    lines.push(format!("Type: {}", page.index.content_type));
    if let Some(compiled_by) = &page.index.compiled_by {
        lines.push(format!("Imprint: {compiled_by}"));
    }
    lines.push(format!("Cells: {}", page.cells.len()));
    lines.push(String::new());

    // Group cells by role for display.
    let role_order = [Role::Heading, Role::Summary, Role::Status, Role::Detail, Role::Action];

    for role in &role_order {
        let cells: Vec<&Cell> = page.cells.iter().filter(|c| &c.role == role).collect();
        if cells.is_empty() {
            continue;
        }

        let role_label = match role {
            Role::Heading => "Heading",
            Role::Summary => "Summary",
            Role::Status => "Status",
            Role::Detail => "Detail",
            Role::Action => "Action",
        };

        lines.push(format!("--- {role_label} ---"));
        for cell in cells {
            lines.push(render_content_detail(&cell.content, 2));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Filter state for the domain filter panel.
pub struct FilterPanel {
    pub filters: Vec<FilterEntry>,
    pub state: ListState,
}

pub struct FilterEntry {
    pub name: String,
    /// Matches against page source or content_type.
    pub prefix: String,
    pub count: usize,
}

impl FilterPanel {
    pub fn new() -> Self {
        let filters = vec![
            FilterEntry { name: "all".into(), prefix: String::new(), count: 0 },
            FilterEntry { name: "gbe".into(), prefix: "gbe.".into(), count: 0 },
            FilterEntry { name: "akasha".into(), prefix: "akasha.".into(), count: 0 },
            FilterEntry { name: "allthing".into(), prefix: "allthing.".into(), count: 0 },
            FilterEntry { name: "lifecycle".into(), prefix: "lifecycle.".into(), count: 0 },
            FilterEntry { name: "writs".into(), prefix: "writs.".into(), count: 0 },
        ];
        let mut state = ListState::default();
        state.select(Some(0));
        Self { filters, state }
    }

    pub fn selected_prefix(&self) -> &str {
        self.state
            .selected()
            .and_then(|i| self.filters.get(i))
            .map(|f| f.prefix.as_str())
            .unwrap_or("")
    }

    pub fn next(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        let next = if i >= self.filters.len() - 1 { 0 } else { i + 1 };
        self.state.select(Some(next));
    }

    pub fn prev(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        let prev = if i == 0 { self.filters.len() - 1 } else { i - 1 };
        self.state.select(Some(prev));
    }

    pub fn update_counts(&mut self, feed: &Feed) {
        for filter in &mut self.filters {
            filter.count = if filter.prefix.is_empty() {
                feed.len()
            } else {
                feed.pages()
                    .iter()
                    .filter(|p| {
                        p.index
                            .source
                            .as_deref()
                            .is_some_and(|s| s.starts_with(&filter.prefix))
                    })
                    .count()
            };
        }
    }

    /// Indices of pages in the feed that match the current filter.
    pub fn filtered_indices(&self, feed: &Feed) -> Vec<usize> {
        let prefix = self.selected_prefix();
        feed.pages()
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                prefix.is_empty()
                    || p.index
                        .source
                        .as_deref()
                        .is_some_and(|s| s.starts_with(prefix))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

/// Draw the three-panel layout.
pub fn draw(
    f: &mut Frame,
    feed: &Feed,
    filter_panel: &mut FilterPanel,
    event_state: &mut ListState,
    show_detail: bool,
    transport_label: &str,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let main_constraints = if show_detail {
        vec![
            Constraint::Length(18),
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ]
    } else {
        vec![Constraint::Length(18), Constraint::Min(1)]
    };

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(main_constraints)
        .split(outer[0]);

    // Left: domain filters.
    let filter_items: Vec<ListItem> = filter_panel
        .filters
        .iter()
        .map(|entry| ListItem::new(format!("{} ({})", entry.name, entry.count)))
        .collect();

    let filters_widget = List::new(filter_items)
        .block(Block::default().borders(Borders::ALL).title(" Domains "))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(filters_widget, main[0], &mut filter_panel.state);

    // Middle: event list from feed pages.
    let filtered = filter_panel.filtered_indices(feed);
    let filtered_count = filtered.len();

    let event_items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let page = &feed.pages()[idx];
            let ts = if page.index.timestamp > 0 {
                format!("{} ", format_ts(page.index.timestamp))
            } else {
                String::new()
            };
            let color = domain_color(page);
            let domain = domain_name(page);
            let summary = page_summary(page);
            let frame_count = page
                .cells
                .iter()
                .filter(|c| c.role == Role::Detail)
                .count();
            let frames_label = if page.index.content_type == "packet" {
                format!(" [{frame_count}F]")
            } else {
                String::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(ts, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<9} ", domain),
                    Style::default().fg(color),
                ),
                Span::raw(summary),
                Span::styled(frames_label, Style::default().fg(Color::Yellow)),
            ]))
        })
        .collect();

    let detail_text = if show_detail {
        event_state
            .selected()
            .and_then(|sel| filtered.get(sel))
            .map(|&idx| page_detail(&feed.pages()[idx]))
            .unwrap_or_else(|| "(no event selected)".to_string())
    } else {
        String::new()
    };

    let events_widget = List::new(event_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Events ({filtered_count}) ")),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(events_widget, main[1], event_state);

    // Right: detail panel.
    if show_detail && main.len() > 2 {
        let detail_widget = Paragraph::new(detail_text)
            .block(Block::default().borders(Borders::ALL).title(" Detail "))
            .wrap(Wrap { trim: false });
        f.render_widget(detail_widget, main[2]);
    }

    // Status bar.
    let follow_indicator = if feed.is_following() { "follow" } else { "paused" };
    let status = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            transport_label.to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("total: {}", feed.len()),
            Style::default().fg(Color::White),
        ),
        Span::raw(" | "),
        Span::styled(follow_indicator, Style::default().fg(
            if feed.is_following() { Color::Green } else { Color::Yellow }
        )),
        Span::raw(" | "),
        Span::styled(
            "j/k: events  J/K: domains  enter: detail  d: trace  q: quit",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(status), outer[1]);
}

fn format_ts(ts: u64) -> String {
    let secs = ts / 1000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    let ms = ts % 1000;
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}
