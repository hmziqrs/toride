# Ratatui Best Practices: Focus, Keyboard Shortcuts, Vim Actions, Layout, and Flex

## Dependencies

```toml
[package]
name = "my-tui"
version = "0.1.0"
edition = "2024"

[dependencies]
ratatui = "0.30"
crossterm = "0.29"
color-eyre = "0.6"
textwrap = "0.16"

# For apps with multiple panes / component-app structure:
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## 1) Focus Model in Ratatui

Ratatui has no built-in focus system. Model focus explicitly in app state.

```rust
#[derive(Copy, Clone, PartialEq)]
enum Pane { Sidebar, Main, Footer }

#[derive(Default, Copy, Clone, PartialEq)]
enum Mode {
    #[default]
    Normal,
    Insert,
    Visual,
    Command,
}

struct App {
    focused_pane: Pane,
    mode: Mode,
    should_quit: bool,
    sidebar_items: Vec<String>,
    sidebar_state: ListState,
    main_state: TableState,
}
```

- Route key events based on `focused_pane` + `mode`.
- Only mutate the active widget's state object on navigation keys.
- Keep per-widget state (`ListState`, `TableState`) in `App`, not inside `draw()`.

## 2) Keyboard Shortcuts Architecture

Use a typed action enum as a semantic layer between raw keys and state mutations.

```rust
#[derive(Copy, Clone)]
enum Action {
    Quit,
    MoveUp,
    MoveDown,
    FocusNext,
    Select,
    EnterInsert,
    Escape,
}

fn map_key_to_action(mode: Mode, pane: Pane, key: KeyCode) -> Option<Action> {
    match (mode, pane, key) {
        (Mode::Normal, _, KeyCode::Char('q')) => Some(Action::Quit),
        (Mode::Normal, _, KeyCode::Char('i')) => Some(Action::EnterInsert),
        (Mode::Normal, _, KeyCode::Char('j') | KeyCode::Down) => Some(Action::MoveDown),
        (Mode::Normal, _, KeyCode::Char('k') | KeyCode::Up) => Some(Action::MoveUp),
        (Mode::Normal, _, KeyCode::Tab) => Some(Action::FocusNext),
        // Context-sensitive: Enter only selects when focus is on Sidebar
        (Mode::Normal, Pane::Sidebar, KeyCode::Enter) => Some(Action::Select),
        (Mode::Insert, _, KeyCode::Esc) => Some(Action::Escape),
        _ => None,
    }
}
```

Recommended flow:
1. `Event::Key(KeyEvent)`
2. `map_key_to_action(app.mode, app.focused_pane, key.code)`
3. `app.update(action)`
4. Rerender

## 3) Vim-Style Modal Input

```rust
impl App {
    fn update(&mut self, action: Action) {
        match (self.mode, action) {
            (Mode::Normal, Action::EnterInsert) => self.mode = Mode::Insert,
            (Mode::Insert, Action::Escape) => self.mode = Mode::Normal,
            (Mode::Normal, Action::MoveDown) => self.move_down(),
            (Mode::Normal, Action::MoveUp) => self.move_up(),
            (Mode::Normal, Action::Quit) => self.should_quit = true,
            _ => {}
        }
    }
}
```

- Store mode in `App`; apply mode-specific keymaps in `map_key_to_action`.
- Display mode indicator in status bar (see Section 7).
- Keep mode transitions atomic — update mode, cursor, focus, and selection together.

Anti-pattern: large nested `match` trees without a keymap abstraction.

## 4) Layout and Flex Patterns

```rust
use ratatui::layout::{Constraint, Flex, Layout};

fn draw(app: &mut App, frame: &mut Frame) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]).areas(frame.area());

    let [sidebar, main] = Layout::horizontal([
        Constraint::Length(24),
        Constraint::Fill(1),
    ]).areas(body);

    // Center a fixed-width element using Flex
    let [_, _content, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(60),
        Constraint::Fill(1),
    ]).flex(Flex::Center).areas(body);

    app.draw_sidebar(frame, sidebar);
    app.draw_main(frame, main);
    app.draw_status(frame, footer);
}
```

- Use `Flex` (`Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`) to control excess space behavior.
- Keep constraints stable and data-driven for predictable resizing.

## 5) Stateful Widgets as Focus Targets

```rust
impl App {
    fn move_down(&mut self) {
        match self.focused_pane {
            Pane::Sidebar => self.sidebar_state.select_next(),
            Pane::Main => {
                let i = self.main_state.selected().map(|i| i + 1).unwrap_or(0);
                // saturating_sub prevents usize underflow panic when list is empty
                self.main_state.select(Some(i.min(self.sidebar_items.len().saturating_sub(1))));
            }
            _ => {}
        }
    }

    fn move_up(&mut self) {
        match self.focused_pane {
            Pane::Sidebar => self.sidebar_state.select_previous(),
            Pane::Main => {
                let i = self.main_state.selected().unwrap_or(0).saturating_sub(1);
                self.main_state.select(Some(i));
            }
            _ => {}
        }
    }

    fn draw_sidebar(&mut self, frame: &mut Frame, area: Rect) {
        // Avoid cloning large item lists in draw(). For large datasets:
        // let list = List::new(self.sidebar_items.iter().map(|s| s.as_str()));
        let list = List::new(self.sidebar_items.clone())
            .highlight_style(Style::new().bold().cyan());
        frame.render_stateful_widget(list, area, &mut self.sidebar_state);
    }
}
```

- Move selection/offset in update logic, not in rendering code.
- `frame.render_stateful_widget(widget, area, &mut state)` — always pass state as `&mut`.

## 6) Styling with the Stylize Trait

```rust
use ratatui::style::Stylize;

