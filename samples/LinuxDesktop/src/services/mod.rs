//! Freedesktop application catalog. The shell keeps plain Rust metadata, so
//! searches never reread desktop entries or touch the session bus.

use std::io;

pub mod notifications;
pub mod quick_settings;

#[cfg(target_os = "linux")]
mod gio;
#[cfg(target_os = "linux")]
mod icons;
#[cfg(target_os = "linux")]
pub use icons::ApplicationIcons;

mod catalog;
pub mod favorites;
pub(crate) mod preferences;
pub use catalog::{ApplicationCatalog, DesktopApplication};

impl DesktopApplication {
    /// Requests activation, allowing GIO to handle D-Bus activation, terminal
    /// selection, the working directory and field codes. No shell is involved.
    pub fn launch(&self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        return gio::launch(&self.id, None);
        #[cfg(not(target_os = "linux"))]
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Application activation requires Linux",
        ))
    }
    #[cfg(target_os = "linux")]
    pub fn launch_on_display(&self, display: &std::ffi::OsStr) -> io::Result<()> {
        gio::launch(&self.id, Some(display))
    }
}

impl ApplicationCatalog {
    pub fn discover() -> Self {
        #[cfg(target_os = "linux")]
        match gio::applications() {
            Ok(applications) => Self::from_applications(applications),
            Err(error) => Self::unavailable(error.to_string()),
        }
        #[cfg(not(target_os = "linux"))]
        Self::unavailable("Application discovery requires Linux".into())
    }
}

#[cfg(test)]
mod tests;
