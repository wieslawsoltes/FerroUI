//! Chrome DevTools Protocol adapter. No server starts implicitly.
pub use ferroui_inspect as inspect;
mod input;
mod protocol;
pub use protocol::CdpSession;
mod browser;
mod log;
pub use browser::Browser;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod server;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use server::DevToolsServer;
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
mod bridge;
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub use bridge::BrowserBridge;
#[cfg(test)]
mod tests;
