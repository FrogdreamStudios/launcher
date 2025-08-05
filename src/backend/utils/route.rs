use crate::frontend::chats::main::Chat;
use crate::frontend::ui::auth::main::Auth;
use crate::frontend::ui::launcher::main::Main;

use dioxus::prelude::*;
use dioxus_router::Routable;

// Empty page components
#[component]
pub fn Home() -> Element {
    rsx! { div {} }
}

#[component]
pub fn Packs() -> Element {
    rsx! { div {} }
}

#[component]
pub fn Settings() -> Element {
    rsx! { div {} }
}

#[component]
pub fn Cloud() -> Element {
    rsx! { div {} }
}

#[component]
pub fn New() -> Element {
    rsx! { div {} }
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/auth")]
    Auth {},
    #[layout(Main)]
    #[redirect("/", || Route::Home {})]
    #[route("/home")]
    Home {},
    #[route("/packs")]
    Packs {},
    #[route("/settings")]
    Settings {},
    #[route("/cloud")]
    Cloud {},
    #[route("/new")]
    New {},
    #[route("/chat/:username")]
    Chat { username: String },
}
