use crate::{
    backend::launcher::models::VersionInfo,
    backend::utils::css::ResourceLoader,
    frontend::services::{instances::main::InstanceManager, launcher},
};

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct VersionSelectorProps {
    pub show: Signal<bool>,
}

#[component]
pub fn VersionSelector(props: VersionSelectorProps) -> Element {
    let mut show = props.show;
    let selected_version = use_signal(|| "1.21.8".to_string());
    let available_versions = use_signal(Vec::<VersionInfo>::new);
    let mut is_loading = use_signal(|| false);

    // Load versions when the component shows
    use_effect(move || {
        if show() && available_versions.read().is_empty() && !*is_loading.read() {
            is_loading.set(true);
            let mut available_versions = available_versions.clone();
            spawn(async move {
                match launcher::get_version_manifest().await {
                    Ok(manifest) => {
                        // Only show release versions to simplify
                        let versions: Vec<VersionInfo> =
                            <Vec<VersionInfo> as Clone>::clone(&manifest.versions)
                                .into_iter()
                                .filter(|v| v.version_type == "release")
                                .take(20) // Limit to 20 most recent
                                .collect();
                        available_versions.set(versions);
                    }
                    Err(e) => log::error!("Failed to get version manifest: {e}"),
                }
                is_loading.set(false);
            });
        }
    });

    let handle_select_click = move |_| {
        let version = selected_version.read().clone();
        match InstanceManager::create_instance_with_version(version) {
            Some(_) => {}
            None => log::error!("Failed to create instance"),
        }
        show.set(false);
    };

    if !show() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "version-selector-backdrop",
            onclick: move |_| show.set(false),

            div {
                class: "version-selector",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "version-selector-header",
                    h3 { class: "version-selector-title", "Select Minecraft version" }
                    button {
                        class: "version-selector-close",
                        onclick: move |_| show.set(false),
                        img { src: ResourceLoader::get_asset("close") }
                    }
                }

                // Selected version display
                div {
                    class: "version-selector-selected",
                    div { class: "selected-version-label", "Selected version:" }
                    div { class: "selected-version-value", "{selected_version.read()}" }
                }

                // Version list
                div {
                    class: "version-selector-content",
                    if *is_loading.read() {
                        div {
                            class: "version-selector-loading",
                            "Loading versions..."
                        }
                    } else {
                        div {
                            class: "version-list",
                            for version in available_versions.read().iter() {
                                div {
                                    key: "{version.id}",
                                    class: format!("version-item{}",
                                        if version.id == *selected_version.read() { " selected" } else { "" }),
                                    onclick: {
                                        let version_id = version.id.clone();
                                        let mut selected_version = selected_version;
                                        move |_| {
                                            selected_version.set(version_id.clone());
                                        }
                                    },
                                    div { class: "version-name", "{version.id}" }
                                    div { class: "version-meta", "{format_date(&version.release_time)}" }
                                }
                            }
                            if available_versions.read().is_empty() && !*is_loading.read() {
                                div { class: "version-list-empty", "No versions available" }
                            }
                        }
                    }
                }

                // Actions
                div {
                    class: "version-selector-actions",
                    button {
                        class: "version-action-btn cancel",
                        onclick: move |_| show.set(false),
                        "Cancel"
                    }
                    button {
                        class: "version-action-btn select",
                        disabled: *is_loading.read(),
                        onclick: handle_select_click,
                        "Create instance"
                    }
                }
            }
        }
    }
}

fn format_date(date_str: &str) -> String {
    date_str
        .split('T')
        .next()
        .map_or_else(|| date_str.to_string(), std::string::ToString::to_string)
}
