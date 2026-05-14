use ratatui::style::Color;
use tachyonfx::{fx, Effect, EffectManager, fx::RepeatMode};

pub struct AnimationState {
    pub manager: EffectManager<String>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            manager: EffectManager::default(),
        }
    }

    pub fn has_active_effects(&self) -> bool {
        self.manager.is_running()
    }

    pub fn process(&mut self, duration_ms: u32, buffer: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        let dur = tachyonfx::Duration::from_millis(duration_ms);
        self.manager.process_effects(dur, buffer, area);
    }

    pub fn enqueue(&mut self, key: &str, effect: Effect) {
        self.manager.add_unique_effect(key.to_string(), effect);
    }

    pub fn cancel(&mut self, key: &str) {
        self.manager.cancel_unique_effect(key);
    }
}

pub fn splash_dissolve() -> Effect {
    fx::dissolve(800)
}

pub fn progress_pulse() -> Effect {
    fx::repeat(fx::coalesce(1200), RepeatMode::Forever)
}

pub fn toast_slide_in() -> Effect {
    fx::slide_in(tachyonfx::Motion::LeftToRight, 30, 0, Color::Black, 300)
}

pub fn toast_slide_out() -> Effect {
    fx::slide_out(tachyonfx::Motion::RightToLeft, 30, 0, Color::Black, 200)
}

pub fn fade_in() -> Effect {
    fx::fade_from_fg(Color::Black, 400)
}

pub fn fade_out() -> Effect {
    fx::fade_to_fg(Color::Black, 300)
}
