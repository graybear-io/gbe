//! POC Operative — executes a periodic task, emits output to stdout.
//!
//! ROLE: Operative (Operative::execute in the real system)
//!
//! In the real system:
//!   - ShellOperative receives TaskDefinition from sentinel via vsock
//!   - Executes params.command, captures stdout
//!   - Returns TaskOutcome (success/failure + output) via vsock
//!
//! For this POC: runs a system monitoring command every N seconds,
//! emitting structured output to stdout. Exits cleanly on SIGTERM
//! or when stdin closes (sentinel disconnected).

use std::io::{self, Write};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static STOP: AtomicBool = AtomicBool::new(false);

fn main() {
    // SIGTERM handler for clean shutdown
    // FUTURE: operative would receive stop via vsock from sentinel
    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, handle_signal as *const () as libc::sighandler_t);
    }

    let interval: u64 = std::env::var("POC_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let mut stdout = io::stdout().lock();

    // FUTURE: this would be a structured TaskOutcome event sent via vsock
    writeln!(stdout, r#"{{"event":"task_started","task":"system-monitor","type":"shell","interval":{interval}}}"#).ok();
    stdout.flush().ok();

    let mut iteration = 0u64;

    while !STOP.load(Ordering::Relaxed) {
        iteration += 1;

        // Run the monitoring commands
        // FUTURE: the command would come from TaskDefinition.params.command
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo \"--- $(date -Iseconds) ---\" && uptime && df -h / | tail -1 && head -3 /proc/meminfo")
            .output();

        match output {
            Ok(out) => {
                // FUTURE: this output would be structured as TaskOutcome.output
                // and published to gbe.tasks.shell.progress via nexus
                stdout.write_all(&out.stdout).ok();
                if !out.stderr.is_empty() {
                    stdout.write_all(&out.stderr).ok();
                }
                stdout.flush().ok();
            }
            Err(e) => {
                writeln!(stdout, r#"{{"event":"command_error","error":"{e}","iteration":{iteration}}}"#).ok();
                stdout.flush().ok();
            }
        }

        // Sleep in small increments so we can respond to signals quickly
        for _ in 0..(interval * 10) {
            if STOP.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    // FUTURE: operative sends TaskOutcome::Success to sentinel via vsock
    writeln!(stdout, r#"{{"event":"task_completed","task":"system-monitor","iterations":{iteration}}}"#).ok();
    stdout.flush().ok();
}

extern "C" fn handle_signal(_sig: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}
