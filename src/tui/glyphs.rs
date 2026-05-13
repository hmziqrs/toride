#[derive(Debug, Clone, Copy)]
pub enum Glyph {
    BorderTopLeft,
    BorderTopRight,
    BorderBottomLeft,
    BorderBottomRight,
    BorderHorizontal,
    BorderVertical,
    BorderRoundTopLeft,
    BorderRoundTopRight,
    BorderRoundBottomLeft,
    BorderRoundBottomRight,
    Selected,
    Unselected,
    Checked,
    Unchecked,
    Check,
    Cross,
    Warn,
    Ellipsis,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Spinner(u8),
    ProgressBlock(f32),
}

impl Glyph {
    pub fn char(&self, unicode: bool) -> &'static str {
        if !unicode {
            return self.ascii();
        }
        match self {
            Self::BorderTopLeft => "┌",
            Self::BorderTopRight => "┐",
            Self::BorderBottomLeft => "└",
            Self::BorderBottomRight => "┘",
            Self::BorderHorizontal => "─",
            Self::BorderVertical => "│",
            Self::BorderRoundTopLeft => "╭",
            Self::BorderRoundTopRight => "╮",
            Self::BorderRoundBottomLeft => "╰",
            Self::BorderRoundBottomRight => "╯",
            Self::Selected => "●",
            Self::Unselected => "○",
            Self::Checked => "☑",
            Self::Unchecked => "☐",
            Self::Check => "✓",
            Self::Cross => "✗",
            Self::Warn => "⚠",
            Self::Ellipsis => "⋯",
            Self::ArrowRight => "›",
            Self::ArrowLeft => "‹",
            Self::ArrowUp => "↑",
            Self::ArrowDown => "↓",
            Self::Spinner(i) => {
                let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                frames[*i as usize % frames.len()]
            }
            Self::ProgressBlock(f) => {
                let blocks = ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
                let idx = (*f * (blocks.len() as f32 - 1.0)).round() as usize;
                blocks[idx.min(blocks.len() - 1)]
            }
        }
    }

    fn ascii(&self) -> &'static str {
        match self {
            Self::BorderTopLeft
            | Self::BorderTopRight
            | Self::BorderBottomLeft
            | Self::BorderBottomRight
            | Self::BorderRoundTopLeft
            | Self::BorderRoundTopRight
            | Self::BorderRoundBottomLeft
            | Self::BorderRoundBottomRight => "+",
            Self::BorderHorizontal => "-",
            Self::BorderVertical => "|",
            Self::Selected => "[*]",
            Self::Unselected => "[ ]",
            Self::Checked => "[x]",
            Self::Unchecked => "[ ]",
            Self::Check => "OK",
            Self::Cross => "X",
            Self::Warn => "!",
            Self::Ellipsis => "...",
            Self::ArrowRight => ">",
            Self::ArrowLeft => "<",
            Self::ArrowUp => "^",
            Self::ArrowDown => "v",
            Self::Spinner(_) => "*",
            Self::ProgressBlock(f) => {
                if *f > 0.5 { "#" } else { "-" }
            }
        }
    }
}
