//! SampleStorage.cs. Synchronous persistence; platform storage is supplied at composition time.
use ferroboard_core::{engine::KeyboardEngine, settings::KeyboardSettings};
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

pub type StorageResult<T> = Result<T, Box<dyn std::error::Error>>;
pub const SETTINGS_KEY: &str = "keyboard.settings";

/// Native synchronous settings/document storage supplied at the composition root.
pub trait SampleStoragePlatform {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>>;
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()>;
    fn local_folder(&self) -> StorageResult<Option<PathBuf>>;
    fn log(&self, message: &str);
    /// Native document storage. File platforms retain the original folder/error boundaries;
    /// browser hosts can use their own synchronous store without a virtual filesystem.
    fn read_document(&self, name: &Path) -> StorageResult<Option<String>> {
        if let Some(folder) = catch(|| self.local_folder()).ok().flatten() {
            let path = folder.join(name);
            if path.is_file() {
                return Ok(Some(read_all_text(&fs::read(path)?)));
            }
        }
        Ok(None)
    }
    fn write_document(&self, name: &Path, content: &str) -> StorageResult<()> {
        if let Some(folder) = catch(|| self.local_folder()).ok().flatten() {
            fs::create_dir_all(&folder)?;
            fs::write(folder.join(name), content)?;
        }
        Ok(())
    }
}

pub struct FileSampleStoragePlatform {
    folder: std::io::Result<PathBuf>,
}
impl FileSampleStoragePlatform {
    pub fn new(folder: PathBuf) -> Self {
        Self { folder: Ok(folder) }
    }
    /// Native per-user data directory, with FERROBOARD_DATA_DIR as an explicit override.
    /// An unavailable directory remains an error at the existing persistence boundary.
    pub fn from_environment() -> Self {
        Self {
            folder: crate::sample_data_paths::application_data_folder(),
        }
    }
    fn folder(&self) -> std::io::Result<&Path> {
        self.folder
            .as_deref()
            .map_err(|error| std::io::Error::new(error.kind(), error.to_string()))
    }
}
impl SampleStoragePlatform for FileSampleStoragePlatform {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        let path = self.folder()?.join(key);
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(read_all_text(&fs::read(path)?)))
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        fs::create_dir_all(self.folder()?)?;
        fs::write(self.folder()?.join(key), value)?;
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(Some(self.folder()?.to_path_buf()))
    }
    fn log(&self, message: &str) {
        println!("{message}");
    }
}

pub struct SampleStorage {
    platform: Rc<dyn SampleStoragePlatform>,
}
impl SampleStorage {
    pub fn new(platform: Rc<dyn SampleStoragePlatform>) -> Rc<Self> {
        Rc::new(Self { platform })
    }
    pub fn load(&self, engine: &KeyboardEngine) {
        self.boundary("Could not load settings", || {
            if let Some(json) = self.platform.get_setting(SETTINGS_KEY)? {
                engine
                    .settings()
                    .copy_from(&KeyboardSettings::from_json(&json)?)?;
            }
            Ok(())
        });
        self.read("emoji-history.json", |json| {
            engine
                .emoji_history()
                .import(json)
                .map_err(|e| format!("{e:?}").into())
        });
        self.read("clipboard.json", |json| {
            engine
                .clipboard_history()
                .import(json)
                .map_err(|e| format!("{e:?}").into())
        });
        self.read("user-dictionary-en.json", |json| {
            engine
                .language_models()
                .get_user_dictionary(&"en".into())
                .import(json)
                .map_err(|e| format!("{e:?}").into())
        });
    }
    pub fn save_settings(&self, settings: &KeyboardSettings) {
        self.boundary("Could not save settings", || {
            self.platform
                .set_setting(SETTINGS_KEY, &settings.to_json()?)
        });
    }
    pub fn save(
        &self,
        engine: &KeyboardEngine,
    ) -> Result<(), ferroboard_core::clipboard::ClipboardError> {
        self.save_settings(&engine.settings());
        // Export is intentionally evaluated OUTSIDE Write's exception boundary, as in C#.
        self.write("emoji-history.json", &engine.emoji_history().export());
        self.write("clipboard.json", &engine.clipboard_history().export(true)?);
        for (language, dictionary) in engine.language_models().user_dictionaries().iter() {
            self.write_path(&dictionary_path(language), &dictionary.export());
        }
        Ok(())
    }
    fn read(&self, name: &str, apply: impl FnOnce(&str) -> StorageResult<()>) {
        self.boundary(&format!("Could not read {name}"), || {
            if let Some(content) = self.platform.read_document(Path::new(name))? {
                apply(&content)?;
            }
            Ok(())
        });
    }
    fn write(&self, name: &str, content: &str) {
        self.write_path(Path::new(name), content);
    }
    fn write_path(&self, name: &Path, content: &str) {
        self.boundary(
            &format!("Could not write {}", name.to_string_lossy()),
            || self.platform.write_document(name, content),
        );
    }
    fn boundary(&self, label: &str, action: impl FnOnce() -> StorageResult<()>) {
        if let Err(error) = catch(action) {
            // Logging is outside the try, matching the source catch body.
            self.platform.log(&format!("{label}: {error}"));
        }
    }
}

