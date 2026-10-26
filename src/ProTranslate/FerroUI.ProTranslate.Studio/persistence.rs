use crate::{StudioDocument, StudioError, TranslationSession};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageEntry {
    pub path: String,
    pub size: u64,
}

/// Relative paths are scoped by the host's storage instance. Writes are atomic:
/// errors must leave the old destination intact. Missing files are `Ok(None)`.
pub trait StudioStorage {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, StudioError>;
    fn write_atomic(&self, path: &str, bytes: &[u8]) -> Result<(), StudioError>;
    /// Atomically create a complete file only if absent; false never replaces it.
    fn write_new_atomic(&self, path: &str, bytes: &[u8]) -> Result<bool, StudioError>;
    fn remove(&self, path: &str) -> Result<(), StudioError>;
    fn list(&self, directory: &str) -> Result<Vec<StorageEntry>, StudioError>;
}

#[derive(Clone, Default)]
pub struct MemoryStudioStorage {
    files: Rc<RefCell<BTreeMap<String, Vec<u8>>>>,
}
impl StudioStorage for MemoryStudioStorage {
    fn write_new_atomic(&self, path: &str, bytes: &[u8]) -> Result<bool, StudioError> {
        validate_path(path)?;
        match self.files.borrow_mut().entry(path.to_owned()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bytes.to_vec());
                Ok(true)
            }
            std::collections::btree_map::Entry::Occupied(_) => Ok(false),
        }
    }
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, StudioError> {
        validate_path(path)?;
        Ok(self.files.borrow().get(path).cloned())
    }
    fn write_atomic(&self, path: &str, bytes: &[u8]) -> Result<(), StudioError> {
        validate_path(path)?;
        self.files
            .borrow_mut()
            .insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }
    fn remove(&self, path: &str) -> Result<(), StudioError> {
        validate_path(path)?;
        self.files.borrow_mut().remove(path);
        Ok(())
    }
    fn list(&self, directory: &str) -> Result<Vec<StorageEntry>, StudioError> {
        validate_path(directory)?;
        let prefix = format!("{directory}/");
        Ok(self
            .files
            .borrow()
            .iter()
            .filter(|(path, _)| {
                path.strip_prefix(&prefix)
                    .is_some_and(|suffix| !suffix.contains('/'))
            })
            .map(|(path, bytes)| StorageEntry {
                path: path.clone(),
                size: bytes.len() as u64,
            })
            .collect())
    }
}

fn validate_path(path: &str) -> Result<(), StudioError> {
    if path.is_empty()
        || path.contains(['\\', ':', '\0'])
        || path.split('/').any(|part| {
            part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' '])
        })
    {
        return Err(StudioError::InvalidInput(
            "Storage paths must be nonempty relative paths without traversal or components ending in a dot or space".into(),
        ));
    }
    Ok(())
}

#[cfg(all(feature = "filesystem", not(target_arch = "wasm32")))]
mod filesystem {
    use super::*;
    use std::{
        fs,
        io::{ErrorKind, Write},
        path::{Path, PathBuf},
    };

