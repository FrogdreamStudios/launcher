use crate::frontend::chats::chat::Chat;
use crate::frontend::ui::auth::auth::Auth;
use crate::frontend::ui::main::cloud::Cloud;
use crate::frontend::ui::main::home::Home;
use crate::frontend::ui::main::main::Main;
use crate::frontend::ui::main::new::New;
use crate::frontend::ui::main::packs::Packs;
use crate::frontend::ui::main::settings::Settings;

use dioxus::prelude::*;
use dioxus_router::Routable;

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