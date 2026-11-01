//! Freedesktop application catalog. The shell keeps plain Rust metadata, so
//! searches never reread desktop entries or touch the session bus.

use std::io;

#[cfg(target_os = "linux")]
mod gio;

#[derive(Clone, Debug)]
pub struct DesktopApplication {
    /// The XDG desktop file ID, including its `.desktop` suffix.
    pub id: String,
    pub name: String,
    pub description: String,
    /// Serialized GIcon, usually a single theme icon name.
    pub icon: String,
    search_text: String,
}

impl DesktopApplication {
    fn new(id: String, name: String, description: String, icon: String) -> Self {
        let search_text = format!("{name}\n{id}\n{description}").to_lowercase();
        Self {
            id,
            name,
            description,
            icon,
            search_text,
        }
    }

    /// Requests activation, allowing GIO to handle D-Bus activation, terminal
    /// selection, the working directory and field codes. No shell is involved.
    pub fn launch(&self) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        return gio::launch(&self.id);
        #[cfg(not(target_os = "linux"))]
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Application activation requires Linux",
        ))
    }
}

#[derive(Default, Debug)]
pub struct ApplicationCatalog {
    applications: Vec<DesktopApplication>,
    unavailable_reason: Option<String>,
}

impl ApplicationCatalog {
    pub fn discover() -> Self {
        #[cfg(target_os = "linux")]
        match gio::applications() {
            Ok(mut applications) => {
                applications.sort_by_cached_key(|app| (app.name.to_lowercase(), app.id.clone()));
                Self {
                    applications,
                    unavailable_reason: None,
                }
            }
            Err(error) => Self {
                unavailable_reason: Some(error.to_string()),
                ..Self::default()
            },
        }
        #[cfg(not(target_os = "linux"))]
        Self {
            unavailable_reason: Some("Application discovery requires Linux".into()),
            ..Self::default()
        }
    }

    pub fn applications(&self) -> &[DesktopApplication] {
        &self.applications
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }

    /// All words must match, in any order. Allocate once per query regardless
    /// of the number of applications.
    pub fn search_indices(&self, query: &str) -> impl Iterator<Item = usize> + '_ {
        let query = query.to_lowercase();
        self.applications
            .iter()
            .enumerate()
            .filter_map(move |(index, app)| {
                query
                    .split_whitespace()
                    .all(|word| app.search_text.contains(word))
                    .then_some(index)
            })
    }
}

#[cfg(test)]
mod tests;