    /// Explicit host directory; no application-data discovery or global state.
    pub struct FileStudioStorage {
        root: PathBuf,
        sequence: Cell<u64>,
    }
    impl FileStudioStorage {
        pub fn new(root: impl Into<PathBuf>) -> Result<Self, StudioError> {
            let root = root.into();
            if !root.is_absolute() {
                return Err(StudioError::InvalidInput(
                    "Filesystem storage root must be absolute".into(),
                ));
            }
            Ok(Self {
                root,
                sequence: Cell::new(0),
            })
        }
        pub fn root(&self) -> &Path {
            &self.root
        }
        fn path(&self, path: &str) -> Result<PathBuf, StudioError> {
            validate_path(path)?;
            Ok(self.root.join(path))
        }
        fn write_file(&self, path: &str, bytes: &[u8], replace: bool) -> Result<bool, StudioError> {
            let destination = self.path(path)?;
            let parent = destination
                .parent()
                .ok_or_else(|| StudioError::InvalidInput("File has no parent".into()))?;
            fs::create_dir_all(parent).map_err(storage_error)?;
            let file_name = destination
                .file_name()
                .ok_or_else(|| StudioError::InvalidInput("File name missing".into()))?
                .to_string_lossy();
            let (temporary, mut file) = loop {
                let next = self.sequence.get().checked_add(1).ok_or_else(|| {
                    StudioError::Storage("Temporary file identities exhausted".into())
                })?;
                self.sequence.set(next);
                let temporary = parent.join(format!(".{file_name}.studio-{next}.tmp"));
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)
                {
                    Ok(file) => break (temporary, file),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(storage_error(error)),
                }
            };
            let written = file.write_all(bytes).and_then(|()| file.sync_all());
            drop(file);
            let result = written.and_then(|()| {
                if replace {
                    fs::rename(&temporary, &destination)
                } else {
                    fs::hard_link(&temporary, &destination)
                }
            });
            // The destination owns a complete inode before its temporary name is removed.
            let _ = fs::remove_file(&temporary);
            match result {
                Ok(()) => Ok(true),
                Err(error) if !replace && error.kind() == ErrorKind::AlreadyExists => Ok(false),
                Err(error) => Err(storage_error(error)),
            }
        }
    }
    fn storage_error(error: std::io::Error) -> StudioError {
        StudioError::Storage(error.to_string())
    }
    impl StudioStorage for FileStudioStorage {
        fn read(&self, path: &str) -> Result<Option<Vec<u8>>, StudioError> {
            match fs::read(self.path(path)?) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(error) => Err(storage_error(error)),
            }
        }
        fn write_atomic(&self, path: &str, bytes: &[u8]) -> Result<(), StudioError> {
            self.write_file(path, bytes, true).map(|_| ())
        }
        fn write_new_atomic(&self, path: &str, bytes: &[u8]) -> Result<bool, StudioError> {
            self.write_file(path, bytes, false)
        }
        fn remove(&self, path: &str) -> Result<(), StudioError> {
            match fs::remove_file(self.path(path)?) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                Err(error) => Err(storage_error(error)),
            }
        }
        fn list(&self, directory: &str) -> Result<Vec<StorageEntry>, StudioError> {
            let entries = match fs::read_dir(self.path(directory)?) {
                Ok(entries) => entries,
                Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
                Err(error) => return Err(storage_error(error)),
            };
            let mut result = Vec::new();
            for entry in entries {
                let entry = entry.map_err(storage_error)?;
                if entry.file_type().map_err(storage_error)?.is_file() {
                    let name = entry
                        .file_name()
                        .into_string()
                        .map_err(|_| StudioError::Storage("Non-UTF8 file name".into()))?;
                    result.push(StorageEntry {
                        path: format!("{directory}/{name}"),
                        size: entry.metadata().map_err(storage_error)?.len(),
                    });
                }
            }
            result.sort_by(|a, b| a.path.cmp(&b.path));
            Ok(result)
        }
    }
}
#[cfg(all(feature = "filesystem", not(target_arch = "wasm32")))]
pub use filesystem::FileStudioStorage;

