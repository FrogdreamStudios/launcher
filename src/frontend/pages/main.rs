//! Main page component with authentication guard.

use crate::frontend::components::layout::Layout;
use crate::frontend::services::context::AuthState;
use dioxus::prelude::*;
use dioxus_router::navigator;

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
