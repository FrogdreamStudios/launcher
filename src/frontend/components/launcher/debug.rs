//! Debug window component.

use dioxus::prelude::*;
use std::collections::VecDeque;
use crate::frontend::services::states::{get_debug_logs, clear_debug_logs, DebugLogEntry};

#[derive(Props, Clone, PartialEq, Eq)]
pub struct DebugWindowProps {
    pub show: Signal<bool>,
    pub instance_id: Signal<Option<u32>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[component]
pub fn DebugWindow(props: DebugWindowProps) -> Element {
    let mut show = props.show;
    let instance_id = props.instance_id;
    let mut is_hiding = use_signal(|| false);
    let mut should_render = use_signal(|| false);
    let mut console_logs = use_signal(|| VecDeque::<LogEntry>::new());

    // Handle show/hide animations
    use_effect(move || {
        if show() {
            should_render.set(true);
            is_hiding.set(false);
        } else if should_render() {
            is_hiding.set(true);
            spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                should_render.set(false);
                is_hiding.set(false);
            });
        }
    });

    // Load real logs from global state
    use_effect(move || {
        if show() {
            spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    if !show() {
                        break;
                    }
                    
                    let debug_logs = get_debug_logs();
                    let mut logs = console_logs.write();
                    logs.clear();
                    
                    // Filter logs for current instance if specified
                    for debug_log in debug_logs.iter() {
                        if let Some(current_instance) = instance_id() {
                            if debug_log.instance_id == Some(current_instance) || debug_log.instance_id.is_none() {
                                logs.push_back(LogEntry {
                                    timestamp: debug_log.timestamp.clone(),
                                    level: debug_log.level.clone(),
                                    message: debug_log.message.clone(),
                                });
                            }
                        } else {
                            // Show all logs if no instance selected
                            logs.push_back(LogEntry {
                                timestamp: debug_log.timestamp.clone(),
                                level: debug_log.level.clone(),
                                message: debug_log.message.clone(),
                            });
                        }
                    }
                }
            });
        }
    });

    let handle_backdrop_click = move |_| {
        show.set(false);
    };

    let handle_close_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        show.set(false);
    };

    let handle_clear_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        clear_debug_logs();
        console_logs.write().clear();
    };

    if !should_render() {
        return rsx! {};
    }

    let animation_class = if is_hiding() {
        "version-selector-hide"
    } else {
        "version-selector-show"
    };

    rsx! {
        style { {crate::backend::utils::css::ResourceLoader::get_css("debug")} }
        div {
            class: "debug-backdrop",
            onclick: handle_backdrop_click,
            div {
                class: "debug-window {animation_class}",
                onclick: |e: Event<MouseData>| e.stop_propagation(),
                
                // Header
                div {
                    class: "debug-header",
                    h2 {
                        class: "debug-title",
                        "Debug"
                    }
                    button {
                        class: "debug-close",
                        onclick: handle_close_click,
                        "✕"
                    }
                }
                
                // Instance info
                div {
                    class: "debug-selected",
                    div {
                        class: "selected-instance-label",
                        "Current Instance:"
                    }
                    div {
                        class: "selected-instance-value",
                        {
                            if let Some(id) = instance_id() {
                                format!("Instance #{}", id)
                            } else {
                                "No instance selected".to_string()
                            }
                        }
                    }
                }
                
                // Actions
                div {
                    class: "debug-actions",
                    button {
                        class: "debug-action-btn",
                        onclick: handle_clear_click,
                        "Clear console"
                    }
                }
                
                // Console content
                div {
                    class: "debug-content",
                    div {
                        class: "console-container",
                        if console_logs.read().is_empty() {
                            div {
                                class: "console-empty",
                                "No logs available"
                            }
                        } else {
                            for log in console_logs.read().iter() {
                                div {
                                    class: "console-line console-{log.level}",
                                    span {
                                        class: "console-timestamp",
                                        "{log.timestamp}"
                                    }
                                    span {
                                        class: "console-level",
                                        "[{log.level}]"
                                    }
                                    span {
                                        class: "console-message",
                                        "{log.message}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}