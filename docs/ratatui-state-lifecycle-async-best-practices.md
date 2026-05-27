# Ratatui Best Practices: State Management, Lifecycle, and Async

## Dependencies

```toml
[package]
name = "my-tui"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
color-eyre = "0.6"
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
futures = "0.3"
textwrap = "0.16"
image = "0.25"
reqwest = { version = "1", features = ["json"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Optional: image support
ratatui-image = { version = "5", features = ["chafa-static"] }
```

## 1) State Management Model (Core Pattern)

Ratatui is render-focused and intentionally does not provide a full app framework. Treat your app as:

1. `App` state struct (domain + UI state)
2. `update()` (event/action → state mutation)
3. `draw()` (state → widgets)

```rust
struct App {
    items: Vec<String>,
    list_state: ListState,
    should_quit: bool,
    dirty: bool,
    error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            dirty: true,  // force first render immediately
            items: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            error: None,
        }
    }
}

enum Action { Quit, MoveUp, MoveDown }

impl App {
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::MoveDown => self.list_state.select_next(),
            Action::MoveUp => self.list_state.select_previous(),
        }
        self.dirty = true;
    }

    fn draw(&mut self, frame: &mut Frame) {
        let list = List::new(self.items.clone())
            .highlight_style(Style::new().bold().cyan());
        frame.render_stateful_widget(list, frame.area(), &mut self.list_state);
    }
}
```

- Keep widget state (`ListState`, `TableState`, `ScrollbarState`) inside `App`, not inside `draw()`.
- Keep domain state and widget state as separate fields.
- Set `dirty = true` in `update()` so the event loop knows to redraw.

## 2) Lifecycle and Terminal Discipline

### Preferred: ratatui::init() / ratatui::restore() (ratatui 0.29+)

```rust
use color_eyre::eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = ratatui::init();   // raw mode + alternate screen + panic hook
    let result = run(&mut terminal).await;
    ratatui::restore();                    // always restores, even after panic

    result
}
```

`ratatui::init()` handles the panic hook internally. This is the recommended approach for new apps.

### Manual: crossterm setup/teardown

Use this when you need fine-grained panic hook control (e.g., logging the panic info before restoring):

```rust
use color_eyre::eyre::Result;
use crossterm::{execute, terminal};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(std::io::stdout(), terminal::LeaveAlternateScreen);
        original_hook(info);
    }));

    terminal::enable_raw_mode()?;
    execute!(std::io::stdout(), terminal::EnterAlternateScreen)?;

    let result = run().await;

    terminal::disable_raw_mode()?;
    execute!(std::io::stdout(), terminal::LeaveAlternateScreen)?;

    result
}
```

- Always call `color_eyre::install()?` first in `main()`.
- Always restore terminal on both normal exit and panic.

## 3) Async Event Architecture

Use `EventStream` from crossterm (requires `event-stream` feature) with a background task channel.

```rust
use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use ratatui::{Terminal, backend::Backend};
use tokio::{select, sync::mpsc};

async fn run(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    let mut app = App::default();
    let mut events = EventStream::new();
    // Use mpsc::unbounded_channel() if back-pressure is not a concern
    let (tx, mut rx) = mpsc::channel::<AppEvent>(32);

    loop {
        if app.dirty {
            terminal.draw(|f| app.draw(f))?;
            app.dirty = false;
        }

        select! {
            Some(Ok(event)) = events.next() => {
                if let Some(action) = map_event_to_action(&app, event) {
                    app.update(action);
                }
            }
            Some(event) = rx.recv() => {
                app.handle_event(event);
            }
        }

        if app.should_quit { break; }
    }
    Ok(())
}
```

Recommended architecture:
1. `EventStream` yields crossterm `Event`s (keyboard, mouse, resize).
2. Map `Event` → `Action` via `map_event_to_action`.
3. Background tasks send results back through an `mpsc::Receiver` in the same `select!`.
4. `terminal.draw(|f| app.draw(f))` renders only when `dirty`.

### Tick Timer (animations / polling)

Add a periodic tick arm to `select!` for animations or timed refreshes:

```rust
use tokio::time::{interval, Duration};

let mut tick = interval(Duration::from_millis(250));

select! {
    Some(Ok(event)) = events.next() => { /* key/mouse/resize */ }
    _ = tick.tick() => {
        app.on_tick();  // advance animation frames, poll external state
        app.dirty = true;
    }
    Some(event) = rx.recv() => { app.handle_event(event); }
}
```

