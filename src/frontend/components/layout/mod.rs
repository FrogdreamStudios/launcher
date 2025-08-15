//! Layout components.

pub mod auth_layout;
pub mod chat_sidebar;
pub mod main;
pub mod navigation;

// Re-export layout components for easier access
pub use auth_layout::AuthLayout;
pub use chat_sidebar::ChatSidebar;
pub use main::Layout;
pub use navigation::Navigation;
