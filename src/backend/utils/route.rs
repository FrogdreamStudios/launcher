use crate::frontend::chats::main::Chat;
use crate::frontend::ui::auth::main::Auth;
use crate::frontend::ui::launcher::cloud::Cloud;
use crate::frontend::ui::launcher::home::Home;
use crate::frontend::ui::launcher::main::Main;
use crate::frontend::ui::launcher::new::New;
use crate::frontend::ui::launcher::packs::Packs;
use crate::frontend::ui::launcher::settings::Settings;

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
