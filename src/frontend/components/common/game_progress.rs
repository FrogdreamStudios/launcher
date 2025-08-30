//! Game progress component for installation and launch progress.

use dioxus::prelude::*;
use crate::backend::utils::css::ResourceLoader;

#[derive(Props, Clone, PartialEq)]
pub struct GameProgressProps {
    pub show: bool,
    pub progress: f32,
    pub status: String,
}

#[component]
pub fn GameProgress(props: GameProgressProps) -> Element {
    let GameProgressProps { show, progress, status } = props;

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
                class: "launch-progress-bar"
            }
        }
    }
}