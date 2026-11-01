//! Limit local source discovery to the file workspace selected by the editor.
use super::model::Model;
use serde_json::Value;
use std::{fs, io::Read, path::PathBuf};
use url::Url;

#[derive(Default)]
pub(crate) struct Workspace {
    roots: Vec<PathBuf>,
}

impl Workspace {
    pub fn configure(&mut self, params: &Value) {
        self.roots = params["workspaceFolders"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["uri"].as_str())
            .chain(params["rootUri"].as_str())
            .filter_map(|s| Url::parse(s).ok()?.to_file_path().ok()?.canonicalize().ok())
            .filter(|p| p.is_dir())
            .collect();
    }
    fn local_path(&self, url: &Url) -> Option<PathBuf> {
        let path = url.to_file_path().ok()?.canonicalize().ok()?;
        self.roots
            .iter()
            .any(|root| path.starts_with(root))
            .then_some(path)
    }
    pub fn read(&self, url: &Url) -> Option<String> {
        const LIMIT: u64 = 16 * 1024 * 1024;
        let path = self.local_path(url)?;
        let meta = fs::metadata(&path).ok()?;
        if !meta.is_file() || meta.len() > LIMIT {
            return None;
        }
        let mut source = String::new();
        fs::File::open(path)
            .ok()?
            .take(LIMIT + 1)
            .read_to_string(&mut source)
            .ok()?;
        (source.len() as u64 <= LIMIT).then_some(source)
    }
    pub fn components(&self, dir: &Url) -> Vec<String> {
        let Some(entries) = self.local_path(dir).and_then(|p| fs::read_dir(p).ok()) else {
            return vec![];
        };
        entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()?
                    .strip_suffix(".qml")
                    .map(str::to_owned)
            })
            .filter(|ty| ty.starts_with(char::is_uppercase))
            .collect()
    }
}

/// Explicit directory imports take precedence over the implicit sibling directory.
pub(super) fn directories(uri: &str, model: &Model, alias: Option<&str>) -> Vec<Url> {
    let Ok(base) = Url::parse(uri) else {
        return vec![];
    };
    model
        .imports
        .iter()
        .rev()
        .filter(|i| i.is_path && !i.is_script() && i.alias.as_deref() == alias)
        .map(|i| i.uri.as_str())
        .chain(alias.is_none().then_some("."))
        .filter_map(|dir| base.join(&format!("{}/", dir.trim_end_matches('/'))).ok())
        .collect()
}
