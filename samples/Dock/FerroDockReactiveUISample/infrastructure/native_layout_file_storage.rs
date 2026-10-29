//! Native picker and storage adapter; presentation code sees only file bytes/names.
use crate::services::{ILayoutFileStorage, LayoutFileContents, LayoutFileFuture};
use ferrodock_model::core::{DockSerializationError, SerializationResult};
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageFile,
        IStorageProvider,
    },
    Ref, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use std::{io, rc::Rc};

pub(crate) struct NativeLayoutFileStorage {
    parent: WeakRef<Control>,
}
impl NativeLayoutFileStorage {
    pub(crate) fn new(parent: Ref<Control>) -> Rc<Self> {
        Rc::new(Self {
            parent: parent.downgrade(),
        })
    }
    fn provider(&self) -> SerializationResult<Rc<dyn IStorageProvider>> {
        let parent = self.parent.upgrade().ok_or_else(|| {
            DockSerializationError::new(io::Error::other("The layout window has closed."))
        })?;
        let top = TopLevel::get_top_level(Some(&parent)).ok_or_else(|| {
            DockSerializationError::new(io::Error::other(
                "No window is available for file selection.",
            ))
        })?;
        Ok(top.storage_provider())
    }
}
fn json_file_type() -> Rc<FilePickerFileType> {
    FilePickerFileType::new(Some("JSON layout"))
        .with_patterns(&["*.json"])
        .with_mime_types(&["application/json"])
        .with_apple_uniform_type_identifiers(&["public.json"])
}
fn open_options() -> FilePickerOpenOptions {
    let mut options = FilePickerOpenOptions::new().with_file_type_filter(vec![json_file_type()]);
    options.set_title(Some("Open docking layout".into()));
    options
}
fn save_options() -> FilePickerSaveOptions {
    let mut options = FilePickerSaveOptions::new()
        .with_default_extension("json")
        .with_file_type_choices(vec![json_file_type()])
        .with_show_overwrite_prompt(true);
    options.set_title(Some("Save docking layout".into()));
    options.set_suggested_file_name(Some("layout.json".into()));
    options
}
async fn read_selected_file(file: Rc<dyn IStorageFile>) -> SerializationResult<LayoutFileContents> {
    let result: io::Result<LayoutFileContents> = async {
        let mut stream = file.open_read_async().await?;
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        Ok(LayoutFileContents {
            name: file.name(),
            bytes,
        })
    }
    .await;
    file.dispose();
    result.map_err(DockSerializationError::new)
}
async fn write_selected_file(
    file: Rc<dyn IStorageFile>,
    bytes: Vec<u8>,
) -> SerializationResult<String> {
    let result: io::Result<String> = async {
        let mut stream = file.open_write_async().await?;
        stream.write_all(&bytes)?;
        stream.flush()?;
        Ok(file.name())
    }
    .await;
    file.dispose();
    result.map_err(DockSerializationError::new)
}
impl ILayoutFileStorage for NativeLayoutFileStorage {
    fn open_layout(&self) -> LayoutFileFuture<Option<LayoutFileContents>> {
        let provider = self.provider();
        Box::pin(async move {
            let provider = provider?;
            if !provider.can_open() {
                return Err(DockSerializationError::new(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Opening files is unavailable on this platform.",
                )));
            }
            let files = provider
                .open_file_picker_async(open_options())
                .await
                .map_err(DockSerializationError::new)?;
            let mut files = files.into_iter();
            let selected = files.next();
            for extra in files {
                extra.dispose();
            }
            match selected {
                Some(file) => read_selected_file(file).await.map(Some),
                None => Ok(None),
            }
        })
    }
    fn save_layout(&self, bytes: Vec<u8>) -> LayoutFileFuture<Option<String>> {
        let provider = self.provider();
        Box::pin(async move {
            let provider = provider?;
            if !provider.can_save() {
                return Err(DockSerializationError::new(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Saving files is unavailable on this platform.",
                )));
            }
            match provider
                .save_file_picker_async(save_options())
                .await
                .map_err(DockSerializationError::new)?
            {
                Some(file) => write_selected_file(file, bytes).await.map(Some),
                None => Ok(None),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::platform::storage::file_io::{BclStorageFile, FileSystemInfo};
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };
    fn ready<T>(future: impl Future<Output = T>) -> T {
        let mut future = pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("native local file stream unexpectedly pending"),
        }
    }
    #[test]
    fn layout_picker_options_use_json_filters_and_confirm_overwriting() {
        let open = open_options();
        assert!(!open.allow_multiple());
        assert_eq!(open.title(), Some("Open docking layout"));
        let types = open.file_type_filter().unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].patterns().unwrap().as_ref(), ["*.json"]);
        assert_eq!(
            types[0].mime_types().unwrap().as_ref(),
            ["application/json"]
        );
        assert_eq!(
            types[0].apple_uniform_type_identifiers().unwrap().as_ref(),
            ["public.json"]
        );
        let save = save_options();
        assert_eq!(save.default_extension(), Some("json"));
        assert_eq!(save.suggested_file_name(), Some("layout.json"));
        assert_eq!(save.show_overwrite_prompt(), Some(true));
        assert_eq!(save.file_type_choices().unwrap().len(), 1);
    }
    #[test]
    fn native_storage_streams_roundtrip_and_truncate_with_original_io_errors() {
        let path = std::path::PathBuf::from("/Volumes/SSD/temp/ferrodock-port")
            .join(format!("layout-file-storage-{}.json", std::process::id()));
        std::fs::write(&path, b"longer previous layout data").unwrap();
        let file = || BclStorageFile::new(FileSystemInfo::file(&path)) as Rc<dyn IStorageFile>;
        let bytes = vec![0xef, 0xbb, 0xbf, b'n', b'u', b'l', b'l'];
        let name = ready(write_selected_file(file(), bytes.clone())).unwrap();
        let loaded = ready(read_selected_file(file())).unwrap();
        assert_eq!(loaded.name, name);
        assert_eq!(loaded.bytes, bytes);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(&path).unwrap();
        let error = ready(read_selected_file(file())).err().unwrap();
        assert_eq!(
            error.cause().downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::NotFound
        );
        let absent = path.join("missing.json");
        let file = BclStorageFile::new(FileSystemInfo::file(absent));
        assert!(ready(write_selected_file(file, vec![1])).is_err());
    }
}
