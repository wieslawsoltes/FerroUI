//! Browser-native synchronous persistence. No filesystem or .NET storage emulation.
use ferroboard_sample::sample_storage::{SampleStoragePlatform, StorageResult};
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct BrowserStorage;
fn error(value: wasm_bindgen::JsValue) -> Box<dyn std::error::Error> {
    io::Error::other(format!("browser storage failed: {value:?}")).into()
}
impl BrowserStorage {
    fn store() -> StorageResult<web_sys::Storage> {
        web_sys::window()
            .ok_or_else(|| io::Error::other("browser window is unavailable"))?
            .local_storage()
            .map_err(error)?
            .ok_or_else(|| io::Error::other("browser local storage is unavailable").into())
    }
    fn read(&self, key: &str) -> StorageResult<Option<String>> {
        Self::store()?
            .get_item(&format!("ferroboard.sample.{key}"))
            .map_err(error)
    }
    fn write(&self, key: &str, value: &str) -> StorageResult<()> {
        Self::store()?
            .set_item(&format!("ferroboard.sample.{key}"), value)
            .map_err(error)
    }
}
impl SampleStoragePlatform for BrowserStorage {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        self.read(key)
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.write(key, value)
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "browser storage has no native filesystem folder",
        )
        .into())
    }
    fn read_document(&self, name: &Path) -> StorageResult<Option<String>> {
        let name = name.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "document key is not valid Unicode",
            )
        })?;
        self.read(name)
    }
    fn write_document(&self, name: &Path, content: &str) -> StorageResult<()> {
        let name = name.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "document key is not valid Unicode",
            )
        })?;
        self.write(name, content)
    }
    fn log(&self, message: &str) {
        web_sys::console::error_1(&message.into());
    }
}
