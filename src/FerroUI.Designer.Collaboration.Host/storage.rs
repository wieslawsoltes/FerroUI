use crate::model::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
#[cfg(test)]
static FAIL_DIRECTORY_SYNC: std::sync::Mutex<std::collections::BTreeSet<PathBuf>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

#[cfg(test)]
pub(crate) fn fail_next_directory_sync(path: PathBuf) {
    FAIL_DIRECTORY_SYNC.lock().unwrap().insert(path);
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    checksum: String,
    payload: String,
}

// The payload is itself JSON, so escaping its quotes/backslashes can double its size. Include
// the outer version/checksum/property names as well; files written at the payload limit must
// remain readable after restart.
const MAX_ENVELOPE_BYTES: usize = MAX_ROOM_BYTES * 2 + 256;

pub(crate) fn private_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).read(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
pub(crate) fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::symlink_metadata(path).map_err(HostError::storage)?;
    ensure(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        ErrorCode::Storage,
        "State path must be a regular file",
    )?;
    ensure(
        metadata.len() <= MAX_ENVELOPE_BYTES as u64,
        ErrorCode::Limit,
        "State file exceeds the room limit",
    )?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(HostError::storage)?
        .take(MAX_ENVELOPE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(HostError::storage)?;
    ensure(
        bytes.len() <= MAX_ENVELOPE_BYTES,
        ErrorCode::Limit,
        "State file exceeds the room limit",
    )?;
    let envelope: Envelope = serde_json::from_slice(&bytes).map_err(HostError::storage)?;
    ensure(
        envelope.payload.len() <= MAX_ROOM_BYTES,
        ErrorCode::Limit,
        "State payload exceeds the room limit",
    )?;
    ensure(
        envelope.version == HOST_PROTOCOL
            && equal_secret(&envelope.checksum, &hash(envelope.payload.as_bytes())),
        ErrorCode::Storage,
        "State checksum/version mismatch; preserve the file and restore a verified backup",
    )?;
    serde_json::from_str(&envelope.payload).map_err(HostError::storage)
}
pub(crate) fn write<T: Serialize>(path: &Path, value: &T, fence: &dyn CommitFence) -> Result<()> {
    let payload = serde_json::to_string(value).map_err(HostError::storage)?;
    ensure(
        payload.len() <= MAX_ROOM_BYTES,
        ErrorCode::Limit,
        "Room exceeds the durable size limit; checkpoint after all members synchronize",
    )?;
    let bytes = serde_json::to_vec(&Envelope {
        version: HOST_PROTOCOL,
        checksum: hash(payload.as_bytes()),
        payload,
    })
    .map_err(HostError::storage)?;
    let parent = path
        .parent()
        .ok_or_else(|| HostError::storage("Storage parent missing"))?;
    let temporary = parent.join(format!(".{}.tmp", Secret::generate()?.expose()));
    let result = (|| {
        let mut file = private_file(&temporary).map_err(HostError::storage)?;
        file.write_all(&bytes).map_err(HostError::storage)?;
        file.sync_all().map_err(HostError::storage)?;
        fence.check()?;
        fs::rename(&temporary, path).map_err(HostError::storage)?;
        #[cfg(test)]
        let injected_fault = FAIL_DIRECTORY_SYNC.lock().unwrap().remove(path);
        #[cfg(test)]
        if injected_fault {
            return Err(HostError::new(
                ErrorCode::Indeterminate,
                "Simulated directory sync failure after rename",
            ));
        }
        #[cfg(unix)]
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error|HostError::new(ErrorCode::Indeterminate,format!("State was replaced but directory durability is uncertain; restart the store before more requests: {error}")))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub(crate) fn open_directory(path: &Path) -> Result<(PathBuf, File)> {
    fs::create_dir_all(path).map_err(HostError::storage)?;
    let metadata = fs::symlink_metadata(path).map_err(HostError::storage)?;
    ensure(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        ErrorCode::Storage,
        "Storage must be a real directory",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(HostError::storage)?;
    }
    let root = fs::canonicalize(path).map_err(HostError::storage)?;
    let lock_path = root.join("server.lock");
    let lock = if lock_path.exists() {
        ensure(
            !fs::symlink_metadata(&lock_path)
                .map_err(HostError::storage)?
                .file_type()
                .is_symlink(),
            ErrorCode::Storage,
            "Lock cannot be a symbolic link",
        )?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(HostError::storage)?
    } else {
        private_file(&lock_path).map_err(HostError::storage)?
    };
    lock.try_lock().map_err(|error| {
        HostError::new(
            ErrorCode::Storage,
            format!("Room storage is already locked or unavailable: {error}"),
        )
    })?;
    Ok((root, lock))
}

/// Explicitly save a credential/invitation checkpoint with owner-only permissions and an atomic
/// replace. The caller chooses a private path; credentials are never written into project XAML.
pub fn save_private<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write(path, value, &no_cancel)
}
pub fn load_private<T: DeserializeOwned>(path: &Path) -> Result<T> {
    read(path)
}
