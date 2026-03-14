//! POC Sentinel — host-level task lifecycle manager.
//!
//! ROLE: Sentinel (per-host service in the real system)
//!
//! In the real system:
//!   - Sentinel runs on each VM host
//!   - Claims tasks from gbe.tasks.shell.queue (CAS on state store)
//!   - Boots Firecracker VM per task, injects operative
//!   - Publishes its own lifecycle events to gbe.sentinel.<host>.lifecycle
//!   - Publishes task progress to gbe.tasks.shell.progress.<task-id>
//!   - Operative output flows through gbe.tasks.shell.output.<task-id>
//!
//! For this POC:
//!   - Registers with envoy router for its own lifecycle stream
//!   - Starts an adapter per operative for task output
//!   - Monitors operatives, publishes progress events
//!   - All events also written to /tmp/nexus/ files (nexus stand-in)

use gbe_protocol::{ControlMessage, DataFrame};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

static STOP: AtomicBool = AtomicBool::new(false);

const TASK_ID: &str = "system-monitor";
const HOST_ID: &str = "ark";
const NEXUS_DIR: &str = "/tmp/nexus";
const DEFAULT_ROUTER: &str = "/tmp/gbe-router.sock";

fn main() {
    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, handle_signal as *const () as libc::sighandler_t);
    }

    let router_sock = std::env::var("GBE_ROUTER").unwrap_or_else(|_| DEFAULT_ROUTER.into());
    let nexus = Path::new(NEXUS_DIR);
    fs::create_dir_all(nexus).expect("failed to create nexus dir");

    // Nexus files (persistence — stands in for Redis streams)
    let lifecycle_path = nexus.join(format!("gbe.sentinel.{HOST_ID}.lifecycle"));
    let progress_path = nexus.join(format!("gbe.tasks.shell.progress.{TASK_ID}"));
    let terminal_path = nexus.join(format!("gbe.tasks.shell.terminal.{TASK_ID}"));
    let trace_path = nexus.join("gbe.trace.sentinel");

    for path in [&lifecycle_path, &progress_path, &terminal_path, &trace_path] {
        File::create(path).unwrap_or_else(|e| panic!("failed to create {}: {e}", path.display()));
    }

    let mut lifecycle_file = open_append(&lifecycle_path);
    let mut progress_file = open_append(&progress_path);
    let mut terminal_file = open_append(&terminal_path);
    let mut trace = open_append(&trace_path);

    // Register sentinel's own lifecycle stream with envoy
    log(&mut trace, "registering lifecycle stream with envoy router");
    let lifecycle_stream = EnvoyStream::register(
        &router_sock,
        vec!["sentinel-lifecycle".into()],
        [
            ("stream_type".into(), "sentinel-lifecycle".into()),
            ("label".into(), format!("sentinel/{HOST_ID}")),
            ("host".into(), HOST_ID.into()),
        ].into(),
    );
    match &lifecycle_stream {
        Ok(s) => log(&mut trace, &format!("lifecycle stream: tool_id={}", s.tool_id)),
        Err(e) => log(&mut trace, &format!("WARNING: failed to register lifecycle stream: {e}")),
    }
    let lifecycle_stream = lifecycle_stream.ok().map(|s| Arc::new(Mutex::new(s)));

    // Emit sentinel startup
    emit_event(
        &format!(r#"{{"event":"sentinel_started","host":"{HOST_ID}"}}"#),
        &mut lifecycle_file,
        &lifecycle_stream,
    );

    // Start the operative via adapter (operative output = separate envoy stream)
    let local_mode = std::env::var("POC_LOCAL").is_ok();
    let operative_bin = std::env::var("POC_OPERATIVE_BIN")
        .unwrap_or_else(|_| "/mnt/projects/gbe/poc/target/debug/poc-operative".into());

    let adapter_bin = std::env::var("POC_ADAPTER_BIN")
        .unwrap_or_else(|_| {
            // Find adapter relative to sentinel
            let envoy_dir = std::env::var("ENVOY_DIR")
                .unwrap_or_else(|_| "../gbe-envoy".into());
            format!("{envoy_dir}/target/debug/gbe-adapter")
        });

    log(&mut trace, &format!("starting adapter for operative output"));

    // Sentinel starts an adapter that wraps the operative via SSH
    // The adapter registers with the router as a separate tool (output stream)
    let operative_cmd = if local_mode {
        operative_bin.clone()
    } else {
        format!("ssh ark '. $HOME/.cargo/env && {operative_bin}'")
    };

    let mut adapter_child = Command::new(&adapter_bin)
        .args(["--router", &router_sock, "--", "sh", "-c", &operative_cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn adapter for operative");

    let adapter_pid = adapter_child.id();
    log(&mut trace, &format!("adapter started: pid={adapter_pid}"));

    // Emit task claimed
    emit_event(
        &format!(r#"{{"event":"sentinel_claimed","task":"{TASK_ID}","host":"{HOST_ID}"}}"#),
        &mut lifecycle_file,
        &lifecycle_stream,
    );

    // Pipe adapter stdout/stderr to trace (adapter's real output goes through envoy)
    let adapter_stdout = adapter_child.stdout.take();
    let adapter_stderr = adapter_child.stderr.take();
    let trace_path_c1 = trace_path.clone();
    let trace_path_c2 = trace_path.clone();

    if let Some(stdout) = adapter_stdout {
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut t = open_append(&trace_path_c1);
            for line in reader.lines().flatten() {
                log(&mut t, &format!("adapter stdout: {line}"));
            }
        });
    }
    if let Some(stderr) = adapter_stderr {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut t = open_append(&trace_path_c2);
            for line in reader.lines().flatten() {
                log(&mut t, &format!("adapter stderr: {line}"));
            }
        });
    }

    // Register progress stream with envoy (task-level events)
    log(&mut trace, "registering progress stream with envoy router");
    let progress_stream = EnvoyStream::register(
        &router_sock,
        vec!["task-progress".into()],
        [
            ("stream_type".into(), "task-progress".into()),
            ("label".into(), format!("progress/{TASK_ID}")),
            ("task_id".into(), TASK_ID.into()),
            ("host".into(), HOST_ID.into()),
        ].into(),
    );
    match &progress_stream {
        Ok(s) => log(&mut trace, &format!("progress stream: tool_id={}", s.tool_id)),
        Err(e) => log(&mut trace, &format!("WARNING: failed to register progress stream: {e}")),
    }
    let progress_stream = progress_stream.ok().map(|s| Arc::new(Mutex::new(s)));

    emit_event(
        &format!(r#"{{"event":"operative_started","task":"{TASK_ID}","adapter_pid":{adapter_pid}}}"#),
        &mut progress_file,
        &progress_stream,
    );

    // Monitor loop
    let mut tick = 0u64;
    loop {
        if STOP.load(Ordering::Relaxed) {
            emit_event(
                &format!(r#"{{"event":"stop_received","task":"{TASK_ID}","tick":{tick}}}"#),
                &mut progress_file,
                &progress_stream,
            );
            log(&mut trace, "stop signal received, terminating adapter");

            // Kill adapter (which kills the operative SSH session)
            kill_child(&mut adapter_child);
            let status = adapter_child.wait().ok();
            log(&mut trace, &format!("adapter exited: {status:?}"));

            emit_event(
                &format!(r#"{{"event":"operative_exited","task":"{TASK_ID}","tick":{tick}}}"#),
                &mut progress_file,
                &progress_stream,
            );

            let term = format!(r#"{{"event":"task_terminal","outcome":"stopped","task":"{TASK_ID}"}}"#);
            emit_event(&term, &mut progress_file, &progress_stream);
            writeln!(terminal_file, "{term}").ok();

            emit_event(
                &format!(r#"{{"event":"sentinel_task_ended","task":"{TASK_ID}","outcome":"stopped"}}"#),
                &mut lifecycle_file,
                &lifecycle_stream,
            );
            break;
        }

        // Check if adapter (and therefore operative) is still alive
        match adapter_child.try_wait() {
            Ok(Some(status)) => {
                log(&mut trace, &format!("adapter exited on its own: {status}"));
                emit_event(
                    &format!(r#"{{"event":"operative_exited","task":"{TASK_ID}","tick":{tick},"status":"{status}"}}"#),
                    &mut progress_file,
                    &progress_stream,
                );
                let term = format!(r#"{{"event":"task_terminal","outcome":"completed","task":"{TASK_ID}"}}"#);
                emit_event(&term, &mut progress_file, &progress_stream);
                writeln!(terminal_file, "{term}").ok();

                emit_event(
                    &format!(r#"{{"event":"sentinel_task_ended","task":"{TASK_ID}","outcome":"completed"}}"#),
                    &mut lifecycle_file,
                    &lifecycle_stream,
                );
                break;
            }
            Ok(None) => {
                // Still running — heartbeat
                emit_event(
                    &format!(r#"{{"event":"heartbeat","task":"{TASK_ID}","state":"running","tick":{tick}}}"#),
                    &mut progress_file,
                    &progress_stream,
                );
            }
            Err(e) => {
                log(&mut trace, &format!("error checking adapter: {e}"));
                break;
            }
        }

        tick += 1;
        for _ in 0..50 {
            if STOP.load(Ordering::Relaxed) { break; }
            thread::sleep(Duration::from_millis(100));
        }
    }

    emit_event(
        &format!(r#"{{"event":"sentinel_stopping","host":"{HOST_ID}"}}"#),
        &mut lifecycle_file,
        &lifecycle_stream,
    );

    log(&mut trace, "sentinel exiting");
}

// --- Envoy stream: registers as a tool, accepts subscribers, publishes frames ---

struct EnvoyStream {
    tool_id: String,
    _control: UnixStream, // Keep alive — router unregisters on drop
    listener: UnixListener,
    clients: Vec<UnixStream>,
    seq: AtomicU64,
}

impl EnvoyStream {
    fn register(
        router_sock: &str,
        capabilities: Vec<String>,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = UnixStream::connect(router_sock)?;
        let control = stream.try_clone()?;
        let mut writer = stream.try_clone()?;
        let mut reader = BufReader::new(stream);

        let msg = serde_json::to_string(&ControlMessage::Connect { capabilities, metadata })?;
        writeln!(writer, "{msg}")?;
        writer.flush()?;

        let mut line = String::new();
        reader.read_line(&mut line)?;
        let ack: ControlMessage = serde_json::from_str(line.trim())?;

        let (tool_id, data_addr) = match ack {
            ControlMessage::ConnectAck { tool_id, data_listen_address } => (tool_id, data_listen_address),
            _ => return Err("expected ConnectAck".into()),
        };

        let socket_path = data_addr.strip_prefix("unix://")
            .ok_or("invalid data address")?;
        let listener = UnixListener::bind(socket_path)?;
        listener.set_nonblocking(true)?;

        Ok(Self { tool_id, _control: control, listener, clients: Vec::new(), seq: AtomicU64::new(0) })
    }

    fn publish(&mut self, json: &str) {
        // Accept new subscribers
        while let Ok((stream, _)) = self.listener.accept() {
            self.clients.push(stream);
        }

        if self.clients.is_empty() { return; }

        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let frame = DataFrame { seq, payload: format!("{json}\n").into_bytes() };
        let bytes = frame.to_bytes();

        self.clients.retain(|client| {
            let mut c = client;
            c.write_all(&bytes).is_ok() && c.flush().is_ok()
        });
    }
}

fn emit_event(json: &str, file: &mut File, stream: &Option<Arc<Mutex<EnvoyStream>>>) {
    writeln!(file, "{json}").ok();
    file.flush().ok();
    if let Some(s) = stream {
        if let Ok(mut s) = s.lock() {
            s.publish(json);
        }
    }
}

// --- Helpers ---

fn open_append(path: &Path) -> File {
    OpenOptions::new().create(true).append(true).open(path)
        .unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()))
}

fn log(file: &mut File, msg: &str) {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    writeln!(file, "[{now}] {msg}").ok();
    file.flush().ok();
}

fn kill_child(child: &mut Child) {
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM); }
    thread::sleep(Duration::from_secs(2));
    let _ = child.kill();
}

extern "C" fn handle_signal(_sig: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}
