//! Minecraft states.

use dioxus::prelude::*;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

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

#[derive(Clone, PartialEq, Debug)]
pub struct UpdateState {
    pub show: bool,
    pub progress: f32,
    pub status: String,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            show: false,
            progress: 0.0,
            status: String::new(),
        }
    }
}

// Global update state
static UPDATE_STATE: Lazy<Arc<Mutex<UpdateState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(UpdateState::default()))
});

pub fn use_update_state() -> (Signal<bool>, Signal<f32>, Signal<String>) {
    let mut show = use_signal(|| false);
    let mut progress = use_signal(|| 0.0);
    let mut status = use_signal(|| String::new());
    
    // Sync with global state
    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Ok(state) = UPDATE_STATE.lock() {
                    show.set(state.show);
                    progress.set(state.progress);
                    status.set(state.status.clone());
                }
            }
        });
    });
    
    (show, progress, status)
}

pub fn set_update_state(show: bool, progress: f32, status: String) {
    if let Ok(mut state) = UPDATE_STATE.lock() {
        state.show = show;
        state.progress = progress;
        state.status = status;
    }
}
