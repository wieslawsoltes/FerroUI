//! Adjacent save candidates are exclusively owned before cleanup is permitted.
use crate::{ProjectError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

pub(super) fn atomic_create(path: &Path, source: &str) -> Result<()> {
    publish(path, source, false)
}

pub(crate) fn atomic_write(path: &Path, source: &str) -> Result<()> {
    publish(path, source, true)
}

fn publish(path: &Path, source: &str, replace: bool) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| ProjectError("File has no parent".into()))?;
    let candidates = (0..32).map(|_| {
        parent.join(format!(
            ".designer-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ))
    });
    let (temporary, mut file) = reserve_candidate(candidates)?;
    // Only the successfully reserved candidate belongs to us. In particular a
    // stale PID/counter collision must never delete another candidate's bytes.
    let result = (|| -> io::Result<()> {
        file.write_all(source.as_bytes())?;
        if replace {
            if let Ok(metadata) = fs::metadata(path) {
                file.set_permissions(metadata.permissions())?;
            }
        }
        file.sync_all()?;
        drop(file);
        if replace {
            fs::rename(&temporary, path)
        } else {
            // A new pathname is published atomically only if it is still absent.
            // Unlike rename(), a hard link cannot clobber a file created after
            // the destination check. Unsupported filesystems fail without loss.
            fs::hard_link(&temporary, path)
        }
    })();
    let _ = fs::remove_file(&temporary);
    result.map_err(Into::into)
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
