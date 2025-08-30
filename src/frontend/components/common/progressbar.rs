//! Progressbar component.

use dioxus::prelude::*;
use crate::backend::utils::css::ResourceLoader;

#[derive(Props, Clone, PartialEq)]
pub struct UpdateProgressProps {
    pub show: bool,
    pub progress: f32,
    pub status: String,
}

#[component]
pub fn UpdateProgress(props: UpdateProgressProps) -> Element {
    let UpdateProgressProps { show, progress, status } = props;

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