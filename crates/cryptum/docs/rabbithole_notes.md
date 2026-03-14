alpine
wlroots (wayland)
smithay (rust version of wlroots)


To help you gauge the depth of this rabbit hole, I’ve categorized these resources by their role in your project. If you're building with **Rust** on **Alpine Linux**, these are the definitive "maps" for the journey.

---

## 1. The Core Infrastructure (Alpine & Wayland)

Before writing code, you need to understand how Alpine handles graphics and how the Wayland protocol actually "talks."

* **[Alpine Linux Wayland Wiki](https://wiki.alpinelinux.org/wiki/Wayland):** Start here to understand the Alpine-specific setup for seat management (`seatd`) and the necessary environment variables like `XDG_RUNTIME_DIR`.
* **[The Wayland Protocol Book](https://wayland-book.com/):** Written by Drew DeVault (the creator of Sway), this is the "missing manual" for how Wayland works conceptually. It explains why there is no "drawing API" and how buffers move between apps.
* **[Wayland Protocol Documentation (Official)](https://wayland.freedesktop.org/docs/html/ch04.html):** A deeper technical dive into the message-based architecture (Requests vs. Events) and the object-oriented nature of the protocol.

---

## 2. The Rust Framework (Smithay)

Smithay is the library that makes building a compositor in Rust feasible without being a driver expert.

* **[The Smithay Book](https://smithay.github.io/smithay/):** This is the primary guide for the library. It explains the "backend" (talking to hardware) versus the "wayland" (talking to apps) modules.
* **[Anvil (The Smithay Reference Compositor)](https://www.google.com/search?q=https://github.com/Smithay/smithay/tree/master/anvil):** This is the single most important repo for you. Anvil is a functional compositor built using Smithay. You can read its source code to see exactly how a "State" struct is managed in Rust.
* **[Smithay Documentation (Docs.rs)](https://docs.rs/smithay/latest/smithay/):** Detailed API reference. Specifically, look at `smithay::backend` for hardware abstraction and `smithay::desktop` for window management helpers.

---

## 3. Practical "Getting Started" Guides

If you want to see someone actually build the skeleton of a compositor, these community guides are excellent:

* **[Making a Wayland Compositor - Part 1](https://aftix.xyz/wayland1/):** A very modern (2025) blog series focusing on building a compositor with Smithay from the ground up.
* **[Building a Compositor in Rust (Video Walkthrough)](https://www.youtube.com/watch?v=BkETJWMz0f0):** A visual overview of the event loop, damage tracking, and rendering logic using Smithay.

---

## 4. Historical & Comparative Context

Sometimes seeing the "minimal" C version helps you understand what the Rust abstractions are doing for you.

* **[TinyWL (C Source Code)](https://github.com/swaywm/wlroots/tree/master/tinywl):** The absolute minimal Wayland compositor (~1,000 lines of C). If you can follow this, the Rust version will make much more sense.
* **[Writing a Wayland Compositor in Rust (Alopex Wiki)](https://wiki.alopex.li/WritingAWaylandCompositorInRust):** A high-level discussion on the complexity of Wayland versus X11 and the history of Rust's early attempts at compositors.

### Quick Tip for Alpine Users

When you're ready to start compiling, you'll need a specific set of build dependencies. On Alpine, that looks like:
`apk add rust cargo wayland-dev libinput-dev mesa-dev libxkbcommon-dev seatd-dev`

**Would you like me to generate a "Minimal Viable Product" checklist for your first weekend of development to keep you from getting lost in the protocol weeds?**

This is a "deep dive" project, but the **Rust + Smithay** stack is arguably the most efficient way to do it in 2026. Because you're targeting **Alpine**, you'll be working in a very "pure" environment where every dependency must be explicitly accounted for.

Here is your **Weekend MVP Checklist** and the **Research Dossier** to get you started.

---

## Part 1: The Research Dossier

Review these in order to understand the architecture before you write a single line of Rust:

### 1. The "Conceptual" Layer

* **[The Wayland Protocol Book](https://wayland-book.com/):** Essential reading. Explains the "message-passing" nature of Wayland (e.g., why the compositor doesn't "draw" for the app, but rather manages buffers).
* **[Wayland vs. X11 for Developers](https://smithay.github.io/smithay-v-0-1.html):** A high-level overview of why the "Compositor is the Server" model matters for your Rust code.

### 2. The "Framework" Layer (Smithay)

* **[The Smithay Book](https://smithay.github.io/book/):** The official guide. Pay close attention to the **"Seat"** and **"Space"** concepts.
* **[Anvil Source Code](https://www.google.com/search?q=https://github.com/Smithay/smithay/tree/master/anvil):** This is your Rosetta Stone. Anvil is the reference compositor. If you get stuck on how to handle a mouse click, search the Anvil repo.

### 3. The "Base OS" Layer (Alpine)

* **[Alpine Wayland Wiki](https://wiki.alpinelinux.org/wiki/Wayland):** Specifically the section on `seatd`. Since you aren't using a heavy Desktop Environment, you need `seatd` to give your Rust binary permission to touch the GPU.

---

## Part 2: The "First Weekend" MVP Checklist

Don't try to build a full OS interface on day one. Follow this "Nested" development strategy:

### Phase 1: The "Nested" Environment (Saturday Morning)

*Instead of booting into a black screen, run your compositor as a window **inside** your current OS. This lets you debug with a browser open next to it.*

1. **Setup Alpine Build-Deps:**
```bash
apk add rust cargo wayland-dev libinput-dev mesa-dev libxkbcommon-dev seatd-dev build-base

```


2. **Initialize Cargo:** Create a new project and add `smithay` with the `backend_winit` feature.
3. **The "Winit" Goal:** Get a blank window to pop up on your current desktop that is being managed by your Rust code. If you can move a virtual cursor inside that window, you’ve passed Step 1.

### Phase 2: The "Protocol" Handshake (Saturday Afternoon)

*Make your compositor talk to a terminal.*

1. **Implement `CompositorHandler`:** This is the delegate that tells Wayland: "I am ready to accept surfaces."
2. **Launch `foot`:** Run `WAYLAND_DISPLAY=wayland-1 foot` from a separate terminal.
3. **The "Surface" Goal:** Your Rust code should log: *"New surface created!"* even if it doesn't render the pixels yet. This proves your state machine is working.

### Phase 3: The "Rendering" Loop (Sunday Morning)

*Actually see the terminal pixels.*

1. **Setup the Renderer:** Use Smithay’s `GlesRenderer`.
2. **The "Space" Goal:** Map the terminal's buffer into a `Space`. When you run `foot`, it should actually appear inside your nested window.

### Phase 4: The "Bare Metal" TTY (Sunday Afternoon)

*The moment of truth.*

1. **Switch Backends:** Swap the `winit` backend for the `drm` (Direct Rendering Manager) backend.
2. **TTY Launch:** Switch to a raw TTY (`Ctrl+Alt+F3`), run your binary, and see if it takes over your entire monitor.
3. **The "Minimalist" Goal:** Have your code automatically spawn `foot` on startup. If you see a terminal on a black screen with no other "bloat," you have successfully built your interface.

---

## Recommended Next Step

Would you like me to generate a **minimal `main.rs**` that uses the `Winit` backend? This would give you a "ready-to-compile" starting point so you can spend your weekend on the logic instead of the boilerplate.

This is your **Starting Line**. To make this weekend productive, we will use the **Winit backend**. This allows your compositor to run as a window inside your current OS, so you don't have to constantly reboot or switch TTYs while you're debugging your logic.

### 1. The Build Environment (Alpine)

Run this on your Alpine machine to ensure you have the C-libraries Rust needs to link against:

```bash
apk add rust cargo wayland-dev libinput-dev mesa-dev libxkbcommon-dev seatd-dev build-base

```

---

### 2. `Cargo.toml`

Create a new project with `cargo init` and replace your `Cargo.toml` dependencies with these. Note the specific features—Smithay is modular, so we only pull in what we need for a terminal-centric GUI.

```toml
[package]
name = "my-interface"
version = "0.1.0"
edition = "2021"

[dependencies]
# The core engine
smithay = { version = "0.3", features = ["backend_winit", "renderer_glow", "desktop", "wayland_frontend"] }

# Wayland server bindings
wayland-server = "0.31"

# Event loop management
calloop = "0.12"

# Logging (vital for seeing why a window didn't open)
tracing = "0.1"
tracing-subscriber = "0.3"

```

---

### 3. `main.rs` (The Skeleton)

This code does three things: it sets up a Wayland "Display," initializes a graphical window (via Winit), and starts an infinite loop that waits for events.

```rust
use smithay::{
    backend::winit::{self, WinitEvent},
    reexports::wayland_server::Display,
    wayland::compositor::CompositorState,
};
use std::time::Duration;

struct MyState {
    display_handle: smithay::reexports::wayland_server::DisplayHandle,
    compositor_state: CompositorState,
    // Add your window list here later
}

fn main() {
    // 1. Initialize Logging
    tracing_subscriber::fmt().init();

    // 2. Create the Wayland Display
    let mut display: Display<MyState> = Display::new().unwrap();
    let dh = display.handle();

    // 3. Initialize Smithay Components (The "Globals")
    let compositor_state = CompositorState::new::<MyState>(&dh);

    let mut state = MyState {
        display_handle: dh,
        compositor_state,
    };

    // 4. Initialize the Winit Backend (Our "Monitor")
    let (mut backend, mut event_loop) = winit::init().expect("Failed to init Winit");

    println!("Compositor started. WAYLAND_DISPLAY=wayland-1");

    // 5. The Event Loop
    loop {
        // Handle events from Winit (mouse, keyboard, window resize)
        event_loop.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                println!("Window resized to {:?}", size);
            }
            WinitEvent::Input(input_event) => {
                // This is where you'll handle keys like Super+T
            }
            WinitEvent::Redraw => {
                // This is where the magic happens: rendering the windows
                let _ = backend.bind().map(|(renderer, _framebuffer)| {
                    renderer.clear([0.1, 0.1, 0.1, 1.0], [].into()).unwrap();
                });
                backend.submit(None).unwrap();
            }
            _ => (),
        });

        // Send updates to Wayland clients (apps like 'foot')
        display.flush_clients().unwrap();
        
        // Keep the CPU from melting (16ms = ~60fps)
        std::thread::sleep(Duration::from_millis(16));
    }
}

// Required boilerplate: telling Smithay how to handle the compositor global
impl smithay::wayland::compositor::CompositorHandler for MyState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    fn commit(&mut self, _surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        // This triggers when a window (like a terminal) has a new frame ready
    }
}

// Delegate macros: These connect Wayland protocol events to your MyState
smithay::delegate_compositor!(MyState);

```

---

### What happens when you run this?

1. **A Window Opens:** You'll see a dark grey window. This is your "monitor."
2. **A Server Starts:** In the background, your code is now a living Wayland server.
3. **The Test:** Open a second terminal and run `WAYLAND_DISPLAY=wayland-1 foot`.
* *Result:* `foot` will connect to your Rust code. Your code will log a "Commit," but nothing will appear on the screen yet because we haven't written the "Drawing" logic (Phase 3 of your weekend).



### Why this is the "Right" Rabbit Hole

Notice that you didn't have to write a single line of code to talk to the Nvidia/AMD/Intel driver. `smithay::backend::winit` handles that. You are free to focus entirely on the **State**—the logic of how your interface should feel.

**Would you like me to explain how to implement the `Space` concept so that the terminal actually appears inside your grey window?**

To get that terminal actually rendering inside your window, we need to introduce the **`Space`**.

In Smithay, a `Space` is a coordinate system that manages your "Windows" (Surfaces). Think of it as the stage, and the windows are the actors. To see the terminal, you need to map the Wayland surface into the `Space` and then tell the renderer to draw that `Space`.

### 1. Update your `MyState` Struct

You need to add a few pieces of "glue" to track the windows and the shell (the protocol that handles desktop-style windows).

```rust
use smithay::desktop::{Space, Window};
use smithay::wayland::shell::xdg::XdgShellState;

struct MyState {
    display_handle: smithay::reexports::wayland_server::DisplayHandle,
    compositor_state: CompositorState,
    
    // The "Stage" where windows live
    space: Space<Window>,
    
    // XDG Shell handles standard window behavior (map, resize, etc.)
    xdg_shell_state: XdgShellState,
}

```

---

### 2. Implement the XDG Shell Handler

When you run `foot`, it sends an "XDG Shell" request to your compositor. You have to tell Rust what to do when a new window appears.

Add this "Delegate" logic to your file:

```rust
use smithay::wayland::shell::xdg::{XdgShellHandler, ToplevelSurface};
use smithay::delegate_xdg_shell;

impl XdgShellHandler for MyState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // This is called when 'foot' starts!
        let window = Window::new_wayland_window(surface);
        
        // Place the window at (0, 0) in our Space
        self.space.map_element(window, (0, 0), true);
        
        println!("A new terminal window has been mapped to the space!");
    }

    fn new_popup(&mut self, _surface: smithay::wayland::shell::xdg::PopupSurface) {}
    fn grab(&mut self, _surface: smithay::wayland::shell::xdg::PopupSurface, _seat: smithay::input::Seat<Self>, _serial: smithay::utils::Serial) {}
}

delegate_xdg_shell!(MyState);

```

---

### 3. The Rendering Logic (The "Magic" Part)

Inside your main loop, where you had `renderer.clear(...)`, you now need to tell the renderer to draw everything in the `Space`.

```rust
// Inside your loop, under WinitEvent::Redraw:
let _ = backend.bind().map(|(renderer, _framebuffer)| {
    // 1. Clear the background to a dark grey
    renderer.clear([0.1, 0.1, 0.1, 1.0], [].into()).unwrap();

    // 2. Draw the windows in the space
    // 'elements' are your windows. This handles all the scaling and buffer math.
    smithay::desktop::space::render_output::<_,  _>(
        &self.space,
        renderer,
        1.0, // Scale
        0,   // Rotation
        [],  // Damage (for now, redraw everything)
    ).unwrap();
});

```

---

### 4. Why this matters for your "Lightweight" goal

By using the `Space` abstraction:

* **Tiling is easy:** If you want a tiling interface, you don't move "pixels"; you just update the coordinates of the `Window` inside the `Space` (e.g., change `(0,0)` to `(800, 0)`).
* **Zero Bloat:** You are only rendering the windows you actually care about. There are no "title bars," "minimize buttons," or "decorations" unless *you* write the code to draw them.

### What happens now?

1. **Run your compositor.**
2. **Run `WAYLAND_DISPLAY=wayland-1 foot**` in another terminal.
3. **Result:** The `foot` terminal window should appear inside your Rust window! It might not be interactive yet (we haven't mapped the keyboard input to the window), but you are now officially rendering a GUI from scratch.

### Your Final "Weekend Step"

The biggest hurdle in Wayland is **Input Dispatching**. Right now, your keyboard is talking to the OS, not your Rust window.

**Would you like the code snippet to "seat" your keyboard so that you can actually type into the terminal you just rendered?**

This is the final "wiring" step. To make your terminal interactive, you need to implement a **Seat**. In Wayland terminology, a "Seat" is a group of input devices (one keyboard, one pointer/mouse, one touch device) assigned to a user.

When you press a key, your code needs to:

1. Capture the event from the hardware (via `winit`).
2. Figure out which window has focus.
3. Send that keypress to that window's buffer.

---

### 1. Update `MyState` with the Seat

You need to add a `Seat` and a `SeatState` to track who is typing where.

```rust
use smithay::input::{Seat, SeatState, SeatHandler};
use smithay::input::keyboard::KeyboardTarget;

struct MyState {
    display_handle: smithay::reexports::wayland_server::DisplayHandle,
    compositor_state: CompositorState,
    space: Space<Window>,
    xdg_shell_state: XdgShellState,
    
    // The Input handling pieces
    seat_state: SeatState<Self>,
    seat: Seat<Self>,
}

```

---

### 2. Implement the Seat Handler

This tells Smithay how to manage input "globals" (the stuff the OS needs to know about your keyboard).

```rust
impl SeatHandler for MyState {
    type KeyboardFocus = Window; // Focus is always on a window
    type PointerFocus = Window;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

smithay::delegate_seat!(MyState);

```

---

### 3. Wiring the Keyboard in the Loop

Now, go back to your `main` function's event loop. You need to catch the `WinitEvent::Input` and pass it to the seat.

```rust
// Inside the main loop:
WinitEvent::Input(input_event) => {
    let dh = &state.display_handle;
    
    // 1. Get the keyboard handle from our seat
    let keyboard = state.seat.get_keyboard().unwrap();

    // 2. Map the Winit input to Smithay's internal input format
    // (This handles the 'Shift', 'Ctrl', and 'Alt' state automatically)
    keyboard.input::<(), _>(state, |state, modifiers, handle| {
        // Here you can check for hotkeys, like Super+Q to quit
        // If it's not a hotkey, it gets sent to the focused window
        smithay::input::keyboard::FilterResult::Forward
    });
}

```

---

### 4. Giving the Terminal "Focus"

By default, your terminal won't receive keys until you tell the compositor it has "focus." Add this line inside your `new_toplevel` function (from the previous step):

```rust
fn new_toplevel(&mut self, surface: ToplevelSurface) {
    let window = Window::new_wayland_window(surface);
    self.space.map_element(window.clone(), (0, 0), true);

    // CRITICAL: Give this new window the keyboard focus!
    let keyboard = self.seat.get_keyboard().unwrap();
    keyboard.set_focus(self, Some(window), smithay::utils::Serial::from(0));
}

```

---

### The Result: A Functional Shell

If you compile and run this now:

1. Your grey window opens.
2. You run `WAYLAND_DISPLAY=wayland-1 foot` in another terminal.
3. `foot` appears inside your Rust window.
4. **You can type!** You can run `ls`, `top`, or even `vim` inside that nested terminal.

### Why this is the "Lightweight" Dream

You now have a 100% custom operating system interface. It doesn't have a taskbar because you didn't code one. It doesn't have background telemetry because you didn't include it.

**What's left?**

* **Tiling Math:** Instead of just mapping every window to `(0,0)`, you can write a Rust function that calculates the width of the screen and divides it by the number of windows.
* **Status Bar:** You could use the `renderer` to draw a simple colored bar at the top with the time or CPU usage.

**How are you feeling about the Rust boilerplate?** If it feels manageable, your next move is to check out the **[Smithay "Anvil" Example](https://www.google.com/search?q=https://github.com/Smithay/smithay/tree/master/anvil)** source code—it’s the "final form" of what we just built, including professional-grade error handling and multi-monitor support.

Would you like me to point out the specific file in the Anvil repo that handles the "Tiling" logic so you can see how to organize your windows?

To see how a "real" compositor handles the transition from a single window to a tiling layout, you should look at the **`handlers`** and **`state`** modules in the Anvil source code.

Specifically, the "Tiling Logic" in Smithay isn't usually a single file, but a combination of how you manage your `Space` and how you respond to `map` events.

---

### The Key File to Study

In the [Smithay Anvil Repository](https://www.google.com/search?q=https://github.com/Smithay/smithay/tree/master/anvil/src), look for:

* **`state.rs`**: This is where the "Global State" lives. Look for how they store the `Space`.
* **`shell/xdg.rs`**: This is where the `new_toplevel` function lives. In Anvil, instead of just placing a window at `(0,0)`, they call a function to find the next available spot.

---

### How to Implement "Automatic Tiling"

If you want to move from "floating" to "tiling," you change your `new_toplevel` logic. Instead of hardcoding `(0,0)`, you calculate the geometry.

**The "Simple Tiling" Logic:**

```rust
fn new_toplevel(&mut self, surface: ToplevelSurface) {
    let window = Window::new_wayland_window(surface);
    
    // 1. Count existing windows
    let window_count = self.space.elements().count();
    
    // 2. Simple Horizontal Split Logic
    // If it's the 1st window: Full Screen (0, 0)
    // If it's the 2nd window: Right Half (width/2, 0)
    let screen_width = 1920; // You'd get this from your output geometry
    let x_pos = if window_count == 0 { 0 } else { screen_width / 2 };
    
    // 3. Map it
    self.space.map_element(window.clone(), (x_pos, 0), true);
    
    // 4. Update the FIRST window's size (Resize logic)
    // You would send a 'configure' event to the first window 
    // to tell it to shrink to 50% width.
}

```

---

### The "Deep Dive" Research Checklist

As you spend your weekend with the code, keep these three Smithay concepts in your "to-learn" tab:

1. **Damage Tracking:** Right now, your code is redrawing the *entire* screen 60 times a second. That's fine for a prototype, but for "lightweight" efficiency, you only want to redraw the pixels that changed (e.g., the cursor moving or a character being typed). Look for `DamageTracker` in the Smithay docs.
2. **Compositor Modifiers:** Learn how `ModifiersState` works so you can implement `Super + Shift + Q` to kill a window or `Super + Enter` to spawn a new terminal.
3. **The Render Loop (`calloop`):** Smithay uses a crate called `calloop`. It’s a specialized event loop for Unix systems. Understanding how it handles timers and sockets will make your interface feel much more responsive.

---

### Final Advice for the Journey

Building an interface is a cycle of **State -> Event -> Render**.

* **State:** I have two terminals open.
* **Event:** User pressed `Super + Right`.
* **Render:** Update the coordinates of Terminal B and redraw.

You are no longer a "user" of an OS; you are the **architect of the interaction**.

**Would you like me to summarize the "Next 3 Steps" for after your weekend MVP is running, or are you ready to go get your hands dirty with the compiler?**