### Debouncing Input (search-as-you-type)

```rust
use tokio::time::{Instant, Duration};

struct App {
    search_query: String,
    last_input: Option<Instant>,  // None until first keystroke
    pending_search: bool,
    // ...
}

impl App {
    fn on_search_key(&mut self, c: char) {
        self.search_query.push(c);
        self.last_input = Some(Instant::now());
        self.pending_search = true;
    }

    fn on_tick(&mut self, tx: &mpsc::Sender<AppEvent>) {
        if self.pending_search {
            if self.last_input.map_or(false, |t| t.elapsed() > Duration::from_millis(300)) {
                self.pending_search = false;
                self.spawn_search(tx.clone());
            }
        }
    }
}
```

### EventHandler Module (component-app scale)

For larger apps, extract event dispatching into its own struct so `run()` stays clean:

```rust
// event.rs
use crossterm::event::{Event, EventStream, KeyEvent};
use futures::StreamExt;
use tokio::{select, sync::mpsc, time::{sleep, Duration}};

pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    Background(BackgroundResult),
}

pub struct EventHandler {
    events: EventStream,
    tick_rate: Duration,
    rx: mpsc::Receiver<BackgroundResult>,
}

impl EventHandler {
    pub async fn next(&mut self) -> AppEvent {
        let tick = sleep(self.tick_rate);
        select! {
            Some(Ok(Event::Key(k))) = self.events.next() => AppEvent::Key(k),
            Some(Ok(Event::Resize(w, h))) = self.events.next() => AppEvent::Resize(w, h),
            Some(r) = self.rx.recv() => AppEvent::Background(r),
            _ = tick => AppEvent::Tick,
        }
    }
}
```

## 4) Background Tasks

```rust
use tokio::{select, sync::mpsc};
use tokio_util::sync::CancellationToken;

enum AppEvent { FetchDone(Result<String, String>) }

impl App {
    fn spawn_fetch(&self, tx: mpsc::Sender<AppEvent>, cancel: CancellationToken, url: String) {
        tokio::spawn(async move {
            select! {
                result = reqwest::get(&url) => {
                    let payload = result
                        .map(|_| url)
                        .map_err(|e| e.to_string());
                    let _ = tx.send(AppEvent::FetchDone(payload)).await;
                }
                _ = cancel.cancelled() => {}
            }
        });
    }

    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::FetchDone(Ok(data)) => self.items.push(data),
            AppEvent::FetchDone(Err(e)) => self.error = Some(e),
        }
        self.dirty = true;
    }
}
```

- Never block the event loop with long synchronous operations.
- Keep mutation serialized in the main update loop via channel messages.
- Use `CancellationToken` (from `tokio-util`) so tasks shut down cleanly on quit.
- Use message passing, not shared `Arc<Mutex<_>>`, between tasks.

## 5) Image Integration

```rust
use image::open as open_image;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
use std::thread;

struct App {
    picker: Picker,
    image_state: Option<StatefulProtocol>,
    // ...
}
```

### Picker initialization with fallback

```rust
use ratatui_image::picker::{Picker, ProtocolType};

fn make_picker() -> Picker {
    Picker::from_query_stdio().unwrap_or_else(|_| {
        Picker::new(ProtocolType::Halfblocks)  // safe fallback for all terminals
    })
}
```

### Background thread loading (std::thread)

```rust
// Capture area before spawning — Rect is Copy
let image_area = Rect::new(0, 0, 40, 20);
let picker = app.picker.clone();
let (tx, rx) = std::sync::mpsc::channel::<StatefulProtocol>();
thread::spawn(move || {
    let dyn_img = open_image("photo.png").unwrap();
    let protocol = picker.new_protocol(dyn_img, image_area.into(), Resize::Fit(None));
    tx.send(protocol).unwrap();
});

// In the event loop, receive when ready
if let Ok(protocol) = rx.try_recv() {
    app.image_state = Some(protocol);
    app.dirty = true;
}

// In draw(), use StatefulImage to avoid re-encoding on redraws
if let Some(ref mut img) = app.image_state {
    frame.render_stateful_widget(StatefulImage::default(), image_area, img);
}
```

### Async loading (tokio context — preferred in async apps)

Use `spawn_blocking` instead of `std::thread::spawn` when inside a tokio runtime — it integrates with tokio's task scheduler and thread pool:

