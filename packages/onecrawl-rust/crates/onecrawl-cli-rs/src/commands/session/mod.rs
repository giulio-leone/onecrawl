// session/ module — split from monolithic session.rs (1401 lines)
// core: SessionInfo, SessionAction, handle, connect, load/save
// launchers: Chrome launch strategies (normal, headless, proxy)
// injection: stealth and cookie injection

pub mod core;
pub(crate) mod launchers;
pub(crate) mod injection;

// Re-export public API
pub use core::{load_session, save_session, connect_to_session, handle, SessionAction};
// injection functions used internally via super::injection in core.rs
// launcher functions used internally via super::launchers in core.rs
