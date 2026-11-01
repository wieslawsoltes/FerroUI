use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) fn config_path(name: &str) -> io::Result<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(".config"))
        })
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No user configuration directory")
        })?;
    Ok(base.join("ferro-desktop").join(name))
}

pub(crate) fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing configuration directory"))?;
    fs::create_dir_all(dir)?;
    let (mut file, staging) = loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let staging = dir.join(format!(".write-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staging)
        {
            Ok(file) => break (file, staging),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = file
        .write_all(bytes)
        .and_then(|()| fs::rename(&staging, path));
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}
