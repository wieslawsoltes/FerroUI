// Shared framework-owned UI session for native integration tests.
#[path = "headless_session.rs"]
mod session;
pub use session::run;