```rust
use tokio::task::spawn_blocking;

async fn load_image(
    picker: Picker,
    area: Rect,
    tx: mpsc::Sender<AppEvent>,
) {
    let result = spawn_blocking(move || {
        let dyn_img = open_image("photo.png")?;
        Ok::<_, image::ImageError>(picker.new_protocol(dyn_img, area.into(), Resize::Fit(None)))
    }).await;

    match result {
        Ok(Ok(protocol)) => { tx.send(AppEvent::ImageLoaded(protocol)).await.ok(); }
        Ok(Err(e)) => { tx.send(AppEvent::Error(e.to_string())).await.ok(); }
        Err(e) => { tx.send(AppEvent::Error(e.to_string())).await.ok(); }
    }
}
```

### Re-encode on terminal resize

```rust
fn handle_resize(&mut self, new_area: Rect) {
    if self.original_image_path.is_some() {
        self.image_state = None;  // invalidate; reload with new area
        self.dirty = true;
    }
}
```

- `Rect` is `Copy` — always capture it by value before spawning.
- Query the terminal protocol once at startup; reuse `Picker` across the app lifetime.
- Store `StatefulProtocol` (not `DynamicImage`) to avoid re-encoding on redraws.
- Use `chafa-static` feature for portable binaries that don't require chafa to be installed.
- Re-encode when the terminal resizes — the existing protocol encodes for a fixed cell size.

## 6) Logging in TUI Apps

`println!` and `eprintln!` are broken while in raw mode / alternate screen — output is invisible or corrupts the display. Use `tracing` with a file writer:

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() -> color_eyre::Result<()> {
    let log_file = std::fs::File::create("/tmp/my-tui.log")?;
    fmt()
        .with_writer(log_file)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    Ok(())
}

// Call before ratatui::init():
// init_logging()?;
// tracing::debug!("app started, {} items loaded", items.len());
```

Anti-pattern: `println!("{:?}", state)` inside a running TUI loop — crashes output or shows garbage on exit.

## 7) Error Handling

```rust
use color_eyre::{eyre::Result, eyre::WrapErr};

fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read config at {}", path.display()))?;
    toml::from_str(&text).wrap_err("Failed to parse config")
}
```

- Use `color-eyre` for rich error context; install it first in `main()`.
- Convert task failures into app events (see Section 4) so the UI stays alive.
- Never leave the terminal in raw/alternate mode after an error.

## 8) Text Wrapping

```rust
use textwrap::wrap;
use ratatui::text::Line;

let wrapped: Vec<Line> = wrap(&long_text, area.width as usize)
    .into_iter()
    .map(|cow| Line::from(cow.into_owned()))
    .collect();
frame.render_widget(Paragraph::new(wrapped), area);
```

Precompute wrapped lines in `update()` when the content changes, not inside `draw()`.

## 9) Release Optimization

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
opt-level = "z"  # optimize for binary size
```

## Pre-Ship Checklist

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-features` clean
- [ ] No `unwrap()` outside tests
- [ ] `color_eyre::install()` is first call in `main()`
- [ ] `ratatui::restore()` (or manual teardown) called on all exit paths including panic
- [ ] `App::default()` sets `dirty: true` so first frame renders immediately
- [ ] All spawned tasks use `CancellationToken` and are joined on quit
- [ ] Logging uses `tracing` to a file, never `println!` in raw mode
- [ ] Image picker uses fallback (`Halfblocks`) when `from_query_stdio()` fails
- [ ] `cargo build --release` with the release profile above succeeds
- [ ] Test on target terminal(s)

## Primary Sources

- Ratatui Concepts: <https://ratatui.rs/concepts/>
- Event Handling Concepts: <https://ratatui.rs/concepts/event-handling/>
- Raw Mode: <https://ratatui.rs/concepts/backends/raw-mode/>
- Async Counter Tutorial: <https://ratatui.rs/tutorials/counter-async-app/>
- Async Event Stream: <https://ratatui.rs/tutorials/counter-async-app/async-event-stream/>
- Full Async Events: <https://ratatui.rs/tutorials/counter-async-app/full-async-events/>
- Full Async Actions: <https://ratatui.rs/tutorials/counter-async-app/full-async-actions/>
- ratatui-image crate: <https://docs.rs/ratatui-image/latest/ratatui_image/>
- Ratatui crate docs: <https://docs.rs/ratatui/latest/ratatui/>
- Widgets module (`Widget` / `StatefulWidget`): <https://docs.rs/ratatui/latest/ratatui/widgets/>
