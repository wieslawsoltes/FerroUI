//! Storage imports: file pickers, storage items and bookmarks.
//!
//! Except for `Caniuse.hasNativeFilePicker`, the functions live in the
//! storage bundle (`storage.js`), which is imported on first use by
//! [`ferro_module::import_storage`](super::ferro_module::import_storage);
//! they are reached through `StorageModule.module` of the main module and
//! may only be called once that import has completed. The asynchronous
//! ones return their promise, awaited with a [`JsTask`].

use super::promise_helper::JsTask;
use super::{non_null, JsObject};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = Caniuse, js_name = hasNativeFilePicker)]
    pub fn has_native_file_picker() -> bool;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageProvider"], js_name = selectFolderDialog)]
    fn select_folder_dialog_raw(start_in: &JsObject, prefer_polyfill: bool) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageProvider"], js_name = openFileDialog)]
    fn open_file_dialog_raw(
        start_in: &JsObject,
        multiple: bool,
        types: Option<Vec<JsObject>>,
        exclude_accept_all_option: bool,
        prefer_polyfill: bool,
    ) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageProvider"], js_name = saveFileDialog)]
    fn save_file_dialog_raw(
        start_in: &JsObject,
        suggested_name: Option<String>,
        types: Option<Vec<JsObject>>,
        exclude_accept_all_option: bool,
        prefer_polyfill: bool,
    ) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = createWellKnownDirectory)]
    pub fn create_well_known_directory(well_known_directory: &str) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageProvider"], js_name = openBookmark)]
    fn open_bookmark_raw(key: &str) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = saveBookmark)]
    fn save_bookmark_raw(item: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = deleteBookmark)]
    fn delete_bookmark_raw(item: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = getProperties)]
    fn get_properties_raw(item: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = openWrite)]
    fn open_write_raw(item: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = openRead)]
    fn open_read_raw(item: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = createFromHandle)]
    fn storage_item_from_handle_raw(handle: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = getItemsIterator)]
    fn get_items_iterator_raw(item: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItems"], js_name = itemsArray)]
    pub fn items_array(item: &JsObject) -> Vec<JsObject>;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItems"], js_name = filesToItemsArray)]
    pub fn files_to_items_array(item: &JsObject) -> Vec<JsObject>;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageProvider"], js_name = createAcceptType)]
    pub fn create_accept_type(description: &str, mime_types: Vec<String>, extensions: Option<Vec<String>>) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = deleteAsync)]
    fn delete_async_raw(file_handle: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = moveAsync)]
    fn move_async_raw(file_handle: &JsObject, destination_folder: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = createFile)]
    fn create_file_raw(folder_handle: &JsObject, name: &str) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = isSameEntry)]
    fn is_same_entry_raw(item: &JsObject, other: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = createFolder)]
    fn create_folder_raw(folder_handle: &JsObject, name: &str) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = getFile)]
    fn get_file_raw(folder_handle: &JsObject, name: &str) -> JsObject;

    #[wasm_bindgen(js_namespace = ["StorageModule", "module", "StorageItem"], js_name = getFolder)]
    fn get_folder_raw(folder_handle: &JsObject, name: &str) -> JsObject;

    /// A storage item of the storage bundle (`StorageItem` of
    /// `storage/storageItem.ts`), or a file system handle.
    pub type StorageItemObject;

    #[wasm_bindgen(method, getter)]
    pub fn name(this: &StorageItemObject) -> Option<String>;

    #[wasm_bindgen(method, getter)]
    pub fn kind(this: &StorageItemObject) -> Option<String>;

    /// What `StorageItem.getProperties` resolves to.
    pub type StorageItemProperties;

    #[wasm_bindgen(method, getter = Size)]
    pub fn size(this: &StorageItemProperties) -> Option<f64>;

    #[wasm_bindgen(method, getter = LastModified)]
    pub fn last_modified(this: &StorageItemProperties) -> Option<f64>;

    /// The asynchronous iterator over the entries of a directory handle.
    pub type ItemsIterator;

    #[wasm_bindgen(method, js_name = next)]
    fn next_raw(this: &ItemsIterator) -> JsObject;

    /// The result of one step of an [`ItemsIterator`].
    pub type ItemsIteratorResult;

    #[wasm_bindgen(method, getter)]
    pub fn done(this: &ItemsIteratorResult) -> Option<bool>;

    #[wasm_bindgen(method, getter)]
    fn value(this: &ItemsIteratorResult) -> JsObject;

    /// An entry of a directory: the array of its name and its handle.
    type ItemsIteratorEntry;

    #[wasm_bindgen(method, indexing_getter)]
    fn get(this: &ItemsIteratorEntry, index: u32) -> JsObject;
}

