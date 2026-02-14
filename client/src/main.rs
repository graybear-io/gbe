//! GBE Client - Terminal UI
//!
//! Renders output and captures input using ratatui.
//!
//! See: docs/design/protocol-v1.md

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gbe_protocol::{ControlMessage, DataFrame};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, info};

mod router_connection;
use router_connection::RouterConnection;

/// GBE Client - Terminal UI
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Router socket path
    #[arg(short, long, default_value = "/tmp/gbe-router.sock")]
    router: String,

    /// Target tool ID to subscribe to
    #[arg(short, long, required = true)]
    target: String,
}

/// Application state
struct App {
    lines: Arc<Mutex<Vec<String>>>,
    follow_mode: bool,
    scroll_offset: usize,
    should_quit: bool,
}

impl App {
    fn new(lines: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            lines,
            follow_mode: true,
            scroll_offset: 0,
            should_quit: false,
        }
    }

    fn toggle_follow_mode(&mut self) {
        self.follow_mode = !self.follow_mode;
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
            self.follow_mode = false;
        }
    }

    fn scroll_down(&mut self) {
        self.scroll_offset += 1;
        self.follow_mode = false;
    }

    fn scroll_to_bottom(&mut self) {
        self.follow_mode = true;
        self.scroll_offset = 0;
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("gbe-client v{}", env!("CARGO_PKG_VERSION"));
    info!("Target: {}", args.target);

    // Connect to router
    let mut router_conn = RouterConnection::connect(&args.router)?;

    // Register with router
    info!("Connecting to router...");
    router_conn.send(&ControlMessage::Connect {
        capabilities: vec![],
    })?;

    // Get assigned ToolId
    let _tool_id = match router_conn.recv()? {
        ControlMessage::ConnectAck {
            tool_id,
            data_listen_address: _,
        } => {
            info!("Assigned ToolId: {}", tool_id);
            tool_id
        }
        msg => {
            anyhow::bail!("Expected ConnectAck, got {:?}", msg);
        }
    };

    // Subscribe to target
    info!("Subscribing to target: {}", args.target);
    router_conn.send(&ControlMessage::Subscribe {
        target: args.target.clone(),
    })?;

    // Get data connection address
    let data_addr = match router_conn.recv()? {
        ControlMessage::SubscribeAck {
            data_connect_address,
            capabilities: _,
        } => {
            info!("Data address: {}", data_connect_address);
            data_connect_address
        }
        ControlMessage::Error { code, message } => {
            anyhow::bail!("Subscription failed: {} - {}", code, message);
        }
        msg => {
            anyhow::bail!("Expected SubscribeAck or Error, got {:?}", msg);
        }
    };

    // Extract path from unix:// URL
    let socket_path = data_addr
        .strip_prefix("unix://")
        .context("Invalid data address format")?;

    // Connect to data stream
    info!("Connecting to data stream: {}", socket_path);
    let data_stream =
        UnixStream::connect(socket_path).context("Failed to connect to data stream")?;

    // Shared line buffer
    let lines = Arc::new(Mutex::new(Vec::new()));
    let lines_clone = lines.clone();

    // Spawn thread to read data frames
    let data_thread = thread::spawn(move || {
        let mut stream = data_stream;
        loop {
            match DataFrame::read_from(&mut stream) {
                Ok(frame) => {
                    if let Ok(line) = String::from_utf8(frame.payload) {
                        let mut lines = lines_clone.lock().unwrap();
                        lines.push(line);
                    }
                }
                Err(e) => {
                    debug!("Data stream closed: {}", e);
                    break;
                }
            }
        }
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run TUI
    let app = App::new(lines);
    let result = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Wait for data thread
    let _ = data_thread.join();

    // Disconnect
    router_conn.send(&ControlMessage::Disconnect)?;

    result?;
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
                .split(f.area());

            // Get lines
            let lines = app.lines.lock().unwrap();
            let total_lines = lines.len();

            // Calculate visible range
            let height = chunks[0].height as usize;
            let (start, end) = if app.follow_mode {
                let start = total_lines.saturating_sub(height);
                (start, total_lines)
            } else {
                let start = app.scroll_offset;
                let end = (start + height).min(total_lines);
                (start, end)
            };

            // Render lines
            let visible_lines: Vec<ListItem> = lines[start..end]
                .iter()
                .map(|line| ListItem::new(line.clone()))
                .collect();

            let list = List::new(visible_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("GBE Client - Lines: {}", total_lines)),
            );

            f.render_widget(list, chunks[0]);

            // Status bar
            let mode = if app.follow_mode {
                "[FOLLOW]"
            } else {
                "[SCROLL]"
            };

            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    mode,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("q", Style::default().fg(Color::Green)),
                Span::raw(":quit  "),
                Span::styled("f", Style::default().fg(Color::Green)),
                Span::raw(":follow  "),
                Span::styled("↑↓", Style::default().fg(Color::Green)),
                Span::raw(":scroll  "),
                Span::styled("End", Style::default().fg(Color::Green)),
                Span::raw(":bottom"),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Keys"));

            f.render_widget(status, chunks[1]);
        })?;

        // Handle input with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('f') => {
                            app.toggle_follow_mode();
                        }
                        KeyCode::Up => {
                            app.scroll_up();
                        }
                        KeyCode::Down => {
                            app.scroll_down();
                        }
                        KeyCode::End => {
                            app.scroll_to_bottom();
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
