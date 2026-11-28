//! The application's store: `store-read`, `store-write`, `store-list` and `store-delete` on a
//! folder of the application (docs/osprey-native-host.md, section 6). Entries are files under the
//! folder; a name is a relative path without `..`. A read puts the entry's text into the slice the
//! request names; a write takes its text from one.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::boundary;
use crate::protocol::Request;

/// The folder of one application's entries.
#[derive(Debug, Clone)]
pub struct Store {
    root: Option<PathBuf>,
}

impl Store {
    /// The store in `root`.
    pub fn at(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    /// A store that holds nothing: every read finds no entry and every write fails (a smoke run).
    pub fn none() -> Self {
        Self { root: None }
    }

    /// The platform's folder for the application `app`: `~/Library/Application Support/Osprey/<app>`
    /// on macOS, `$XDG_DATA_HOME/osprey/<app>` (or `~/.local/share/osprey/<app>`) on Linux,
    /// `%APPDATA%\Osprey\<app>` on Windows: where the other Osprey Native hosts keep it.
    pub fn for_app(app: &str) -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let root = if cfg!(target_os = "macos") {
            home.map(|home| home.join("Library/Application Support/Osprey").join(app))
        } else if cfg!(windows) {
            std::env::var_os("APPDATA").map(|data| PathBuf::from(data).join("Osprey").join(app))
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .filter(|data| !data.is_empty())
                .map(PathBuf::from)
                .or_else(|| home.map(|home| home.join(".local/share")))
                .map(|data| data.join("osprey").join(app))
        };
        Self { root }
    }

    /// Answers one store request with its result, or the error its completion carries.
    pub fn answer(&self, request: &Request) -> Result<Value, String> {
        match request.kind.as_str() {
            "store-read" => self.read(request),
            "store-write" => self.write(request),
            "store-list" => self.list(request),
            "store-delete" => {
                fs::remove_file(self.entry(request.text("name")?)?).map_err(failed)?;
                Ok(Value::Null)
            }
            kind => Err(format!("unsupported capability {kind}")),
        }
    }

    fn entry(&self, name: &str) -> Result<PathBuf, String> {
        let relative = Path::new(name);
        let safe = !relative.is_absolute()
            && relative
                .components()
                .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
        if !safe {
            return Err(format!("invalid store entry {name}"));
        }
        let root = self.root.as_ref().ok_or("this run has no store")?;
        Ok(root.join(relative))
    }

    fn read(&self, request: &Request) -> Result<Value, String> {
        let name = request.text("name")?;
        let limit = request
            .fields
            .get("limit")
            .and_then(Value::as_u64)
            .ok_or("store-read has no valid limit")?;
        let missing = |error: std::io::Error| match error.kind() {
            std::io::ErrorKind::NotFound => format!("no such store entry {name}"),
            _ => format!("could not read store entry {name}: {error}"),
        };
        let path = match self.root {
            Some(_) => self.entry(name)?,
            None => return Err(format!("no such store entry {name}")),
        };
        let size = fs::metadata(&path).map_err(missing)?.len();
        if size > limit {
            return Err(format!("store entry {name} exceeds limit {limit}"));
        }
        let text = String::from_utf8(fs::read(&path).map_err(missing)?)
            .map_err(|_| format!("store entry {name} is not UTF-8"))?;
        boundary::put_slice(request.text("slice")?, &text)?;
        Ok(Value::Null)
    }

    fn write(&self, request: &Request) -> Result<Value, String> {
        let path = self.entry(request.text("name")?)?;
        let text =
            boundary::get_slice(request.text("slice")?).ok_or("store-write slice is missing")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(failed)?;
        }
        let mut temporary = path.clone().into_os_string();
        temporary.push(".tmp");
        fs::write(&temporary, text).map_err(failed)?;
        fs::rename(&temporary, &path).map_err(failed)?;
        Ok(Value::Null)
    }

    fn list(&self, request: &Request) -> Result<Value, String> {
        let folder = self.entry(request.text("name")?)?;
        let mut names = fs::read_dir(folder)
            .map_err(failed)?
            .map(|entry| {
                entry
                    .map_err(failed)?
                    .file_name()
                    .into_string()
                    .map_err(|_| "a store entry name is not UTF-8".to_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;
        names.sort();
        Ok(Value::Array(names.into_iter().map(Value::String).collect()))
    }
}

fn failed(error: std::io::Error) -> String {
    format!("store operation failed: {error}")
}
