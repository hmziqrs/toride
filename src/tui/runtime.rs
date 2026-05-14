use std::time::{Duration, Instant};

use crossterm::event::{EventStream, Event};
use futures::{StreamExt, FutureExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::tui::animation::AnimationState;
use crate::tui::caps::TerminalCaps;
use crate::tui::model::Model;
use crate::tui::update::{self, Action, Effect};

pub async fn run() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let caps = TerminalCaps::detect();
    let mut terminal = ratatui::init();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableBracketedPaste
    )?;

    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
    let cancel = CancellationToken::new();
    let mut model = Model::initial(caps);
    let mut animations = AnimationState::new();
    let mut last_frame = Instant::now();

    spawn_terminal_events(action_tx.clone(), cancel.clone());
    spawn_logical_tick(action_tx.clone(), cancel.clone());
    spawn_signal_watcher(action_tx.clone());

    // Send Init action
    let _ = action_tx.send(Action::Init);

    loop {
        let Some(action) = action_rx.recv().await else { break };

        let new_effects = update::update(&mut model, action);
        for eff in new_effects {
            if let crate::tui::update::Effect::PushFx(fx_effect) = eff {
                animations.enqueue("current", fx_effect);
            } else {
                crate::tui::effects::spawn_effect(eff, action_tx.clone(), cancel.clone());
            }
        }

        let active = animations.has_active_effects();
        if model.needs_render || active {
            let elapsed = last_frame.elapsed();
            last_frame = Instant::now();
            terminal.draw(|frame| {
                let area = frame.area();
                crate::tui::view::view(frame, &model);
                // Process animation effects on the rendered buffer
                animations.process(elapsed.as_millis() as u32, frame.buffer_mut(), area);
            })?;
            model.needs_render = false;
        }

        if active {
            let tx = action_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(16)).await;
                let _ = tx.send(Action::AnimationTick);
            });
        }

        if model.should_quit {
            break;
        }
    }

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableBracketedPaste
    ).ok();
    ratatui::restore();

    Ok(())
}

fn spawn_terminal_events(tx: mpsc::UnboundedSender<Action>, cancel: CancellationToken) {
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                maybe_event = reader.next().fuse() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if tx.send(Action::Key(key)).is_err() { break; }
                        }
                        Some(Ok(Event::Resize(w, h))) => {
                            if tx.send(Action::Resize(w, h)).is_err() { break; }
                        }
                        Some(Ok(Event::FocusGained)) => {
                            if tx.send(Action::FocusGained).is_err() { break; }
                        }
                        Some(Ok(Event::FocusLost)) => {
                            if tx.send(Action::FocusLost).is_err() { break; }
                        }
                        Some(Ok(Event::Paste(text))) => {
                            if tx.send(Action::Paste(text)).is_err() { break; }
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

fn spawn_logical_tick(tx: mpsc::UnboundedSender<Action>, cancel: CancellationToken) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = interval.tick() => {
                    if tx.send(Action::Tick).is_err() { break; }
                }
            }
        }
    });
}

fn spawn_signal_watcher(tx: mpsc::UnboundedSender<Action>) {
    tokio::spawn(async move {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
        let _ = tx.send(Action::Quit);
    });
}
