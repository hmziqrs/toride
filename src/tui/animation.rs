pub struct AnimationState {
    pub manager: tachyonfx::EffectManager<String>,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            manager: tachyonfx::EffectManager::default(),
        }
    }

    pub fn has_active_effects(&self) -> bool {
        self.manager.is_running()
    }

    pub fn process(&mut self, duration_ms: u32) {
        // Effects processed via tachyonfx during terminal.draw() in a future iteration
        let _ = duration_ms;
    }
}
