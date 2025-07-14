use dioxus::prelude::*;

#[derive(Clone)]
pub struct AuthState {
    pub is_authenticated: Signal<bool>,
}