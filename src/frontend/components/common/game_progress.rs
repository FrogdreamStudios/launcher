//! Game progress component for installation and launch progress.

use crate::backend::utils::css::ResourceLoader;
use crate::frontend::services::states::ProgressStatus;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct GameProgressProps {
    pub show: bool,
    pub progress: f32,
    pub status: String,
    pub status_type: ProgressStatus,
}

#[component]
pub fn GameProgress(props: GameProgressProps) -> Element {
    let GameProgressProps {
        show,
        progress,
        status,
        status_type,
    } = props;

    if !show {
        return rsx! { div {} };
    }

    rsx! {
        style {
            dangerous_inner_html: ResourceLoader::get_css("progress")
        }

        div {
            class: "launch-progress-container",
            style: "--progress-width: {progress}%",

            div {
                class: "launch-progress-text",
                "{status}"
            }

            div {
                class: match status_type {
                    ProgressStatus::InProgress => "launch-progress-bar",
                    ProgressStatus::Success => "launch-progress-bar success",
                    ProgressStatus::Failed => "launch-progress-bar failed",
                }
            }
        }
    }
}
