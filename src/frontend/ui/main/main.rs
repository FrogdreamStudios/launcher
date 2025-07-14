use crate::frontend::components::layout::Layout;
use crate::frontend::ui::auth::auth_context::AuthState;
use dioxus::prelude::*;
use dioxus_router::prelude::navigator;

#[component]
pub fn Main() -> Element {
    let nav = navigator();
    let auth = use_context::<AuthState>();

    if !(auth.is_authenticated)() {
        nav.replace("/auth");
        return rsx! { div {} };
    }

    rsx! { Layout {} }
}
