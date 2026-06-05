//! SSH management content area.
//!
//! Renders inside the dashboard's content region when [`Section::Ssh`](crate::data::Section)
//! is the active sidebar section. Provides a horizontal sub-tab bar for each SSH
//! subsystem (Keys, Known Hosts, Config, Agent, Forwarding, Diagnostics) and
//! delegates rendering and input handling to the active tab.

use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::action::Action;
use crate::data::SshSection;
use crate::ui::theme::Palette;
use crate::ui::widgets::render_panel;

use self::keys_tab::KeysTab;

pub mod keys_tab;

// ── Focus ────────────────────────────────────────────────────────────────────

/// Which region currently has keyboard focus within the SSH content area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    /// The sub-tab bar at the top.
    TabBar,
    /// The main content list below the tab bar.
    List,
}

// ── SshContent ───────────────────────────────────────────────────────────────

/// SSH management content rendered inside the dashboard content area.
///
/// Owns all sub-tab state and delegates rendering/input to the active tab.
pub struct SshContent {
    /// Currently active sub-tab.
    tab: SshSection,
    /// Which region has keyboard focus.
    focus: Focus,
    /// Keys sub-tab state.
    keys: KeysTab,
}

impl SshContent {
    /// Create a new SSH content area with default state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tab: SshSection::Keys,
            focus: Focus::List,
            keys: KeysTab::new(),
        }
    }

    /// Currently active sub-tab.
    #[must_use]
    pub fn tab(&self) -> SshSection {
        self.tab
    }

    /// Provide live SSH key data (called from the data collector).
    pub fn set_keys(&mut self, keys: Vec<SshKeyEntry>) {
        self.keys.set_keys(keys);
    }

    // ── Input handling ──────────────────────────────────────────────────────

    /// Handle a key press. Returns `Some(Action)` for navigation, `None` if consumed.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<Action> {
        // If the active tab has a modal open, route input there first.
        if self.keys.has_modal() {
            return self.keys.handle_key(code);
        }

        match self.focus {
            Focus::TabBar => self.handle_tab_bar_key(code),
            Focus::List => self.handle_list_key(code),
        }
    }

    fn handle_tab_bar_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.tab = self.tab.prev();
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.tab = self.tab.next();
                None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.focus = Focus::List;
                None
            }
            KeyCode::Enter => {
                self.focus = Focus::List;
                None
            }
            KeyCode::Esc => return Some(Action::Back),
            _ => None,
        }
    }

    fn handle_list_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                // Delegate up/down to the active tab's handle_key
                self.active_tab_mut().handle_key(code)
            }
            KeyCode::Tab => {
                self.focus = Focus::TabBar;
                None
            }
            KeyCode::Esc => return Some(Action::Back),
            // Delegate remaining keys to the active tab.
            _ => self.active_tab_mut().handle_key(code),
        }
    }

    /// Handle a mouse event.
    pub fn handle_mouse(&mut self, _mouse: MouseEvent) -> Option<Action> {
        // TODO: mouse support for tab bar clicks and list item clicks
        None
    }

    fn active_tab(&self) -> &dyn SshTab {
        match self.tab {
            SshSection::Keys => &self.keys,
            SshSection::KnownHosts => &self.keys, // placeholder
            SshSection::Config => &self.keys,     // placeholder
            SshSection::Agent => &self.keys,      // placeholder
            SshSection::Forwarding => &self.keys, // placeholder
            SshSection::Diagnostics => &self.keys, // placeholder
        }
    }

    fn active_tab_mut(&mut self) -> &mut dyn SshTab {
        match self.tab {
            SshSection::Keys => &mut self.keys,
            SshSection::KnownHosts => &mut self.keys, // placeholder
            SshSection::Config => &mut self.keys,     // placeholder
            SshSection::Agent => &mut self.keys,      // placeholder
            SshSection::Forwarding => &mut self.keys, // placeholder
            SshSection::Diagnostics => &mut self.keys, // placeholder
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    /// Render the full SSH content area.
    pub fn view(&mut self, frame: &mut Frame, area: Rect, p: Palette) {
        let inner = render_panel(frame, area, None, p.text, p.border, p.bg);

        // Split into tab bar + content area
        let [tab_bar_area, _, content_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);

        self.render_tab_bar(frame, tab_bar_area, p);
        self.render_content(frame, content_area, p);
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect, p: Palette) {
        let tabs = SshSection::all();
        let mut spans: Vec<Span> = Vec::new();

        for (i, tab) in tabs.iter().enumerate() {
            let is_active = *tab == self.tab;
            let is_focused = self.focus == Focus::TabBar && is_active;

            if i > 0 {
                spans.push(Span::styled("  ", Style::default()));
            }

            let style = if is_active && is_focused {
                Style::new()
                    .fg(p.bg)
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::new()
                    .fg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(p.text_dim)
            };

            spans.push(Span::styled(format!(" {} ", tab.label()), style));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_content(&mut self, frame: &mut Frame, area: Rect, p: Palette) {
        match self.tab {
            SshSection::Keys => self.keys.view(frame, area, p),
            SshSection::KnownHosts
            | SshSection::Config
            | SshSection::Agent
            | SshSection::Forwarding
            | SshSection::Diagnostics => {
                // Placeholder for unimplemented tabs
                let msg = Line::from(vec![
                    Span::styled(self.tab.label(), Style::new().fg(p.accent).bold()),
                    Span::styled(" tab — coming in next phase", Style::new().fg(p.text_dim)),
                ]);
                let centered = Rect::new(area.x, area.y + area.height / 2, area.width, 1);
                frame.render_widget(Paragraph::new(msg).centered(), centered);
            }
        }
    }
}

impl Default for SshContent {
    fn default() -> Self {
        Self::new()
    }
}

// ── SshTab trait ─────────────────────────────────────────────────────────────

/// Interface shared by all SSH sub-tabs.
trait SshTab {
    /// Handle a tab-specific key press (including scroll).
    fn handle_key(&mut self, code: KeyCode) -> Option<Action>;
    /// Render the tab content.
    fn view(&mut self, frame: &mut Frame, area: Rect, p: Palette);
}

// ── SshKeyEntry ──────────────────────────────────────────────────────────────

/// Lightweight presentation model for an SSH key row in the Keys tab.
#[derive(Clone, Debug)]
pub struct SshKeyEntry {
    /// File name (e.g. "id_ed25519").
    pub name: String,
    /// Key type label (e.g. "Ed25519", "RSA 4096").
    pub key_type: String,
    /// SHA-256 fingerprint (truncated for display).
    pub fingerprint: String,
    /// Whether the private key is passphrase-encrypted.
    pub encrypted: bool,
    /// Octal permissions string (e.g. "0600").
    pub permissions: String,
    /// Whether the public key file (.pub) exists.
    pub has_public: bool,
    /// Whether a certificate is associated.
    pub has_cert: bool,
    /// Number of config hosts referencing this key.
    pub host_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_keys_tab() {
        let content = SshContent::new();
        assert_eq!(content.tab(), SshSection::Keys);
    }

    #[test]
    fn default_matches_new() {
        let from_new = SshContent::new();
        let from_default = SshContent::default();
        assert_eq!(from_new.tab(), from_default.tab());
    }

    #[test]
    fn render_snapshot() {
        use crate::ui::theme::CHARM;
        use ratatui::{Terminal, backend::TestBackend};

        let mut content = SshContent::new();
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        terminal
            .draw(|f| content.view(f, f.area(), CHARM))
            .unwrap();
        let output = terminal.backend().to_string();
        assert!(output.contains("Keys"), "tab bar visible: {output}");
    }

    #[test]
    fn render_snapshot_with_keys() {
        use crate::ui::theme::CHARM;
        use ratatui::{Terminal, backend::TestBackend};

        let mut content = SshContent::new();
        content.set_keys(vec![
            SshKeyEntry {
                name: "id_ed25519".into(),
                key_type: "Ed25519".into(),
                fingerprint: "SHA256:abc123".into(),
                encrypted: true,
                permissions: "0600".into(),
                has_public: true,
                has_cert: false,
                host_count: 2,
            },
        ]);
        let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
        terminal
            .draw(|f| content.view(f, f.area(), CHARM))
            .unwrap();
        let output = terminal.backend().to_string();
        assert!(output.contains("id_ed25519"), "key name visible: {output}");
    }

    #[test]
    fn tab_cycling_left_right() {
        let mut content = SshContent::new();
        content.focus = Focus::TabBar;
        assert_eq!(content.tab(), SshSection::Keys);
        content.handle_key(KeyCode::Right);
        assert_eq!(content.tab(), SshSection::KnownHosts);
        content.handle_key(KeyCode::Right);
        assert_eq!(content.tab(), SshSection::Config);
        content.handle_key(KeyCode::Left);
        assert_eq!(content.tab(), SshSection::KnownHosts);
    }

    #[test]
    fn tab_bar_to_list_on_down() {
        let mut content = SshContent::new();
        content.focus = Focus::TabBar;
        content.handle_key(KeyCode::Down);
        assert_eq!(content.focus, Focus::List);
    }

    #[test]
    fn list_to_tab_bar_on_tab() {
        let mut content = SshContent::new();
        content.focus = Focus::List;
        content.handle_key(KeyCode::Tab);
        assert_eq!(content.focus, Focus::TabBar);
    }
}
