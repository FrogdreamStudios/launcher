//! Minecraft version selector component.

use crate::{
    backend::launcher::models::VersionInfo,
    backend::utils::css::ResourceLoader,
    frontend::services::{instances::InstanceManager, launcher},
};

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SelectorProps {
    pub show: Signal<bool>,
}

#[component]
pub fn Selector(props: SelectorProps) -> Element {
    let mut show = props.show;
    let selected_version = use_signal(|| "1.21.8".to_string());
    let available_versions = use_signal(Vec::<VersionInfo>::new);
    let mut filtered_versions = use_signal(Vec::<VersionInfo>::new);
    let mut is_loading = use_signal(|| false);
    let mut version_filter = use_signal(|| "all".to_string()); // all, release, snapshot, beta, alpha

    // Load versions when the component shows
    use_effect(move || {
        if show() && available_versions.read().is_empty() && !*is_loading.read() {
            is_loading.set(true);
            let mut available_versions = available_versions.clone();
            spawn(async move {
                match launcher::get_version_manifest().await {
                    Ok(manifest) => {
                        // Load all versions without filtering
                        let versions: Vec<VersionInfo> = manifest.versions;
                        available_versions.set(versions);
                    }
                    Err(e) => log::error!("Failed to get version manifest: {e}"),
                }
                is_loading.set(false);
            });
        }
    });

    // Filter versions based on selected filter
    use_effect(move || {
        let filter = version_filter.read().clone();
        let all_versions = available_versions.read().clone();

        let filtered: Vec<VersionInfo> = match filter.as_str() {
            "release" => all_versions.into_iter().filter(|v| v.version_type == "release").collect(),
            "snapshot" => all_versions.into_iter().filter(|v| v.version_type == "snapshot").collect(),
            "beta" => all_versions.into_iter().filter(|v| v.version_type == "old_beta").collect(),
            "alpha" => all_versions.into_iter().filter(|v| v.version_type == "old_alpha").collect(),
            _ => all_versions, // "all" or any other value
        };

        filtered_versions.set(filtered);
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

                // Filter buttons
                div {
                    class: "version-selector-filters",
                    button {
                        class: format!("version-filter-btn{}", if version_filter() == "all" { " active" } else { "" }),
                        onclick: move |_| version_filter.set("all".to_string()),
                        "All"
                    }
                    button {
                        class: format!("version-filter-btn{}", if version_filter() == "release" { " active" } else { "" }),
                        onclick: move |_| version_filter.set("release".to_string()),
                        "Releases"
                    }
                    button {
                        class: format!("version-filter-btn{}", if version_filter() == "snapshot" { " active" } else { "" }),
                        onclick: move |_| version_filter.set("snapshot".to_string()),
                        "Snapshots"
                    }
                    button {
                        class: format!("version-filter-btn{}", if version_filter() == "beta" { " active" } else { "" }),
                        onclick: move |_| version_filter.set("beta".to_string()),
                        "Betas"
                    }
                    button {
                        class: format!("version-filter-btn{}", if version_filter() == "alpha" { " active" } else { "" }),
                        onclick: move |_| version_filter.set("alpha".to_string()),
                        "Alphas"
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
                            for version in filtered_versions.read().iter() {
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
                            if filtered_versions.read().is_empty() && !*is_loading.read() {
                                div { class: "version-list-empty", "No versions available for selected filter" }
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
