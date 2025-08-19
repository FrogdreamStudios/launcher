//! Application routing system.

use crate::frontend::pages::auth::Auth;
use crate::frontend::pages::main::main::Main;
use crate::frontend::pages::new::New as NewPage;
use crate::frontend::pages::settings::Settings as SettingsPage;
use crate::frontend::services::chats::main::Chat;

use dioxus::prelude::*;
use dioxus_router::Routable;

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
    rsx! { SettingsPage {} }
}

#[component]
pub fn Cloud() -> Element {
    rsx! { div {} }
}

#[component]
pub fn New() -> Element {
    rsx! { NewPage {} }
}

/// Main routing enum for the application.
#[derive(Clone, Routable, Debug, PartialEq, Eq)]
pub enum Route {
    /// Authentication page route.
    #[route("/auth")]
    Auth {},
    /// Main layout wrapper with home page as default.
    #[layout(Main)]
    #[redirect("/", || Route::Home {})]
    #[route("/home")]
    Home {},
    /// Mod packs management page.
    #[route("/packs")]
    Packs {},
    /// Application settings page.
    #[route("/settings")]
    Settings {},
    /// Cloud storage management page.
    #[route("/cloud")]
    Cloud {},
    /// New instance creation page.
    #[route("/new")]
    New {},
    /// Chat page with dynamic username parameter.
    #[route("/chat/:username")]
    Chat { username: String },
}
