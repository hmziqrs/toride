use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticToken {
    BgBase,
    BgRaised,
    BgOverlay,
    FgPrimary,
    FgSecondary,
    FgMuted,
    FgInverse,
    Accent,
    AccentDim,
    Success,
    Warning,
    Danger,
    Info,
    Border,
    BorderFocus,
    SelectionBg,
    SelectionFg,
    SpinnerActive,
    ProgressFill,
    ProgressTrack,
}

#[derive(Debug, Clone)]
pub struct Theme {
    colors: std::collections::HashMap<SemanticToken, Color>,
    pub unicode: bool,
    pub no_color: bool,
}

impl Theme {
    pub fn new(caps: &crate::tui::caps::TerminalCaps) -> Self {
        let no_color = caps.no_color && !caps.force_color;

        let colors = if no_color {
            Self::no_color_palette()
        } else if caps.truecolor {
            Self::dark_palette()
        } else {
            Self::dark_palette_256()
        };

        Self {
            colors,
            unicode: caps.unicode,
            no_color,
        }
    }

    fn dark_palette() -> std::collections::HashMap<SemanticToken, Color> {
        let mut m = std::collections::HashMap::new();
        m.insert(SemanticToken::BgBase, Color::Rgb(11, 14, 20));
        m.insert(SemanticToken::BgRaised, Color::Rgb(17, 21, 28));
        m.insert(SemanticToken::BgOverlay, Color::Rgb(22, 27, 34));
        m.insert(SemanticToken::FgPrimary, Color::Rgb(230, 237, 243));
        m.insert(SemanticToken::FgSecondary, Color::Rgb(177, 186, 196));
        m.insert(SemanticToken::FgMuted, Color::Rgb(110, 118, 129));
        m.insert(SemanticToken::FgInverse, Color::Rgb(11, 14, 20));
        m.insert(SemanticToken::Accent, Color::Rgb(122, 162, 247));
        m.insert(SemanticToken::AccentDim, Color::Rgb(61, 74, 107));
        m.insert(SemanticToken::Success, Color::Rgb(158, 206, 106));
        m.insert(SemanticToken::Warning, Color::Rgb(224, 175, 104));
        m.insert(SemanticToken::Danger, Color::Rgb(247, 118, 142));
        m.insert(SemanticToken::Info, Color::Rgb(125, 207, 255));
        m.insert(SemanticToken::Border, Color::Rgb(48, 54, 61));
        m.insert(SemanticToken::BorderFocus, Color::Rgb(122, 162, 247));
        m.insert(SemanticToken::SelectionBg, Color::Rgb(122, 162, 247));
        m.insert(SemanticToken::SelectionFg, Color::Rgb(11, 14, 20));
        m.insert(SemanticToken::SpinnerActive, Color::Rgb(122, 162, 247));
        m.insert(SemanticToken::ProgressFill, Color::Rgb(122, 162, 247));
        m.insert(SemanticToken::ProgressTrack, Color::Rgb(48, 54, 61));
        m
    }

    fn dark_palette_256() -> std::collections::HashMap<SemanticToken, Color> {
        let mut m = std::collections::HashMap::new();
        m.insert(SemanticToken::BgBase, Color::Indexed(234));
        m.insert(SemanticToken::BgRaised, Color::Indexed(235));
        m.insert(SemanticToken::BgOverlay, Color::Indexed(236));
        m.insert(SemanticToken::FgPrimary, Color::Indexed(254));
        m.insert(SemanticToken::FgSecondary, Color::Indexed(249));
        m.insert(SemanticToken::FgMuted, Color::Indexed(243));
        m.insert(SemanticToken::FgInverse, Color::Indexed(234));
        m.insert(SemanticToken::Accent, Color::Indexed(111));
        m.insert(SemanticToken::AccentDim, Color::Indexed(60));
        m.insert(SemanticToken::Success, Color::Indexed(150));
        m.insert(SemanticToken::Warning, Color::Indexed(179));
        m.insert(SemanticToken::Danger, Color::Indexed(210));
        m.insert(SemanticToken::Info, Color::Indexed(117));
        m.insert(SemanticToken::Border, Color::Indexed(239));
        m.insert(SemanticToken::BorderFocus, Color::Indexed(111));
        m.insert(SemanticToken::SelectionBg, Color::Indexed(111));
        m.insert(SemanticToken::SelectionFg, Color::Indexed(234));
        m.insert(SemanticToken::SpinnerActive, Color::Indexed(111));
        m.insert(SemanticToken::ProgressFill, Color::Indexed(111));
        m.insert(SemanticToken::ProgressTrack, Color::Indexed(239));
        m
    }

    fn no_color_palette() -> std::collections::HashMap<SemanticToken, Color> {
        let mut m = std::collections::HashMap::new();
        m.insert(SemanticToken::BgBase, Color::Reset);
        m.insert(SemanticToken::BgRaised, Color::Reset);
        m.insert(SemanticToken::BgOverlay, Color::Reset);
        m.insert(SemanticToken::FgPrimary, Color::Reset);
        m.insert(SemanticToken::FgSecondary, Color::Reset);
        m.insert(SemanticToken::FgMuted, Color::Reset);
        m.insert(SemanticToken::FgInverse, Color::Reset);
        m.insert(SemanticToken::Accent, Color::Reset);
        m.insert(SemanticToken::AccentDim, Color::Reset);
        m.insert(SemanticToken::Success, Color::Reset);
        m.insert(SemanticToken::Warning, Color::Reset);
        m.insert(SemanticToken::Danger, Color::Reset);
        m.insert(SemanticToken::Info, Color::Reset);
        m.insert(SemanticToken::Border, Color::Reset);
        m.insert(SemanticToken::BorderFocus, Color::Reset);
        m.insert(SemanticToken::SelectionBg, Color::Reset);
        m.insert(SemanticToken::SelectionFg, Color::Reset);
        m.insert(SemanticToken::SpinnerActive, Color::Reset);
        m.insert(SemanticToken::ProgressFill, Color::Reset);
        m.insert(SemanticToken::ProgressTrack, Color::Reset);
        m
    }

    pub fn color(&self, token: SemanticToken) -> Color {
        self.colors.get(&token).copied().unwrap_or(Color::Reset)
    }

    pub fn style(&self, token: SemanticToken) -> Style {
        if self.no_color {
            return Style::default();
        }
        Style::default().fg(self.color(token))
    }

    pub fn styled(&self, token: SemanticToken, modifiers: Modifier) -> Style {
        if self.no_color {
            return Style::default().add_modifier(modifiers);
        }
        Style::default()
            .fg(self.color(token))
            .add_modifier(modifiers)
    }

    pub fn bg_style(&self, bg: SemanticToken, fg: SemanticToken) -> Style {
        if self.no_color {
            return Style::default();
        }
        Style::default()
            .fg(self.color(fg))
            .bg(self.color(bg))
    }
}
