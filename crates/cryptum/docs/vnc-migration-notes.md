# Display Migration Attempts — 2026-03-13/14

## Goal

Replace the custom Smithay compositor with a headless VNC-accessible
display stack so cryptum can run on headless servers without a GPU.

## Environment

- Alpine 3.21 (x86_64) in UTM VM on macOS
- No GPU passthrough — virtual card0/renderD128 but CREATE_DUMB fails
- 9p shared filesystem (has dentry cache issues — flags/files created
  on host not visible to running processes on guest)
- TigerVNC 1.16.0 viewer on macOS
- macOS Screen Sharing unable to connect to wayvnc at all

## What We Tried

### 1. Sway headless + wayvnc (default GLES renderer)

- `WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1`
- **Result**: DRM_IOCTL_MODE_CREATE_DUMB permission denied (GBM
  buffer allocation fails). Sway starts but can't allocate output
  buffers. VNC shows solid color, foot not visible.

### 2. Sway headless + wayvnc + pixman renderer

- Added `WLR_RENDERER=pixman`
- **Result**: No DRM errors. Sway runs, foot renders. But wayvnc
  sends the wrong buffer to VNC — foot appears at ~696x494 in the
  bottom-left of the 1280x720 window. `grim` screenshots from inside
  sway show correct full-screen rendering, confirming the compositor
  is correct but wayvnc misinterprets the pixman buffer stride/pitch.
- Tried `WLR_ALLOCATOR=shm` — no change.
- Tried `foot --fullscreen`, `foot --maximized`, `for_window` resize,
  `for_window fullscreen enable` — sway tree reports correct 1280x720
  rect but VNC display unchanged.
- Very slow keyboard response with pixman renderer.

### 3. Sway headless + wayvnc + mesa-dri-gallium

- Installed `mesa-dri-gallium` for llvmpipe software GL.
- Added `WLR_RENDER_DRM_DEVICE=/dev/dri/renderD128`.
- **Result**: Same DRM_IOCTL_MODE_CREATE_DUMB errors. The virtual GPU
  in UTM doesn't support dumb buffer allocation even with mesa.

### 4. Cage (kiosk compositor) + wayvnc + pixman

- `cage foot` with `WLR_BACKENDS=headless WLR_RENDERER=pixman`
- cage 0.2.0 from Alpine repos
- **Result**: Same buffer stride issue as sway + wayvnc + pixman.
  Foot renders at wrong size in VNC viewer.

### 5. Xvfb + x11vnc + xterm

- `Xvfb :99 -screen 0 1280x720x24`, `x11vnc`, `xterm -geometry 160x45+0+0`
- **Result**: VNC connects, xterm visible, but still not positioned/
  sized correctly. No window manager means `-maximized` doesn't work.
  Manual geometry gets close but doesn't fill screen.

### 6. Weston --backend=rdp + kiosk shell (2026-03-14)

- Weston 14.0.0 with `--backend=rdp --renderer=pixman --shell=kiosk-shell.so`
- Alpine packages: `weston`, `weston-backend-rdp`, `weston-clients`
- Auto-generated self-signed TLS cert for RDP
- Connected via `xfreerdp` from macOS
- **Result**: Weston starts, RDP listens on :3389, xfreerdp connects.
  But the RDP backend only creates `wl_seat` globals per-connected RDP
  peer — and these seats are scoped to the RDP session, not advertised
  to new Wayland clients connecting to the compositor socket. Foot
  (and any terminal) fails with "no seats available (wl_seat interface
  too old?)". `weston-simple-shm` runs (no seat needed) but no terminal
  can launch. This is a fundamental design limitation: weston's RDP
  backend is for remoting existing apps, not for hosting new ones.
- Alpine also does not package `weston-backend-vnc` (needs neatvnc/aml).

### Not Tried (Future Options)

- **Xvfb + x11vnc + twm/openbox + xterm**: Adding a WM to handle
  maximize/fullscreen might fix the X11 sizing issue.
- **Build weston with VNC backend from source**: VNC backend has
  different seat model than RDP — might expose seats to local clients.
  Requires building weston with `-Dbackend-vnc=true` + neatvnc + aml.
- **Build wayvnc from source**: Newer wayvnc might fix the pixman
  buffer stride issue.
- **tmux/mosh over SSH**: Skip display entirely — pure terminal
  multiplexing. Simplest possible solution for remote terminal access.
- **ttyd (web terminal)**: Terminal over HTTP/WebSocket. No VNC needed.
  Lightweight, packages on Alpine, serves a real PTY over the network.
- **Sway on beefier hardware with real GPU**: The GLES renderer path
  would likely work if CREATE_DUMB succeeds.

## Infrastructure Issues Discovered

### 9p filesystem caching

Files created on the macOS host are not visible to running processes
on the Alpine guest due to kernel dentry caching. Workaround: use
local filesystem (`/tmp/ark-flags/`) for flag files, trigger via SSH.

### ark-watch service management

- `start-stop-daemon` matches on `/bin/sh` broadly — had to use the
  script path directly as the command.
- `do_kill` only killed the pipe PID, not child processes (sway,
  wayvnc, foot). Fixed with unconditional `killall`.
- Stale pidfiles prevent restart — need `zap` + manual cleanup.

### 7. ttyd + native WebSocket client (2026-03-14) ✅

- Installed `ttyd 1.7.7` on Alpine: `apk add ttyd`
- `ttyd -p 7681 -W /bin/sh` — serves a PTY over WebSocket
- Built `ttyd-connect` (Rust, ~150 lines, tungstenite) as a native
  macOS client. No browser needed.
- **Protocol discovery**: ttyd expects a JSON text handshake on connect
  (`{"AuthToken":"","columns":N,"rows":N}`) before spawning the shell.
  After that, binary frames with ASCII prefix bytes: `'0'` = input/output,
  `'1'` = resize. The GitHub issue (#1400) documents `0x00`/`0x01` prefix
  bytes, but the actual browser source uses `charCodeAt(0)` on the enum
  strings — so `'0'` = `0x30`, not `0x00`.
- **Result**: Full interactive shell from macOS terminal. Resize
  propagation via SIGWINCH. Clean exit. No VNC, no compositor, no GPU.

## Architecture Pivot

The display stack is no longer a Wayland compositor on the VM. Instead:

```
VM (producer)                    macOS (consumer)
─────────────                    ────────────────
command → ttyd :7681             ttyd-connect (WebSocket)
           │                           │
      PTY over WebSocket ──────→  native terminal render
```

**Three-plane model:**
- **Control plane**: envoy protocol (JSON, Unix sockets) — unchanged
- **Data plane**: envoy frames (binary, Unix sockets) — unchanged
- **Display plane**: ttyd (PTY bytes, WebSocket) — NEW

ttyd is outside envoy. The envoy client renders TUI output to a PTY,
ttyd transports that PTY remotely, and `ttyd-connect` renders it in a
native terminal. Cryptum's future role is a client-side multi-stream
terminal compositor, not a VM-side Wayland compositor.

## Conclusion

Six compositor/VNC approaches failed (attempts 1-6). The seventh
attempt succeeded by abandoning the compositor model entirely.
ttyd + native WebSocket client provides the remote display surface
needed to unblock envoy testing. Wayland compositor deferred to future
hardware with real GPU support.
