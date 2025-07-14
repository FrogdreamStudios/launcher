use crate::backend::utils::css_loader::CssLoader;
use dioxus::prelude::*;

#[component]
pub fn AuthLayout(children: Element) -> Element {
    let mut show_ui = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            show_ui.set(true);
        });
    });

    rsx! {
        style {
            dangerous_inner_html: CssLoader::get_combined_auth()
        }

        div {
            class: if show_ui() { "auth-container fade-in" } else { "auth-container fade-out" },
            {children}
        }
    }
}
