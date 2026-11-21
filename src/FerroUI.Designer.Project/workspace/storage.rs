//! Adjacent save candidates are exclusively owned before cleanup is permitted.
use crate::{ProjectError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

pub(super) fn atomic_create(path: &Path, source: &str) -> Result<()> {
    atomic_create_with_metadata(path, source).map(|_| ())
}

pub(super) fn atomic_create_with_metadata(path: &Path, source: &str) -> Result<fs::Metadata> {
    publish(path, source, false, || Ok(()))
}

pub(crate) fn atomic_write(path: &Path, source: &str) -> Result<()> {
    atomic_write_checked(path, source, || Ok(())).map(|_| ())
}

pub(super) fn atomic_write_checked(
    path: &Path,
    source: &str,
    check: impl FnOnce() -> Result<()>,
) -> Result<fs::Metadata> {
    publish(path, source, true, check)
}

pub(super) fn backup_file(path: &Path) -> Result<PathBuf> {
    for candidate in candidates(path)? {
        match fs::hard_link(path, &candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(ProjectError("Could not reserve a file move backup".into()))
}

fn candidates(path: &Path) -> Result<impl Iterator<Item = PathBuf> + '_> {
    let parent = path
        .parent()
        .ok_or_else(|| ProjectError("File has no parent".into()))?;
    Ok((0..32).map(move |_| {
        parent.join(format!(
            ".designer-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ))
    }))
}

fn publish(
    path: &Path,
    source: &str,
    replace: bool,
    check: impl FnOnce() -> Result<()>,
) -> Result<fs::Metadata> {
    let (temporary, mut file) = reserve_candidate(candidates(path)?)?;
    // Only the successfully reserved candidate belongs to us. In particular a
    // stale PID/counter collision must never delete another candidate's bytes.
    let result = (|| -> Result<fs::Metadata> {
        file.write_all(source.as_bytes())?;
        if replace {
            if let Ok(metadata) = fs::metadata(path) {
                file.set_permissions(metadata.permissions())?;
            }
        }
        file.sync_all()?;
        let metadata = file.metadata()?;
        drop(file);
        check()?;
        if replace {
            fs::rename(&temporary, path)?;
        } else {
            // A new pathname is published atomically only if it is still absent.
            // Unlike rename(), a hard link cannot clobber a file created after
            // the destination check. Unsupported filesystems fail without loss.
            fs::hard_link(&temporary, path)?;
        }
        Ok(metadata)
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn reserve_candidate(paths: impl IntoIterator<Item = PathBuf>) -> io::Result<(PathBuf, File)> {
    for path in paths {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not reserve a save candidate",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collided_candidates_are_never_removed_and_new_saves_never_replace_files() {
        let directory =
            std::env::temp_dir().join(format!("ferroui-save-ownership-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let occupied = directory.join("occupied.tmp");
        let available = directory.join("available.tmp");
        let destination = directory.join("View.xaml");
        fs::write(&occupied, "belongs to another writer").unwrap();
        assert!(reserve_candidate([occupied.clone()]).is_err());
        let (reserved, file) = reserve_candidate([occupied.clone(), available.clone()]).unwrap();
        assert_eq!(reserved, available);
        drop(file);
        assert_eq!(
            fs::read_to_string(&occupied).unwrap(),
            "belongs to another writer"
        );
        atomic_create(&destination, "<Grid/>").unwrap();
        assert!(atomic_create(&destination, "<Button/>").is_err());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "<Grid/>");
        atomic_write(&destination, "<Border/>").unwrap();
        assert_eq!(fs::read_to_string(&destination).unwrap(), "<Border/>");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 3);
        fs::remove_dir_all(directory).unwrap();
    }
}
