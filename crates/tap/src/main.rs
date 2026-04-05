//! Bus tap — subscribe to ecosystem subjects, inspect packets and frames in real time.
//!
//! A diagnostic tool that taps the nexus bus across all domains (gbe, akasha, allthing).
//! Displays events as they flow, with frame stack inspection for Packet payloads.
//!
//! Built on primer: bus messages are compiled into cells/pages via imprints,
//! collected in a feed, and rendered by a terminal mediatronic.

mod mediatronic;

use std::io;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use tokio::sync::mpsc;

use gbe_nexus::{
    DomainPayload, Message, MessageHandler, StartPosition, SubscribeOpts, Transport, TransportError,
};
use primer::{Feed, Page};
use primer::imprint::ImprintRegistry;
use primer::imprints::default_registry;

use mediatronic::FilterPanel;

/// A compiled page with its raw source data for diagnostic inspection.
struct TapPage {
    page: Page,
    raw: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Message handler: receives bus messages, compiles via imprints, sends pages
// ---------------------------------------------------------------------------

struct TapHandler {
    tx: mpsc::UnboundedSender<TapPage>,
    registry: Arc<ImprintRegistry>,
}

#[async_trait]
impl MessageHandler for TapHandler {
    async fn handle(&self, msg: &dyn Message) -> Result<(), TransportError> {
        let envelope = msg.envelope();
        let subject = envelope.subject.clone();
        let payload_bytes = msg.payload();

        // Try to parse as DomainPayload, fall back to raw JSON.
        let data: serde_json::Value =
            if let Ok(dp) = DomainPayload::<serde_json::Value>::from_bytes(&payload_bytes) {
                dp.data
            } else if let Ok(val) = serde_json::from_slice(&payload_bytes) {
                val
            } else {
                let raw = String::from_utf8_lossy(&payload_bytes);
                serde_json::json!({ "raw": raw.to_string() })
            };

        // Compile through the imprint registry.
        if let Some(page) = self.registry.compile(&subject, &data) {
            let page = page.with_timestamp(envelope.timestamp);
            let _ = self.tx.send(TapPage { page, raw: data });
        }

        msg.ack().await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Subscription setup
// ---------------------------------------------------------------------------

/// Build the full list of subjects to subscribe to across all domains.
fn all_subjects() -> Vec<String> {
    let mut subjects = Vec::new();

    // GBE lifecycle.
    let roles = gbe_architect::role_names();
    let all_components: Vec<&str> = roles
        .iter()
        .copied()
        .chain(["overseer", "herald"].iter().copied())
        .collect();

    for comp in &all_components {
        subjects.push(gbe_jobs_domain::subjects::lifecycle::started(comp));
        subjects.push(gbe_jobs_domain::subjects::lifecycle::stopped(comp));
        subjects.push(gbe_jobs_domain::subjects::lifecycle::heartbeat(comp));
        subjects.push(gbe_jobs_domain::subjects::lifecycle::degraded(comp));
        subjects.push(gbe_jobs_domain::subjects::lifecycle::capabilities(comp));
    }

    // Writs (ecosystem infrastructure).
    for role in roles {
        subjects.push(gbe_jobs_domain::subjects::writs::role(role));
    }
    subjects.push(gbe_jobs_domain::subjects::writs::RESPONSES.to_string());

    // Akasha domain events.
    subjects.push("akasha.collect.url".to_string());
    subjects.push("akasha.gathered.completed".to_string());
    subjects.push("akasha.gathered.failed".to_string());
    subjects.push("akasha.triage.completed".to_string());
    subjects.push("akasha.gather.key".to_string());

    // Allthing comlog.
    subjects.push("allthing.comlog.discord".to_string());
    subjects.push("allthing.comlog.overseer".to_string());

    subjects
}

async fn subscribe_all(
    transport: &Arc<dyn Transport>,
    tx: &mpsc::UnboundedSender<TapPage>,
    registry: &Arc<ImprintRegistry>,
) -> Result<Vec<Box<dyn gbe_nexus::Subscription>>, TransportError> {
    let mut subs = Vec::new();

    for subject in all_subjects() {
        let _ = transport
            .ensure_stream(gbe_nexus::StreamConfig {
                subject: subject.clone(),
                max_age: Duration::from_secs(86400),
                max_bytes: None,
                max_msgs: None,
            })
            .await;

        let sub = transport
            .subscribe(
                &subject,
                "tap",
                Box::new(TapHandler {
                    tx: tx.clone(),
                    registry: Arc::clone(registry),
                }),
                Some(SubscribeOpts {
                    start_from: StartPosition::Latest,
                    ..Default::default()
                }),
            )
            .await?;
        subs.push(sub);
    }

    Ok(subs)
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a == "--demo");

    let (transport, label): (Arc<dyn Transport>, String) = if demo {
        let t = gbe_nexus_memory::MemoryTransport::new(
            gbe_nexus_memory::MemoryTransportConfig::default(),
        );
        (Arc::new(t), "memory (demo)".to_string())
    } else {
        let shared = frame::config::load_shared().unwrap_or_default();
        let config = gbe_nexus_redis::RedisTransportConfig {
            url: shared.redis_url.clone(),
            max_payload_size: shared.max_payload_size,
        };
        let t = gbe_nexus_redis::RedisTransport::connect(config).await?;
        (Arc::new(t), format!("redis: {}", shared.redis_url))
    };

    // Matter compiler: imprint registry with all built-in imprints.
    let registry = Arc::new(default_registry());

    // Feed: the ordered sequence of pages.
    let mut feed = Feed::new(5000);
    // Parallel raw data store — indexed same as feed, for pipeline traces.
    let mut raw_data: Vec<serde_json::Value> = Vec::new();

    // Channel for pages from the bus handler.
    let (tx, mut rx) = mpsc::unbounded_channel::<TapPage>();

    let _subs = subscribe_all(&transport, &tx, &registry).await?;

    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut filter_panel = FilterPanel::new();
    let mut event_state = ListState::default();
    let mut show_detail = false;

    loop {
        // Ingest new pages from the bus.
        while let Ok(tap_page) = rx.try_recv() {
            feed.push(tap_page.page);
            raw_data.push(tap_page.raw);
            // Keep raw_data in sync with feed capacity.
            while raw_data.len() > feed.len() {
                raw_data.remove(0);
            }
        }
        filter_panel.update_counts(&feed);

        // Auto-follow: keep event selection at the tail if following.
        let filtered = filter_panel.filtered_indices(&feed);
        if feed.is_following() && !filtered.is_empty() {
            event_state.select(Some(filtered.len() - 1));
        }

        // Draw.
        terminal.draw(|f| {
            mediatronic::draw(
                f,
                &feed,
                &mut filter_panel,
                &mut event_state,
                show_detail,
                &label,
            );
        })?;

        // Input.
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') | KeyCode::Down => {
                    let filtered = filter_panel.filtered_indices(&feed);
                    let len = filtered.len();
                    if len > 0 {
                        let i = event_state.selected().unwrap_or(0);
                        let next = if i >= len - 1 { len - 1 } else { i + 1 };
                        event_state.select(Some(next));
                        // Disable follow if not at the end.
                        if next < len - 1 {
                            feed.pause_follow();
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let filtered = filter_panel.filtered_indices(&feed);
                    if !filtered.is_empty() {
                        let i = event_state.selected().unwrap_or(0);
                        let prev = if i == 0 { 0 } else { i - 1 };
                        event_state.select(Some(prev));
                        feed.pause_follow();
                    }
                }
                KeyCode::Char('G') => {
                    // Jump to bottom, resume follow.
                    let filtered = filter_panel.filtered_indices(&feed);
                    if !filtered.is_empty() {
                        event_state.select(Some(filtered.len() - 1));
                        feed.resume_follow();
                    }
                }
                KeyCode::Char('g') => {
                    // Jump to top.
                    let filtered = filter_panel.filtered_indices(&feed);
                    if !filtered.is_empty() {
                        event_state.select(Some(0));
                        feed.pause_follow();
                    }
                }
                KeyCode::Char('J') => {
                    filter_panel.next();
                    event_state.select(None);
                }
                KeyCode::Char('K') => {
                    filter_panel.prev();
                    event_state.select(None);
                }
                KeyCode::Char('d') => {
                    // Pipeline trace: dump the selected event's full compilation trace.
                    let filtered = filter_panel.filtered_indices(&feed);
                    if let Some(&feed_idx) = event_state.selected().and_then(|sel| filtered.get(sel)) {
                        if let Some(raw) = raw_data.get(feed_idx) {
                            let subject = feed.pages()[feed_idx]
                                .index
                                .source
                                .as_deref()
                                .unwrap_or("?");
                            let trace = registry.compile_traced(subject, raw);

                            // Temporarily leave TUI to show trace.
                            disable_raw_mode()?;
                            crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                            println!("{trace}");
                            println!("Press enter to return to tap...");

                            let mut buf = String::new();
                            io::stdin().read_line(&mut buf)?;

                            enable_raw_mode()?;
                            crossterm::execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            terminal.clear()?;
                        }
                    }
                }
                KeyCode::Enter => show_detail = !show_detail,
                KeyCode::Esc => show_detail = false,
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
