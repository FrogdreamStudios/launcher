//! Minecraft states.

use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum GameStatus {
    Idle,
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::Idle
    }
}

pub fn use_game_state() -> Signal<GameStatus> {
    use_signal(|| GameStatus::Idle)
}
