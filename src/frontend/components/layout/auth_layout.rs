use crate::frontend::assets::main::ResourceLoader;
use crate::frontend::components::common::titlebar::TitleBar;
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
            dangerous_inner_html: ResourceLoader::get_auth_css_with_fonts()
        }

        TitleBar {}

        div {
            class: if show_ui() { "auth-container fade-in" } else { "auth-container fade-out" },
            {children}
        }
    }
}
