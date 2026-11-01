//! Ordered XDG desktop IDs shared by the launcher, taskbar and dock.
use super::preferences::{config_path, replace};
use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum Edit {
    Pin,
    Unpin,
    Earlier,
    Later,
}

pub fn load() -> io::Result<Vec<String>> {
    read(&config_path("favorites.json")?)
}

fn read(path: &Path) -> io::Result<Vec<String>> {
    match fs::read(path) {
        Ok(bytes) => {
            let ids = serde_json::from_slice::<Vec<String>>(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let mut unique = std::collections::HashSet::new();
            Ok(ids
                .into_iter()
                .filter(|id| !id.is_empty() && unique.insert(id.clone()))
                .collect())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

pub fn update(id: &str, edit: Edit) -> io::Result<Vec<String>> {
    update_at(&config_path("favorites.json")?, id, edit)
}

fn update_at(path: &Path, id: &str, edit: Edit) -> io::Result<Vec<String>> {
    // A nested preview and a login shell may edit simultaneously. Transform the
    // latest contents under a stable lock (the data file itself gets renamed).
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| io::Error::other("Missing favorites directory"))?,
    )?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path.with_extension("lock"))?;
    lock.lock()?;
    let mut ids = read(path)?;
    match (edit, ids.iter().position(|item| item == id)) {
        (Edit::Pin, None) => ids.push(id.to_owned()),
        (Edit::Unpin, Some(i)) => {
            ids.remove(i);
        }
        (Edit::Earlier, Some(i)) if i > 0 => ids.swap(i, i - 1),
        (Edit::Later, Some(i)) if i + 1 < ids.len() => ids.swap(i, i + 1),
        _ => return Ok(ids),
    }
    let bytes = serde_json::to_vec(&ids).map_err(io::Error::other)?;
    replace(path, &bytes)?;
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_pins_reorder_and_unpin_without_losing_unrelated_edits() {
        let dir = std::env::temp_dir().join(format!("ferro-pin-store-{}", std::process::id()));
        let path = dir.join("favorites.json");
        assert_eq!(read(&path).unwrap(), Vec::<String>::new());
        std::thread::scope(|scope| {
            for id in ["a.desktop", "b.desktop"] {
                let path = &path;
                scope.spawn(move || update_at(path, id, Edit::Pin).unwrap());
            }
        });
        let before = read(&path).unwrap();
        assert_eq!(before.len(), 2);
        let after = update_at(&path, &before[1], Edit::Earlier).unwrap();
        assert_eq!(after, [before[1].clone(), before[0].clone()]);
        assert_eq!(update_at(&path, &after[0], Edit::Pin).unwrap(), after);
        assert_eq!(update_at(&path, &after[0], Edit::Later).unwrap(), before);
        update_at(&path, &before[0], Edit::Unpin).unwrap();
        assert_eq!(read(&path).unwrap(), [before[1].clone()]);
        fs::write(&path, "corrupt").unwrap();
        assert!(update_at(&path, "c.desktop", Edit::Pin).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"corrupt");
        fs::remove_dir_all(dir).unwrap();
    }
}
