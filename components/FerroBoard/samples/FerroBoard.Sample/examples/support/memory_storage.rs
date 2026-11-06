//! Native RAM-backed sample storage for isolated validation; no filesystem emulation.
use ferroboard_sample::sample_storage::{SETTINGS_KEY, SampleStoragePlatform, StorageResult};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Default)]
pub struct MemoryStorage {
    pub settings: RefCell<HashMap<String, String>>,
    pub documents: RefCell<HashMap<PathBuf, String>>,
    pub reads: Cell<usize>,
    pub writes: Cell<usize>,
    pub errors: RefCell<Vec<String>>,
}
impl MemoryStorage {
    pub fn private() -> Rc<Self> {
        let settings = ferroboard_core::settings::KeyboardSettings::default();
        settings.set_incognito_mode(true);
        settings.set_learn_words(false);
        settings.set_emoji_history_enabled(false);
        settings.set_clipboard_history_enabled(false);
        settings.set_clipboard_suggestion(false);
        let storage = Rc::new(Self::default());
        storage
            .settings
            .borrow_mut()
            .insert(SETTINGS_KEY.into(), settings.to_json().unwrap());
        storage
    }
}
impl SampleStoragePlatform for MemoryStorage {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.settings.borrow().get(key).cloned())
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.writes.set(self.writes.get() + 1);
        self.settings.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(None)
    }
    fn read_document(&self, name: &Path) -> StorageResult<Option<String>> {
        Ok(self.documents.borrow().get(name).cloned())
    }
    fn write_document(&self, name: &Path, value: &str) -> StorageResult<()> {
        self.documents
            .borrow_mut()
            .insert(name.to_owned(), value.into());
        Ok(())
    }
    fn log(&self, message: &str) {
        self.errors.borrow_mut().push(message.into());
        eprintln!("Memory sample storage: {message}");
    }
}
