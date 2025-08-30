use crate::backend::utils::progress_bridge;
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum GameStatus {
    Idle,
    Launching { progress: f32, message: String },
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl GameStatus {
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Launching { .. })
    }

    pub fn get_progress(&self) -> f32 {
        match self {
            Self::Launching { progress, .. } => *progress,
            Self::Idle => 0.0,
        }
    }

    pub fn get_message(&self) -> &str {
        match self {
            Self::Launching { message, .. } => message,
            Self::Idle => "",
        }
    }
}

pub fn use_game_state() -> Signal<GameStatus> {
    let game_status = use_signal(|| GameStatus::Idle);

    // Initialize progress listening on first use
    use_effect(move || {
        use crate::backend::launcher::progress::ProgressStage;
        let mut rx = progress_bridge::init_progress_channel();
        let mut status = game_status;

        spawn(async move {
            while let Some(progress_info) = rx.recv().await {
                // Check if this is a completion or failure signal
                if progress_info.stage == ProgressStage::Completed || progress_info.stage == ProgressStage::Failed {
                    // Reset to Idle state to hide progress bar
                    status.set(GameStatus::Idle);
                } else {
                    // Normal progress update for all other stages
                    status.set(GameStatus::Launching {
                        progress: progress_info.progress,
                        message: progress_info.message,
                    });
                }
            }
        });
    });

    game_status
}
