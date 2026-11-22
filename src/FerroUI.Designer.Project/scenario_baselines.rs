//! Scoped screenshot files and Git references for native scenario review.
use crate::{file_path, file_uri, require, ProjectError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[path = "scenario_baselines/git.rs"]
mod git;
pub use git::Repository;

#[cfg(test)]
#[path = "scenario_baselines/tests.rs"]
mod tests;

const MAX_PNG_BYTES: u64 = 16 * 1024 * 1024;

/// Resolve an existing baseline or a destination that has yet to be created.
pub fn destination(base: &Path, workspace: &Path, name: &str) -> Result<PathBuf> {
    let path = file_uri(base.join(name)).and_then(|uri| file_path(&uri))?;
    require(
        path.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png")),
        "Screenshot baselines require a .png path",
    )?;
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                let resolved = ancestor.canonicalize()?;
                let relative = path.strip_prefix(ancestor).expect("ancestor of path");
                require(
                    resolved.starts_with(workspace),
                    "Screenshot baselines must be inside the workspace",
                )?;
                require(
                    if relative.as_os_str().is_empty() {
                        resolved.is_file()
                    } else {
                        resolved.is_dir()
                    },
                    "A screenshot baseline needs a file path",
                )?;
                return Ok(if relative.as_os_str().is_empty() {
                    resolved
                } else {
                    resolved.join(relative)
                });
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(ProjectError("Baseline directory is unavailable".into()))
}

pub fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ProjectError(format!("{}: {error}", path.display()))),
    };
    let mut bytes = Vec::new();
    file.take(MAX_PNG_BYTES + 1).read_to_end(&mut bytes)?;
    require(
        bytes.len() as u64 <= MAX_PNG_BYTES,
        "Baseline PNG exceeds 16 MiB",
    )?;
    Ok(Some(bytes))
}

/// Publish an approved capture while checking the source and accepted baseline
/// immediately before the atomic update. Serialize writers for the same destination.
pub fn publish(
    workspace: &Path,
    path: &Path,
    reviewed: Option<&[u8]>,
    image: &[u8],
    validate: impl FnOnce() -> Result<()>,
) -> Result<()> {
    require(
        path.starts_with(workspace),
        "Screenshot baselines must be inside the workspace",
    )?;
    let directory = std::env::temp_dir().join("ferroui-scenario-locks");
    fs::create_dir_all(&directory)?;
    let key = ferroui_reload_protocol::fingerprint(path.as_os_str().as_encoded_bytes());
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(format!("{key:016x}")))?;
    lock.lock()?;
    let file = path
        .to_str()
        .ok_or_else(|| ProjectError("Use a UTF-8 baseline path".into()))?;
    let check_destination = || {
        require(
            destination(workspace, workspace, file)? == path,
            "The baseline destination changed. Review the current capture again.",
        )
    };
    check_destination()?;
    fs::create_dir_all(path.parent().expect("baseline parent"))?;
    crate::workspace::publish_bytes(path, image, reviewed.is_some(), || {
        check_destination()?;
        require(
            read(path)?.as_deref() == reviewed,
            "The accepted baseline changed. Review the current capture again.",
        )?;
        validate()
    })?;
    Ok(())
}