fn dictionary_path(language: &ferroboard_core::text::Utf16Text) -> PathBuf {
    let units = "user-dictionary-"
        .encode_utf16()
        .chain(language.units().iter().copied())
        .chain(".json".encode_utf16())
        .collect::<Vec<_>>();
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&units).into()
    }
    #[cfg(not(windows))]
    {
        // System.IO's Unix path encoding replaces unpaired UTF-16 units. The unchanged C#
        // surrogate-filename case proves this bridge on the selected native platform.
        String::from_utf16_lossy(&units).into()
    }
}

fn catch<T>(action: impl FnOnce() -> StorageResult<T>) -> StorageResult<T> {
    // Source catches Exception, including synchronous exceptions from import/settings callbacks.
    // Rust callback panics unwind through this same intentional source error-handling boundary.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).unwrap_or_else(|error| {
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("storage callback panicked with a non-string payload");
        Err(message.to_owned().into())
    })
}

/// File.ReadAllText's BOM detection and replacement decoding, without requiring valid UTF-8 files.
fn read_all_text(bytes: &[u8]) -> String {
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe, 0, 0]) {
        return decode_utf32(bytes, true);
    }
    if let Some(bytes) = bytes.strip_prefix(&[0, 0, 0xfe, 0xff]) {
        return decode_utf32(bytes, false);
    }
    for (bom, little) in [([0xff, 0xfe], true), ([0xfe, 0xff], false)] {
        if let Some(bytes) = bytes.strip_prefix(&bom) {
            let (units, remainder) = bytes.as_chunks::<2>();
            let values = units
                .iter()
                .map(|b| {
                    if little {
                        u16::from_le_bytes([b[0], b[1]])
                    } else {
                        u16::from_be_bytes([b[0], b[1]])
                    }
                })
                .collect::<Vec<_>>();
            let mut text = String::from_utf16_lossy(&values);
            if !remainder.is_empty() {
                text.push('\u{fffd}');
            }
            return text;
        }
    }
    String::from_utf8_lossy(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)).into_owned()
}
fn decode_utf32(bytes: &[u8], little: bool) -> String {
    let (chunks, remainder) = bytes.as_chunks::<4>();
    let mut result = chunks
        .iter()
        .map(|b| {
            let b = [b[0], b[1], b[2], b[3]];
            char::from_u32(if little {
                u32::from_le_bytes(b)
            } else {
                u32::from_be_bytes(b)
            })
            .unwrap_or('\u{fffd}')
        })
        .collect::<String>();
    if !remainder.is_empty() {
        result.push('\u{fffd}');
    }
    result
}

#[cfg(test)]
mod native_folder_tests {
    use super::*;
    #[test]
    fn unavailable_directory_returns_errors_through_existing_storage_boundary() {
        let platform = FileSampleStoragePlatform {
            folder: Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no native data directory",
            )),
        };
        assert!(platform.get_setting(SETTINGS_KEY).is_err());
        assert!(platform.set_setting(SETTINGS_KEY, "{}").is_err());
        assert!(platform.local_folder().is_err());
    }
}
