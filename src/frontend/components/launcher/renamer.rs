//! Instance rename dialog component.

use crate::{
    backend::utils::css::ResourceLoader, frontend::services::instances::InstanceManager,
};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq, Eq)]
pub struct RenameDialogProps {
    pub show: Signal<bool>,
    pub instance_id: Signal<Option<u32>>,
    pub current_name: Signal<String>,
}

#[component]
pub fn RenameDialog(props: RenameDialogProps) -> Element {
    let mut show = props.show;
    let instance_id = props.instance_id;
    let current_name = props.current_name;

    let mut new_name = use_signal(String::new);
    let mut is_hiding = use_signal(|| false);
    let mut should_render = use_signal(|| false);

    // Initialize new_name when dialog opens
    use_effect(move || {
        if show() && !current_name().is_empty() {
            new_name.set(current_name());
            should_render.set(true);
            is_hiding.set(false);
        } else if !show() && should_render() {
            // Start hide animation
            is_hiding.set(true);
            spawn(async move {
                // Wait for animation to complete
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                should_render.set(false);
                is_hiding.set(false);
                new_name.set(String::new());
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

    let handle_cancel_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        show.set(false);
    };

    let handle_rename_click = move |e: Event<MouseData>| {
        e.stop_propagation();
        let can_rename = !new_name().trim().is_empty() && new_name().trim() != current_name();
        if can_rename {
            if let Some(id) = instance_id() {
                InstanceManager::rename_instance(id, new_name().trim());
                show.set(false);
            }
        }
    };

    let handle_input_change = move |e: Event<FormData>| {
        let value: String = e.value().chars().take(7).collect();
        new_name.set(value);
    };

    let handle_key_press = move |e: Event<KeyboardData>| match e.key() {
        Key::Enter => {
            if let Some(id) = instance_id() {
                if !new_name().trim().is_empty() && new_name().trim() != current_name() {
                    InstanceManager::rename_instance(id, new_name().trim());
                    show.set(false);
                }
            }
        }
        Key::Escape => {
            show.set(false);
        }
        _ => {}
    };

    if !should_render() {
        return rsx! {};
    }

    let dialog_class = if is_hiding() {
        "rename-dialog rename-dialog-hide"
    } else {
        "rename-dialog rename-dialog-show"
    };

    let can_rename = !new_name().trim().is_empty() && new_name().trim() != current_name();

    rsx! {
        div {
            class: "rename-dialog-backdrop",
            onclick: handle_backdrop_click,

            div {
                class: "{dialog_class}",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "rename-dialog-header",
                    h2 {
                        class: "rename-dialog-title",
                        "Rename instance"
                    }
                    button {
                        class: "rename-dialog-close",
                        onclick: handle_close_click,
                        img {
                            src: "{ResourceLoader::get_asset(\"close\")}",
                            alt: "Close"
                        }
                    }
                }

                // Content
                div {
                    class: "rename-dialog-content",

                    div {
                        class: "rename-current-name",
                        div {
                            class: "rename-current-label",
                            "Current name:"
                        }
                        div {
                            class: "rename-current-value",
                            "{current_name()}"
                        }
                    }

                    div {
                        class: "rename-input-section",
                        div {
                            class: "rename-input-label",
                            "New name:"
                        }
                        input {
                            r#type: "text",
                            class: "rename-input",
                            value: "{new_name()}",
                            placeholder: "Enter new name...",
                            maxlength: "7",
                            autofocus: true,
                            oninput: handle_input_change,
                            onkeydown: handle_key_press,
                        }
                        div {
                            class: "rename-char-count",
                            "{new_name().len()}/7"
                        }
                    }
                }

                // Actions
                div {
                    class: "rename-dialog-actions",
                    button {
                        class: "rename-action-btn cancel",
                        onclick: handle_cancel_click,
                        "Cancel"
                    }
                    button {
                        class: "rename-action-btn rename",
                        class: if !can_rename { "disabled" },
                        onclick: handle_rename_click,
                        disabled: !can_rename,
                        "Rename"
                    }
                }
            }
        }
    }
}
