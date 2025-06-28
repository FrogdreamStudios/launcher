// src/route.rs
use crate::application::auth::auth::Auth;
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::application::main::main::Main;
use crate::application::main::home::Home;
use crate::application::main::mods_and_packs::ModsAndPacks;
use crate::application::main::settings::Settings;
use crate::application::main::cloud::Cloud;
use crate::application::main::new::New;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/auth")]
    Auth {},
    #[layout(Main)]
    #[redirect("/", || Route::Home {})]
    #[route("/home")]
    Home {},
    #[route("/mods_and_packs")]
    ModsAndPacks {},
    #[route("/settings")]
    Settings {},
    #[route("/cloud")]
    Cloud {},
    #[route("/new")]
    New {},
}