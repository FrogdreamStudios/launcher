use dioxus::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GameStatus {
    Idle,
    Launching,
    Running,
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl GameStatus {
    pub const fn is_active(&self) -> bool {
        matches!(self, GameStatus::Launching | GameStatus::Running)
    }
}

pub fn use_game_state() -> Signal<GameStatus> {
    use_signal(|| GameStatus::Idle)
}
