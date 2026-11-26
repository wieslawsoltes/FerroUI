use crate::{require, uri::canonical_path, ProjectError, Result};
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub(super) fn relative_path(name: &str) -> Result<&Path> {
    let path = Path::new(name);
    require(
        name.len() <= 4096
            && !name.contains(['\\', ':'])
            && !name.chars().any(char::is_control)
            && name.split('/').all(|part| !matches!(part, "" | "." | ".."))
            && path
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
        "Use a relative path with forward slashes inside its designated folder",
    )?;
    Ok(path)
}

pub(super) fn confined_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    let path = canonical_path(&root.join(relative))?;
    require(
        path.starts_with(root) && path != root,
        "The handoff path leaves its designated folder",
    )?;
    Ok(path)
}

pub(super) fn existing_file(path: &Path) -> Result<PathBuf> {
    require(
        path.is_absolute(),
        "Choose a local file with an absolute path",
    )?;
    let path = path.canonicalize()?;
    require(path.is_file(), "The handoff source must be a regular file")?;
    Ok(path)
}

/// Create only the missing parents of a checked destination. Failed copies
/// remove their own empty directories, even if another writer occupies the path.
pub(super) fn with_destination(path: &Path, publish: impl FnOnce() -> Result<()>) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| ProjectError("The asset destination has no parent".into()))?;
    require(
        canonical_path(parent)? == parent,
        "The asset destination was redirected",
    )?;
    let missing: Vec<_> = parent
        .ancestors()
        .take_while(|path| !path.exists())
        .collect();
    let mut created = Vec::new();
    let result = (|| {
        for directory in missing.into_iter().rev() {
            match fs::create_dir(directory) {
                Ok(()) => created.push(directory),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            require(
                directory.canonicalize()? == directory,
                "The asset destination was redirected",
            )?;
        }
        publish()
    })();
    if result.is_err() {
        for directory in created.into_iter().rev() {
            let _ = fs::remove_dir(directory);
        }
    }
    result
}

pub(super) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    require(
        fs::metadata(path)?.is_file(),
        "The handoff source must be a regular file",
    )?;
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    require(
        before.is_file() && before.len() <= limit,
        "The handoff source exceeds the review size limit",
    )?;
    let mut bytes = Vec::new();
    (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    require(
        bytes.len() as u64 == before.len()
            && after.len() == before.len()
            && after.modified()? == before.modified()?,
        "The handoff source changed during review",
    )?;
    Ok(bytes)
}