/// The script side throws or rejects when there is no `start_in`; a missing
/// value is passed as `undefined`.
fn optional(value: Option<&JsObject>) -> JsObject {
    value.cloned().unwrap_or(JsObject::UNDEFINED)
}

pub fn select_folder_dialog(start_in: Option<&JsObject>, prefer_polyfill: bool) -> JsTask {
    JsTask::new(select_folder_dialog_raw(&optional(start_in), prefer_polyfill))
}

pub fn open_file_dialog(
    start_in: Option<&JsObject>,
    multiple: bool,
    types: Option<Vec<JsObject>>,
    exclude_accept_all_option: bool,
    prefer_polyfill: bool,
) -> JsTask {
    JsTask::new(open_file_dialog_raw(&optional(start_in), multiple, types, exclude_accept_all_option, prefer_polyfill))
}

pub fn save_file_dialog(
    start_in: Option<&JsObject>,
    suggested_name: Option<String>,
    types: Option<Vec<JsObject>>,
    exclude_accept_all_option: bool,
    prefer_polyfill: bool,
) -> JsTask {
    JsTask::new(save_file_dialog_raw(
        &optional(start_in),
        suggested_name,
        types,
        exclude_accept_all_option,
        prefer_polyfill,
    ))
}

pub fn open_bookmark(key: &str) -> JsTask {
    JsTask::new(open_bookmark_raw(key))
}

pub fn save_bookmark(item: &JsObject) -> JsTask {
    JsTask::new(save_bookmark_raw(item))
}

pub fn delete_bookmark(item: &JsObject) -> JsTask {
    JsTask::new(delete_bookmark_raw(item))
}

pub fn get_properties(item: &JsObject) -> JsTask {
    JsTask::new(get_properties_raw(item))
}

pub fn open_write(item: &JsObject) -> JsTask {
    JsTask::new(open_write_raw(item))
}

pub fn open_read(item: &JsObject) -> JsTask {
    JsTask::new(open_read_raw(item))
}

pub fn storage_item_from_handle(handle: &JsObject) -> Option<JsObject> {
    non_null(storage_item_from_handle_raw(handle))
}

pub fn get_items_iterator(item: &JsObject) -> Option<ItemsIterator> {
    non_null(get_items_iterator_raw(item)).map(JsCast::unchecked_into)
}

pub fn delete_async(file_handle: &JsObject) -> JsTask {
    JsTask::new(delete_async_raw(file_handle))
}

pub fn move_async(file_handle: &JsObject, destination_folder: &JsObject) -> JsTask {
    JsTask::new(move_async_raw(file_handle, destination_folder))
}

pub fn create_file(folder_handle: &JsObject, name: &str) -> JsTask {
    JsTask::new(create_file_raw(folder_handle, name))
}

/// Whether both storage items are the same entry of the file system;
/// resolves to a boolean.
pub fn is_same_entry(item: &JsObject, other: &JsObject) -> JsTask {
    JsTask::new(is_same_entry_raw(item, other))
}

pub fn create_folder(folder_handle: &JsObject, name: &str) -> JsTask {
    JsTask::new(create_folder_raw(folder_handle, name))
}

pub fn get_file(folder_handle: &JsObject, name: &str) -> JsTask {
    JsTask::new(get_file_raw(folder_handle, name))
}

pub fn get_folder(folder_handle: &JsObject, name: &str) -> JsTask {
    JsTask::new(get_folder_raw(folder_handle, name))
}

/// The `kind` of a storage item or handle: `"file"` or `"directory"`.
pub fn kind_of(item: &JsObject) -> Option<String> {
    item.unchecked_ref::<StorageItemObject>().kind()
}

/// The `name` of a storage item or handle.
pub fn name_of(item: &JsObject) -> Option<String> {
    item.unchecked_ref::<StorageItemObject>().name()
}

impl ItemsIterator {
    /// The next entry of the directory.
    pub fn next(&self) -> JsTask {
        JsTask::new(self.next_raw())
    }
}

impl ItemsIteratorResult {
    /// The handle of the entry (element 1 of the `[name, handle]` pair).
    pub fn item(&self) -> Option<JsObject> {
        let value = non_null(self.value())?;
        non_null(value.unchecked_ref::<ItemsIteratorEntry>().get(1))
    }
}
