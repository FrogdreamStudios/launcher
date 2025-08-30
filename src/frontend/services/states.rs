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

// Game progress state for installation and launch
#[derive(Clone, PartialEq, Debug)]
pub struct GameProgressState {
    pub show: bool,
    pub progress: f32,
    pub status: String,
    pub status_type: ProgressStatus,
    pub instance_id: Option<u32>,
}

impl Default for GameProgressState {
    fn default() -> Self {
        Self {
            show: false,
            progress: 0.0,
            status: String::new(),
            status_type: ProgressStatus::default(),
            instance_id: None,
        }
    }
}

// Global game progress state
static GAME_PROGRESS_STATE: Lazy<Arc<Mutex<GameProgressState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(GameProgressState::default()))
});

pub fn use_game_progress_state() -> (Signal<bool>, Signal<f32>, Signal<String>, Signal<ProgressStatus>, Signal<Option<u32>>) {
    let mut show = use_signal(|| false);
    let mut progress = use_signal(|| 0.0);
    let mut status = use_signal(|| String::new());
    let mut status_type = use_signal(|| ProgressStatus::default());
    let mut instance_id = use_signal(|| None::<u32>);
    
    // Sync with global state
    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Ok(state) = GAME_PROGRESS_STATE.lock() {
                    show.set(state.show);
                    progress.set(state.progress);
                    status.set(state.status.clone());
                    status_type.set(state.status_type.clone());
                    instance_id.set(state.instance_id);
                }
            }
        });
    });
    
    (show, progress, status, status_type, instance_id)
}

pub fn set_game_progress_state(show: bool, progress: f32, status: String, status_type: ProgressStatus, instance_id: Option<u32>) {
    if let Ok(mut state) = GAME_PROGRESS_STATE.lock() {
        state.show = show;
        state.progress = progress;
        state.status = status;
        state.status_type = status_type;
        state.instance_id = instance_id;
    }
}

// Convenience function for backward compatibility
pub fn set_game_progress_state_simple(show: bool, progress: f32, status: String, instance_id: Option<u32>) {
    set_game_progress_state(show, progress, status, ProgressStatus::InProgress, instance_id);
}

// Running instances tracking
use std::collections::{HashSet, VecDeque};
use chrono::{DateTime, Local};

// Debug console logs
#[derive(Clone, PartialEq, Debug)]
pub struct DebugLogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub instance_id: Option<u32>,
}

static DEBUG_LOGS: Lazy<Arc<Mutex<VecDeque<DebugLogEntry>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(VecDeque::new()))
});

pub fn add_debug_log(level: String, message: String, instance_id: Option<u32>) {
    if let Ok(mut logs) = DEBUG_LOGS.lock() {
        let now: DateTime<Local> = Local::now();
        logs.push_back(DebugLogEntry {
            timestamp: now.format("%H:%M:%S").to_string(),
            level,
            message,
            instance_id,
        });
        
        // Keep only last 500 entries
        while logs.len() > 500 {
            logs.pop_front();
        }
    }
}

pub fn get_debug_logs() -> VecDeque<DebugLogEntry> {
    if let Ok(logs) = DEBUG_LOGS.lock() {
        logs.clone()
    } else {
        VecDeque::new()
    }
}

pub fn clear_debug_logs() {
    if let Ok(mut logs) = DEBUG_LOGS.lock() {
        logs.clear();
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ProgressStatus {
    InProgress,
    Success,
    Failed,
}

impl Default for ProgressStatus {
    fn default() -> Self {
        Self::InProgress
    }
}

static RUNNING_INSTANCES: Lazy<Arc<Mutex<HashSet<u32>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashSet::new()))
});

pub fn is_instance_running(instance_id: u32) -> bool {
    if let Ok(running) = RUNNING_INSTANCES.lock() {
        running.contains(&instance_id)
    } else {
        false
    }
}

pub fn set_instance_running(instance_id: u32, running: bool) {
    if let Ok(mut instances) = RUNNING_INSTANCES.lock() {
        if running {
            instances.insert(instance_id);
        } else {
            instances.remove(&instance_id);
        }
    }
}