/// Names inside a storage root, supplied once by the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StudioPaths {
    sessions_directory: String,
    recovery_path: String,
    backups_directory: String,
    settings_path: String,
}
impl StudioPaths {
    pub fn sessions_directory(&self) -> &str {
        &self.sessions_directory
    }
    pub fn recovery_path(&self) -> &str {
        &self.recovery_path
    }
    pub fn backups_directory(&self) -> &str {
        &self.backups_directory
    }
    pub fn settings_path(&self) -> &str {
        &self.settings_path
    }
    pub fn new(
        sessions_directory: impl Into<String>,
        recovery_path: impl Into<String>,
        backups_directory: impl Into<String>,
        settings_path: impl Into<String>,
    ) -> Result<Self, StudioError> {
        let paths = Self {
            sessions_directory: sessions_directory.into(),
            recovery_path: recovery_path.into(),
            backups_directory: backups_directory.into(),
            settings_path: settings_path.into(),
        };
        for path in [
            &paths.sessions_directory,
            &paths.recovery_path,
            &paths.backups_directory,
            &paths.settings_path,
        ] {
            validate_path(path)?;
        }
        let locations = [
            &paths.sessions_directory,
            &paths.recovery_path,
            &paths.backups_directory,
            &paths.settings_path,
        ]
        // Compare conservatively across case-insensitive native filesystems.
        // Unicode case mappings also cover expansions and final-sigma forms;
        // the actual host paths retain their original spelling.
        .map(|path| path.to_uppercase().to_lowercase());
        for (index, left) in locations.iter().enumerate() {
            for right in &locations[index + 1..] {
                if left == right
                    || left.starts_with(&format!("{right}/"))
                    || right.starts_with(&format!("{left}/"))
                {
                    return Err(StudioError::InvalidInput("Studio storage locations must be disjoint; paths cannot be equal or nested, including case aliases".into()));
                }
            }
        }
        Ok(paths)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveOutcome {
    pub path: String,
    pub saved_revision: u64,
    pub document_is_clean: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupMetadata {
    pub path: String,
    pub created_at_millis: Option<i64>,
    pub sequence: u64,
    pub size: u64,
    pub entry_count: Option<usize>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupRecord {
    schema_version: u32,
    created_at_millis: i64,
    sequence: u64,
    session: TranslationSession,
}

/// Session, recovery and backup storage share the same explicit host boundary.
pub struct SessionService {
    storage: Rc<dyn StudioStorage>,
    paths: StudioPaths,
    backup_sequence: Cell<u64>,
}
impl SessionService {
    pub fn new(storage: Rc<dyn StudioStorage>, paths: StudioPaths) -> Self {
        Self {
            storage,
            paths,
            backup_sequence: Cell::new(0),
        }
    }
    pub fn storage(&self) -> &Rc<dyn StudioStorage> {
        &self.storage
    }
    pub fn paths(&self) -> &StudioPaths {
        &self.paths
    }
    pub fn default_session_path(
        &self,
        session: &TranslationSession,
    ) -> Result<String, StudioError> {
        crate::document::validate_session_id(&session.session_id)?;
        Ok(format!(
            "{}/{}.json",
            self.paths.sessions_directory, session.session_id
        ))
    }
    pub fn save_snapshot(
        &self,
        session: &TranslationSession,
        path: Option<&str>,
    ) -> Result<String, StudioError> {
        let path = path
            .map(str::to_owned)
            .map_or_else(|| self.default_session_path(session), Ok)?;
        self.storage
            .write_atomic(&path, session.to_json()?.as_bytes())?;
        Ok(path)
    }
    pub fn save_session(
        &self,
        document: &mut StudioDocument,
        path: Option<&str>,
        now_millis: i64,
    ) -> Result<SaveOutcome, StudioError> {
        let snapshot = document.snapshot(now_millis);
        let path = self.save_snapshot(&snapshot, path)?;
        let document_is_clean = document.mark_saved(&snapshot);
        Ok(SaveOutcome {
            path,
            saved_revision: snapshot.revision,
            document_is_clean,
        })
    }
    pub fn load_session(&self, path: &str) -> Result<Option<TranslationSession>, StudioError> {
        self.storage
            .read(path)?
            .map(|bytes| decode_session(&bytes))
            .transpose()
    }
    pub fn save_recovery(
        &self,
        document: &StudioDocument,
        now_millis: i64,
    ) -> Result<(), StudioError> {
        self.storage.write_atomic(
            &self.paths.recovery_path,
            document.snapshot(now_millis).to_json()?.as_bytes(),
        )
    }
    pub fn recovery_session(&self) -> Result<Option<TranslationSession>, StudioError> {
        self.load_session(&self.paths.recovery_path)
    }
    pub fn clear_recovery(&self) -> Result<(), StudioError> {
        self.storage.remove(&self.paths.recovery_path)
    }
    pub fn restore_recovery(&self, document: &mut StudioDocument) -> Result<bool, StudioError> {
        let Some(session) = self.recovery_session()? else {
            return Ok(false);
        };
        document.restore(session, true)?;
        Ok(true)
    }
    pub fn create_backup(
        &self,
        session: &TranslationSession,
        now_millis: i64,
    ) -> Result<BackupMetadata, StudioError> {
        session.validate()?;
        let maximum = self
            .list_backups()?
            .iter()
            .map(|backup| backup.sequence)
            .max()
            .unwrap_or(0);
        self.backup_sequence
            .set(self.backup_sequence.get().max(maximum));
        let (path, sequence, bytes) = loop {
            let sequence = self
                .backup_sequence
                .get()
                .checked_add(1)
                .ok_or_else(|| StudioError::Storage("Backup sequence exhausted".into()))?;
            self.backup_sequence.set(sequence);
            let path = format!(
                "{}/backup_{now_millis:020}_{sequence:016}.json",
                self.paths.backups_directory
            );
            let bytes = serde_json::to_vec_pretty(&BackupRecord {
                schema_version: 1,
                created_at_millis: now_millis,
                sequence,
                session: session.clone(),
            })?;
            if self.storage.write_new_atomic(&path, &bytes)? {
                break (path, sequence, bytes);
            }
        };
        Ok(BackupMetadata {
            path,
            created_at_millis: Some(now_millis),
            sequence,
            size: bytes.len() as u64,
            entry_count: Some(session.catalog.entries.len()),
            error: None,
        })
    }
    pub fn list_backups(&self) -> Result<Vec<BackupMetadata>, StudioError> {
        let mut result = Vec::new();
        for entry in self.storage.list(&self.paths.backups_directory)? {
            let file = entry.path.rsplit('/').next().unwrap_or("");
            if !file.starts_with("backup_") || !file.ends_with(".json") {
                continue;
            }
            let Some(bytes) = self.storage.read(&entry.path)? else {
                continue;
            };
            let parsed = decode_backup(&bytes);
            let (created_at_millis, sequence, entry_count, error) = match parsed {
                Ok(record) => (
                    Some(record.created_at_millis),
                    record.sequence,
                    Some(record.session.catalog.entries.len()),
                    None,
                ),
                Err(error) => (None, 0, None, Some(error.to_string())),
            };
            result.push(BackupMetadata {
                path: entry.path,
                created_at_millis,
                sequence,
                size: entry.size,
                entry_count,
                error,
            });
        }
        result.sort_by(|a, b| {
            b.created_at_millis
                .cmp(&a.created_at_millis)
                .then_with(|| b.sequence.cmp(&a.sequence))
                .then_with(|| b.path.cmp(&a.path))
        });
        Ok(result)
    }
    pub fn restore_backup(&self, path: &str) -> Result<Option<TranslationSession>, StudioError> {
        self.ensure_backup_path(path)?;
        self.storage
            .read(path)?
            .map(|bytes| decode_backup(&bytes).map(|record| record.session))
            .transpose()
    }
    pub fn delete_backup(&self, path: &str) -> Result<(), StudioError> {
        self.ensure_backup_path(path)?;
        self.storage.remove(path)
    }
    pub fn rotate_backups(&self, max_count: usize) -> Result<usize, StudioError> {
        let backups = self.list_backups()?;
        let mut removed = 0;
        for backup in backups.into_iter().skip(max_count) {
            self.delete_backup(&backup.path)?;
            removed += 1;
        }
        Ok(removed)
    }
    /// Rotation runs only once a complete replacement backup is safely stored.
    pub fn backup_and_rotate(
        &self,
        session: &TranslationSession,
        now_millis: i64,
        max_count: usize,
    ) -> Result<BackupMetadata, StudioError> {
        if max_count == 0 {
            return Err(StudioError::InvalidInput(
                "Backup retention must be positive".into(),
            ));
        }
        let backup = self.create_backup(session, now_millis)?;
        self.rotate_backups(max_count)?;
        Ok(backup)
    }
    fn ensure_backup_path(&self, path: &str) -> Result<(), StudioError> {
        validate_path(path)?;
        let suffix = path
            .strip_prefix(&format!("{}/", self.paths.backups_directory))
            .ok_or_else(|| {
                StudioError::InvalidInput("Path is outside the backup directory".into())
            })?;
        if suffix.contains('/') || !suffix.starts_with("backup_") || !suffix.ends_with(".json") {
            return Err(StudioError::InvalidInput(
                "Path is not a Studio backup".into(),
            ));
        }
        Ok(())
    }
}

fn decode_session(bytes: &[u8]) -> Result<TranslationSession, StudioError> {
    TranslationSession::from_json(
        std::str::from_utf8(bytes)
            .map_err(|error| StudioError::Serialization(error.to_string()))?,
    )
}
fn decode_backup(bytes: &[u8]) -> Result<BackupRecord, StudioError> {
    let record: BackupRecord = serde_json::from_slice(bytes)?;
    if record.schema_version != 1 {
        return Err(StudioError::Serialization(
            "Unsupported backup schema".into(),
        ));
    }
    record.session.validate()?;
    Ok(record)
}

/// Host-driven scheduling: call `tick` from a UI timer or an executor. No worker
/// threads borrow a document, and failed saves remain due for the next tick.
pub struct AutoSaveScheduler {
    enabled: bool,
    interval_millis: i64,
    next_due: Option<i64>,
}
impl AutoSaveScheduler {
    pub fn new(interval_seconds: u32) -> Result<Self, StudioError> {
        if !(10..=300).contains(&interval_seconds) {
            return Err(StudioError::InvalidInput(
                "Autosave interval must be 10–300 seconds".into(),
            ));
        }
        Ok(Self {
            enabled: false,
            interval_millis: i64::from(interval_seconds) * 1000,
            next_due: None,
        })
    }
    pub fn start(&mut self, now_millis: i64) {
        self.enabled = true;
        self.next_due = Some(now_millis.saturating_add(self.interval_millis));
    }
    pub fn stop(&mut self) {
        self.enabled = false;
        self.next_due = None;
    }
    pub fn is_running(&self) -> bool {
        self.enabled
    }
    pub fn tick(
        &mut self,
        now_millis: i64,
        document: &StudioDocument,
        sessions: &SessionService,
    ) -> Result<bool, StudioError> {
        if !self.enabled || self.next_due.is_none_or(|due| now_millis < due) {
            return Ok(false);
        }
        let saved = document.is_dirty();
        if saved {
            sessions.save_recovery(document, now_millis)?;
        }
        self.next_due = Some(now_millis.saturating_add(self.interval_millis));
        Ok(saved)
    }
}
