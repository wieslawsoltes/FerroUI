use super::File;
use crate::{workspace::atomic_create, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Claim the new directory exclusively. A project is discoverable by Cargo only
/// after its manifest has been published, following the source and build script.
pub(super) fn create(root: &Path, files: &[File]) -> Result<()> {
    fs::create_dir(root)?;
    let mut created = CreatedPaths {
        directories: vec![root.to_owned()],
        files: Vec::new(),
    };
    for file in files {
        let path = root.join(file.path);
        if let Some(parent) = path.parent().filter(|parent| *parent != root) {
            if !created.directories.iter().any(|path| path == parent) {
                fs::create_dir(parent)?;
                created.directories.push(parent.to_owned());
            }
        }
        atomic_create(&path, &file.source)?;
        created.files.push((path, &file.source));
    }
    created.files.clear();
    created.directories.clear();
    Ok(())
}

struct CreatedPaths<'a> {
    directories: Vec<PathBuf>,
    files: Vec<(PathBuf, &'a str)>,
}

impl Drop for CreatedPaths<'_> {
    fn drop(&mut self) {
        for (path, source) in self.files.iter().rev() {
            if fs::read(path).is_ok_and(|bytes| bytes == source.as_bytes()) {
                let _ = fs::remove_file(path);
            }
        }
        for path in self.directories.iter().rev() {
            let _ = fs::remove_dir(path);
        }
    }
}