// Preferred
"NORMAL".bold().on_cyan()
"item text".dim()
"error".red().bold()
"selected".cyan()
"warning".yellow()

// Avoid
Style::default().fg(Color::White)
Style::new().add_modifier(Modifier::BOLD)
```

Color palette:
- Primary: `.cyan()`, `.green()`
- Error: `.red()`
- Warning: `.yellow()` (sparingly)
- Muted: `.dim()`, `.dark_gray()`
- Accent: `.magenta()`

## 7) Status Bar

```rust
fn draw_status(&self, frame: &mut Frame, area: Rect) {
    let mode_label = match self.mode {
        Mode::Normal  => " NORMAL ".bold().on_cyan(),
        Mode::Insert  => " INSERT ".bold().on_green(),
        Mode::Visual  => " VISUAL ".bold().on_magenta(),
        Mode::Command => " COMMAND ".bold().on_yellow(),
    };

    let status = Line::from(vec![
        mode_label.into(),
        format!(" {} items ", self.sidebar_items.len()).dim().into(),
    ]);
    frame.render_widget(Paragraph::new(status), area);
}
```

## 8) Key Bindings Display

```rust
let help = Line::from(vec![
    " q ".bold().cyan(),
    "quit ".dim(),
    " ↑↓ ".bold().cyan(),
    "navigate ".dim(),
    " Tab ".bold().cyan(),
    "focus ".dim(),
]);
frame.render_widget(Paragraph::new(help), footer_area);
```

## 9) Text Wrapping

```rust
use textwrap::wrap;
use ratatui::text::Line;

let wrapped: Vec<Line> = wrap(&long_text, area.width as usize)
    .into_iter()
    .map(|cow| Line::from(cow.into_owned()))
    .collect();
frame.render_widget(Paragraph::new(wrapped), area);
```

## 10) Centered Popup

```rust
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, center, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).areas(area);

    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).areas(center);

    center
}
```

## 11) Template Guidance

For apps with focus, modes, and multiple panes, use the `component-app` template:

```bash
cp -r ~/.agents/skills/ratatui-tui-blacktop/assets/templates/component-app/* .
# or with pedronauck skill:
cp -r ~/.claude/skills/ratatui-tui-pedronauck/assets/templates/component-app/* .
```

Structure:
- `app.rs` — `App` state, `update()` logic
- `action.rs` — `Action` enum (place `Action` and `Mode` here)
- `event.rs` — event handling, `map_key_to_action`
- `ui.rs` — all rendering
- `tui.rs` — terminal setup/teardown

### Modern Terminal Init (ratatui 0.29+)

Prefer the ratatui convenience helpers over manual crossterm calls:

```rust
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();       // sets raw mode + alternate screen
    let result = run(&mut terminal).await;
    ratatui::restore();                        // always restores, even on panic
    result
}
```

`ratatui::init()` / `ratatui::restore()` are equivalent to the full crossterm setup/teardown block but require no manual panic hook. Use the manual crossterm approach only when you need fine-grained control over the hook.

### Logging in TUI Apps

`println!` and `eprintln!` are broken while in alternate screen / raw mode — output is invisible or corrupts the display. Use `tracing` with a file appender instead:

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let file = std::fs::File::create("/tmp/my-tui.log").unwrap();
    fmt()
        .with_writer(file)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
// Then: tracing::debug!("pane = {:?}", app.focused_pane);
```

Anti-pattern: `println!("{:?}", state)` inside a raw-mode loop — use `tracing::debug!` or `tracing::trace!` instead.

## 12) Component Trait (Large Apps)

When multiple panes grow independently, encapsulate each behind a `Component` trait:

```rust
use crossterm::event::KeyEvent;

pub enum EventResult {
    Consumed,
    Ignored,
    Action(Action),
}

pub trait Component {
    fn handle_key(&mut self, key: KeyEvent) -> EventResult;
    fn render(&self, frame: &mut Frame, area: Rect);
    fn focus(&mut self) {}
    fn blur(&mut self) {}
}
```

Usage in `App::update`:

```rust
let result = match self.focused_pane {
    Pane::Sidebar => self.sidebar.handle_key(key),
    Pane::Main    => self.main.handle_key(key),
    Pane::Footer  => EventResult::Ignored,
};
if let EventResult::Action(action) = result {
    self.update(action);
}
```

- Each component owns its `ListState`/`TableState` and focus flag.
- Parent routes keys; components return `Action` for app-level effects.
- Prefer this over a single giant `match (pane, mode, key)` once panes have 5+ bindings each.

## 13) Testing Targets

- Focus routing across panes/widgets.
- Mode switching correctness (Normal/Insert/etc.).
- Shortcut conflicts and precedence.
- Layout behavior under terminal resize.
- Stateful widget selection persistence after redraw.

## Primary Sources

- Layout Concepts: <https://ratatui.rs/concepts/layout/>
- Flex enum docs: <https://docs.rs/ratatui/latest/ratatui/layout/enum.Flex.html>
- Layout examples: <https://ratatui.rs/examples/layout/>
- Flex example: <https://ratatui.rs/examples/layout/flex/>
- Event handling concepts: <https://ratatui.rs/concepts/event-handling/>
- Widgets and StatefulWidget docs: <https://docs.rs/ratatui/latest/ratatui/widgets/>
- `StatefulWidget` trait: <https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html>
- `ListState`: <https://docs.rs/ratatui/latest/ratatui/widgets/struct.ListState.html>
- `TableState`: <https://docs.rs/ratatui/latest/ratatui/widgets/struct.TableState.html>
