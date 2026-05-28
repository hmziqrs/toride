use color_eyre::eyre::{Result, WrapErr};
use crossterm::{execute, terminal};
use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
};
use std::io::stdout;
use tokio::select;

use toride::ui::welcome::WelcomeScreen;

fn main() -> Result<()> {
    color_eyre::install()?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    terminal::enable_raw_mode()
        .wrap_err("Failed to enable raw mode — are you running in a TTY?")?;
    execute!(stdout(), terminal::EnterAlternateScreen)
        .wrap_err("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to create terminal")?;

    let result = tokio::runtime::Runtime::new()
        .wrap_err("Failed to create tokio runtime")?
        .block_on(run(&mut terminal));

    let _ = terminal::disable_raw_mode();
    let _ = execute!(stdout(), terminal::LeaveAlternateScreen);

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let mut welcome = WelcomeScreen::new();
    let mut events = EventStream::new();

    loop {
        terminal.draw(|frame| welcome.render(frame))?;

        select! {
            Some(Ok(event)) = events.next() => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        other => {
                            welcome.handle_key(other);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
