use color_eyre::eyre::{Result, WrapErr};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, terminal,
};
use std::io::stdout;

use toride::app::App;

fn main() -> Result<()> {
    // Init tracing before color_eyre so SSH write errors are logged to stderr.
    // The TUI uses the alternate screen buffer, so stderr output is visible
    // after the app exits (or when redirected: `toride 2>log.txt`).
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(false)
        .init();

    color_eyre::install()?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            stdout(),
            terminal::LeaveAlternateScreen,
            DisableMouseCapture
        );
        original_hook(panic_info);
    }));

    terminal::enable_raw_mode()
        .wrap_err("Failed to enable raw mode — are you running in a TTY?")?;
    execute!(stdout(), terminal::EnterAlternateScreen, EnableMouseCapture)
        .wrap_err("Failed to enter alternate screen")?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    let terminal = ratatui::Terminal::new(backend).wrap_err("Failed to create terminal")?;

    let result = tokio::runtime::Runtime::new()
        .wrap_err("Failed to create tokio runtime")?
        .block_on(App::new().run(terminal));

    let _ = terminal::disable_raw_mode();
    let _ = execute!(
        stdout(),
        terminal::LeaveAlternateScreen,
        DisableMouseCapture
    );

    result
}
