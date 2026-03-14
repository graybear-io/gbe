mod protocol;

use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::header;
use tungstenite::protocol::Message;
use tungstenite::stream::MaybeTlsStream;

use protocol::{SERVER_OUTPUT, frame_input, frame_resize, init_message, parse_server_message};

static DONE: AtomicBool = AtomicBool::new(false);

fn main() {
    let debug = std::env::var("TTYD_DEBUG").is_ok();
    let url = match parse_url() {
        Some(u) => u,
        None => {
            eprintln!("usage: ttyd-connect <host:port | ws://host:port/ws>");
            std::process::exit(1);
        }
    };

    let mut request = url.into_client_request().expect("invalid URL");
    request
        .headers_mut()
        .insert(header::SEC_WEBSOCKET_PROTOCOL, "tty".parse().unwrap());

    let (mut ws, _response) = tungstenite::connect(request).expect("failed to connect to ttyd");

    // Send auth/init handshake — ttyd spawns the shell after receiving this
    let (cols, rows) = term_size().unwrap_or((80, 24));
    ws.send(Message::Text(init_message(cols, rows).into()))
        .expect("failed to send init handshake");

    // Read initial messages (title, preferences) until first output
    let mut stdout = io::stdout().lock();
    loop {
        match ws.read() {
            Ok(Message::Binary(data)) => {
                if let Some((typ, payload)) = parse_server_message(&data) {
                    if debug {
                        eprintln!(
                            "[debug] type=0x{typ:02x} len={} preview={:?}",
                            data.len(),
                            String::from_utf8_lossy(&payload[..payload.len().min(60)])
                        );
                    }
                    if typ == SERVER_OUTPUT {
                        stdout.write_all(payload).ok();
                        stdout.flush().ok();
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("error during handshake: {e}");
                std::process::exit(1);
            }
            _ => {}
        }
    }
    drop(stdout);

    // Set read timeout for the main loop
    if let MaybeTlsStream::Plain(ref tcp) = *ws.get_ref() {
        tcp.set_read_timeout(Some(Duration::from_millis(10))).ok();
    }

    // Put terminal in raw mode
    let _raw_guard = RawMode::enable();

    // Stdin reader thread
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        while !DONE.load(Ordering::Relaxed) {
            match handle.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // SIGWINCH handler for terminal resize
    let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>();
    install_sigwinch_handler(resize_tx);

    let mut stdout = io::stdout().lock();

    loop {
        // Forward stdin
        while let Ok(bytes) = rx.try_recv() {
            if ws
                .send(Message::Binary(frame_input(&bytes).into()))
                .is_err()
            {
                DONE.store(true, Ordering::Relaxed);
                return;
            }
        }

        // Forward resize events
        while let Ok((c, r)) = resize_rx.try_recv() {
            ws.send(Message::Binary(frame_resize(c, r).into())).ok();
        }

        // Read from WebSocket
        match ws.read() {
            Ok(Message::Binary(data)) => {
                if let Some((typ, payload)) = parse_server_message(&data)
                    && typ == SERVER_OUTPUT
                {
                    stdout.write_all(payload).ok();
                    stdout.flush().ok();
                }
            }
            Ok(Message::Ping(data)) => {
                ws.send(Message::Pong(data)).ok();
            }
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
            }
            Err(e) => {
                if debug {
                    eprintln!("[debug] connection error: {e}");
                }
                break;
            }
            _ => {}
        }
    }

    DONE.store(true, Ordering::Relaxed);
}

// --- URL parsing ---

fn parse_url() -> Option<String> {
    let arg = std::env::args().nth(1)?;
    if arg.starts_with("ws://") || arg.starts_with("wss://") {
        Some(arg)
    } else {
        Some(format!("ws://{arg}/ws"))
    }
}

// --- Terminal size ---

fn term_size() -> Option<(u16, u16)> {
    unsafe {
        let mut ws = std::mem::MaybeUninit::<libc::winsize>::uninit();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, ws.as_mut_ptr()) == 0 {
            let ws = ws.assume_init();
            Some((ws.ws_col, ws.ws_row))
        } else {
            None
        }
    }
}

// --- SIGWINCH ---

/// Pipe-based signal handler. Write end goes to signal handler (async-signal-safe),
/// read end is polled in the main loop via mpsc.
fn install_sigwinch_handler(resize_tx: mpsc::Sender<(u16, u16)>) {
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    let read_fd = fds[0];
    let write_fd = fds[1];

    // Set write end non-blocking so signal handler never blocks
    unsafe {
        let flags = libc::fcntl(write_fd, libc::F_GETFL);
        libc::fcntl(write_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    // Store write fd for signal handler
    SIGWINCH_PIPE.store(write_fd, Ordering::SeqCst);

    unsafe {
        libc::signal(
            libc::SIGWINCH,
            sigwinch_handler as *const () as libc::sighandler_t,
        );
    }

    // Reader thread: reads from pipe, queries term size, sends to channel
    thread::spawn(move || {
        let mut byte = [0u8; 1];
        while !DONE.load(Ordering::Relaxed) {
            let n = unsafe { libc::read(read_fd, byte.as_mut_ptr().cast(), 1) };
            if n > 0 {
                if let Some(size) = term_size() {
                    let _ = resize_tx.send(size);
                }
            } else {
                thread::sleep(Duration::from_millis(50));
            }
        }
        unsafe { libc::close(read_fd) };
    });
}

use std::sync::atomic::AtomicI32;
static SIGWINCH_PIPE: AtomicI32 = AtomicI32::new(-1);

extern "C" fn sigwinch_handler(_sig: libc::c_int) {
    let fd = SIGWINCH_PIPE.load(Ordering::SeqCst);
    if fd >= 0 {
        unsafe {
            let _ = libc::write(fd, [1u8].as_ptr().cast(), 1);
        }
    }
}

// --- Raw terminal mode ---

struct RawMode {
    orig: libc::termios,
}

impl RawMode {
    fn enable() -> Self {
        unsafe {
            let mut orig = std::mem::zeroed::<libc::termios>();
            libc::tcgetattr(libc::STDIN_FILENO, &mut orig);
            let mut raw = orig;
            libc::cfmakeraw(&mut raw);
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
            Self { orig }
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.orig);
        }
        print!("\x1b[?25h\x1b[0m\r\n");
        io::stdout().flush().ok();
    }
}
