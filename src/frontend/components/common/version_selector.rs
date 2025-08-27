use crate::{
    backend::launcher::{launcher::MinecraftLauncher, models::VersionInfo},
    backend::utils::css::main::ResourceLoader,
    frontend::services::instances::main::InstanceManager,
};
use crate::{log_error, log_info};
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum VersionFilter {
    Release,
    Beta,
    Alpha,
    Snapshot,
    All,
}

impl VersionFilter {
    pub fn to_string(&self) -> &'static str {
        match self {
            VersionFilter::Release => "Releases",
            VersionFilter::Beta => "Beta",
            VersionFilter::Alpha => "Alpha",
            VersionFilter::Snapshot => "Snapshots",
            VersionFilter::All => "All",
        }
    }

    pub fn matches(&self, version_type: &str) -> bool {
        match self {
            VersionFilter::Release => version_type == "release",
            VersionFilter::Beta => version_type == "old_beta",
            VersionFilter::Alpha => version_type == "old_alpha",
            VersionFilter::Snapshot => version_type == "snapshot",
            VersionFilter::All => true,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct VersionSelectorProps {
    pub show: Signal<bool>,
}

#[component]
pub fn VersionSelector(props: VersionSelectorProps) -> Element {
    let mut show = props.show;
    let selected_version = use_signal(|| "1.21.8".to_string());
    let mut available_versions = use_signal(Vec::<VersionInfo>::new);
    let filtered_versions = use_signal(Vec::<VersionInfo>::new);
    let mut is_loading = use_signal(|| false);
    let current_filter = use_signal(|| VersionFilter::Release);
    let mut is_hiding = use_signal(|| false);
    let mut should_render = use_signal(|| false);

    // Watch for show changes and handle animation
    use_effect(move || {
        if show() {
            should_render.set(true);
            is_hiding.set(false);
        } else if should_render() {
            // Small delay before starting hide animation
            spawn(async move {
                is_hiding.set(true);
                // Hide after animation completes (150 ms)
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                should_render.set(false);
                is_hiding.set(false);
            });
        }
    });

    // Load versions when the component mounts
    use_effect(move || {
        if show() && available_versions.read().is_empty() && !*is_loading.read() {
            is_loading.set(true);
            spawn(async move {
                match load_available_versions().await {
                    Ok(versions) => {
                        available_versions.set(versions);
                        apply_filter(
                            &available_versions.read(),
                            &current_filter(),
                            filtered_versions,
                        );
                    }
                    Err(e) => log_error!("Failed to load versions: {e}"),
                }
                is_loading.set(false);
            });
        }
    });

    // Apply filter when filter changes
    use_effect(move || {
        let filter = current_filter();
        let versions = available_versions.read();
        apply_filter(&versions, &filter, filtered_versions);
    });

    // Handle clicks outside the selector
    let handle_backdrop_click = move |_| {
        show.set(false);
    };

    let handle_select_click = move |_| {
        let version = selected_version.read().clone();
        println!("Selected version in handle_select_click: {version}");
        log_info!("Creating instance with version: {version}");

        // Create the instance
        if let Some(instance_id) = InstanceManager::create_instance_with_version(version.clone()) {
            log_info!("Created instance {instance_id} with version {version}");

            // Verify the instance was created with a correct version
            use crate::frontend::services::instances::main::INSTANCES;
            let instances = INSTANCES.read();
            if let Some(instance) = instances.get(&instance_id) {
                println!(
                    "VERIFICATION: Instance {} created with version: {}",
                    instance_id, instance.version
                );
            } else {
                println!("ERROR: Cannot find newly created instance {instance_id}");
            }
        } else {
            log_error!("Failed to create instance with version {version}");
        }

        show.set(false);
    };

    if !should_render() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "version-selector-backdrop",
            onclick: handle_backdrop_click,

            div {
                class: if is_hiding() { "version-selector version-selector-hide" } else { "version-selector version-selector-show" },
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "version-selector-header",
                    h3 { class: "version-selector-title", "Select Minecraft Version" }
                    button {
                        class: "version-selector-close",
                        onclick: move |_| show.set(false),
                        img { src: ResourceLoader::get_asset("close") }
                    }
                }

                // Filter buttons
                div {
                    class: "version-selector-filters",
                    for filter in [VersionFilter::Release, VersionFilter::Beta, VersionFilter::Alpha, VersionFilter::Snapshot, VersionFilter::All] {
                        button {
                            key: "{filter.to_string()}",
                            class: format!("version-filter-btn{}",
                                if filter == current_filter() { " active" } else { "" }),
                            onclick: {
                                let filter_copy = filter.clone();
                                let mut current_filter = current_filter;
                                move |_| {
                                    println!("Filter changed to: {filter_copy:?}");
                                    current_filter.set(filter_copy.clone());
                                }
                            },
                            "{filter.to_string()}"
                        }
                    }
                }

                // Selected version display
                div {
                    class: "version-selector-selected",
                    div { class: "selected-version-label", "Selected Version:" }
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
                                            println!("Version selected: {version_id}");
                                            selected_version.set(version_id.clone());
                                        }
                                    },
                                    div { class: "version-name", "{version.id}" }
                                    div { class: "version-meta",
                                        "{version.version_type} | {format_date(&version.release_time)}"
                                    }
                                }
                            }
                            if filtered_versions.read().is_empty() && !*is_loading.read() {
                                div { class: "version-list-empty", "No versions found for this filter" }
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
                        onclick: handle_select_click,
                        "Select"
                    }
                }
            }
        }
    }
}

async fn load_available_versions() -> crate::utils::Result<Vec<VersionInfo>> {
    let launcher = MinecraftLauncher::new(None, None).await?;
    Ok(launcher.get_available_versions()?.to_vec())
}

fn apply_filter(
    versions: &[VersionInfo],
    filter: &VersionFilter,
    mut filtered_versions: Signal<Vec<VersionInfo>>,
) {
    let filtered: Vec<VersionInfo> = versions
        .iter()
        .filter(|v| filter.matches(&v.version_type))
        .cloned()
        .collect();
    filtered_versions.set(filtered);
}

fn format_date(date_str: &str) -> String {
    date_str
        .split('T')
        .next()
        .map_or_else(|| date_str.to_string(), std::string::ToString::to_string)
}
