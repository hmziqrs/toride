use std::env;

#[derive(Debug, Clone)]
pub struct TerminalCaps {
    pub truecolor: bool,
    pub unicode: bool,
    pub no_color: bool,
    pub force_color: bool,
    pub width: u16,
    pub height: u16,
}

impl TerminalCaps {
    pub fn detect() -> Self {
        let truecolor = env::var("COLORTERM")
            .map(|v| v == "truecolor" || v == "24bit")
            .unwrap_or(false);
        let unicode = env::var("LANG")
            .or_else(|_| env::var("LC_ALL"))
            .or_else(|_| env::var("LC_CTYPE"))
            .map(|v| v.contains("UTF-8") || v.contains("utf8"))
            .unwrap_or(false)
            && env::var("TERM").map(|v| v != "linux").unwrap_or(true);
        let no_color = env::var("NO_COLOR")
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false);
        let force_color = env::var("FORCE_COLOR")
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let (width, height) = crossterm::terminal::size().unwrap_or((100, 32));

        Self {
            truecolor,
            unicode,
            no_color,
            force_color,
            width,
            height,
        }
    }

    pub fn for_test() -> Self {
        Self {
            truecolor: true,
            unicode: true,
            no_color: false,
            force_color: false,
            width: 100,
            height: 32,
        }
    }
}
