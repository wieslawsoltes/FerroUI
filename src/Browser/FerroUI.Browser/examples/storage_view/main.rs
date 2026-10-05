//! The storage provider of the browser in a web page: the end-to-end check
//! of file pickers, storage items, streams and bookmarks.
//!
//! The view is a text block. The behaviour tests (`scripts/browser/tests`)
//! run scenarios through the `storageViewRun` export, which uses the storage
//! provider of the top-level as an application would, and read their outcome
//! with `storageViewResult`. The pickers are those of the page: the tests
//! replace them with functions that return handles of the origin private
//! file system, or let the `native-file-system-adapter` polyfill show its
//! own (`?PreferPolyfill=true`).
//!
//! Build and assemble the site with `scripts/build-browser.sh storage_view`.

#![cfg_attr(target_os = "emscripten", no_main)]

use ferroui_base::input::{DataTransferExtensions, DragDrop, DragDropEffects};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontManagerOptions};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerFileTypes, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions,
    IStorageFile, IStorageFolder, IStorageItem, IStorageProvider, WellKnownFolder,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Thickness};
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication, TextBlock,
    TopLevel,
};
use ferroui_fonts_inter::AppBuilderExtension;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::io::{Read, Write};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

thread_local! {
    static MAIN_VIEW: RefCell<Option<Ref<Control>>> = const { RefCell::new(None) };
    static RESULTS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self { base: Application::construct() })
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_name(Some("FerroUI storage view".to_string()));
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(single_view) = lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            let text = TextBlock::new();
            text.set_text(Some("Storage"));
            text.set_margin(Thickness::uniform(16.0));
            // The whole view is a drop target for files.
            let view = Border::new();
            view.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(0xff, 0xff, 0xff)))));
            view.set_child(Some(text.upcast()));
            DragDrop::set_allow_drop(&view, true);
            DragDrop::add_drag_over_handler(&view, |_, e| {
                e.set_drag_effects(e.drag_effects() & DragDropEffects::COPY);
            });
            DragDrop::add_drop_handler(&view, |_, e| {
                let files = e.data_transfer().try_get_files().unwrap_or_default();
                start("drop".to_string(), read_dropped(files));
                e.set_drag_effects(e.drag_effects() & DragDropEffects::COPY);
            });
            let view: Ref<Control> = view.upcast();
            MAIN_VIEW.with(|main_view| *main_view.borrow_mut() = Some(view.clone()));
            single_view.set_main_view(Some(view));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

/// The storage provider of the top-level of the view.
fn storage_provider() -> Result<Rc<dyn IStorageProvider>, String> {
    let view = MAIN_VIEW.with(|view| view.borrow().clone()).ok_or("no view")?;
    let top_level = TopLevel::get_top_level(Some(&view)).ok_or("the view has no top-level")?;
    Ok(top_level.storage_provider())
}

fn text_types() -> Vec<Rc<FilePickerFileType>> {
    vec![
        FilePickerFileTypes::text_plain(),
        FilePickerFileType::new(Some("Data")).with_mime_types(&["application/octet-stream"]).with_patterns(&["*.bin"]),
    ]
}

async fn read_text(file: &Rc<dyn IStorageFile>) -> Result<String, String> {
    let mut stream = file.open_read_async().await.map_err(|e| e.to_string())?;
    let mut text = String::new();
    stream.read_to_string(&mut text).map_err(|e| e.to_string())?;
    Ok(text)
}

async fn write_bytes(file: &Rc<dyn IStorageFile>, chunks: &[&[u8]]) -> Result<(), String> {
    let mut stream = file.open_write_async().await.map_err(|e| e.to_string())?;
    for chunk in chunks {
        stream.write_all(chunk).map_err(|e| e.to_string())?;
    }
    stream.flush().map_err(|e| e.to_string())
}

async fn pick_file(provider: &Rc<dyn IStorageProvider>, multiple: bool) -> Result<Vec<Rc<dyn IStorageFile>>, String> {
    let mut options = FilePickerOpenOptions::new();
    options.set_allow_multiple(multiple);
    options.set_file_type_filter(Some(text_types()));
    provider.open_file_picker_async(options).await.map_err(|e| e.to_string())
}

/// The names and contents of dropped files.
async fn read_dropped(files: Vec<Rc<dyn IStorageItem>>) -> Result<String, String> {
    let mut contents = Vec::new();
    for item in &files {
        let file = item.clone().as_storage_file().ok_or("a dropped item is not a file")?;
        contents.push(read_text(&file).await?);
    }
    Ok(format!("count={};names={};content={}", files.len(), names(&files), contents.join("|")))
}

fn names(items: &[Rc<dyn IStorageItem>]) -> String {
    let mut names: Vec<String> = items.iter().map(|item| item.name()).collect();
    names.sort();
    names.join(",")
}

/// A large pattern that does not repeat with any power-of-two period.
fn pattern(length: usize) -> Vec<u8> {
    (0..length).map(|i| ((i * 7 + i / 251) % 256) as u8).collect()
}

async fn scenario(name: String, argument: String) -> Result<String, String> {
    let provider = storage_provider()?;
    match name.as_str() {
        "capabilities" => Ok(format!(
            "can_open={};can_save={};can_pick_folder={}",
            provider.can_open(),
            provider.can_save(),
            provider.can_pick_folder()
        )),
        "open_read" => {
            let files = pick_file(&provider, argument == "multiple").await?;
            let mut contents = Vec::new();
            let mut sizes = Vec::new();
            for file in &files {
                contents.push(read_text(file).await?);
                let properties = file.get_basic_properties_async().await;
                sizes.push(format!("{:?}/{}", properties.size(), properties.date_modified().is_some()));
            }
            let items: Vec<Rc<dyn IStorageItem>> = files.iter().map(|file| file.clone() as Rc<dyn IStorageItem>).collect();
            Ok(format!(
                "count={};names={};paths={};sizes={};content={}",
                files.len(),
                names(&items),
                files.iter().map(|file| file.path().to_string()).collect::<Vec<_>>().join(","),
                sizes.join(","),
                contents.join("|")
            ))
        }
        "save_write" => {
            let mut options = FilePickerSaveOptions::new();
            options.set_suggested_file_name(Some(argument));
            options.set_default_extension(Some("txt".to_string()));
            options.set_file_type_choices(Some(text_types()));
            let Some(file) = provider.save_file_picker_async(options).await.map_err(|e| e.to_string())? else {
                return Ok("saved=none".to_string());
            };
            write_bytes(&file, &[b"Hello, ", b"storage"]).await?;
            Ok(format!("saved={}", file.name()))
        }
        "large" => {
            // Written in chunks while the module memory grows: every write copies out of the
            // memory as it is at that moment.
            let length: usize = argument.parse().map_err(|_| "the argument is the length")?;
            let Some(file) =
                provider.save_file_picker_async(FilePickerSaveOptions::new()).await.map_err(|e| e.to_string())?
            else {
                return Ok("saved=none".to_string());
            };
            let data = pattern(length);
            let mut stream = file.open_write_async().await.map_err(|e| e.to_string())?;
            let mut ballast = Vec::new();
            for chunk in data.chunks(256 * 1024) {
                stream.write_all(chunk).map_err(|e| e.to_string())?;
                ballast.push(vec![1u8; 4 * 1024 * 1024]);
            }
            drop(ballast);
            drop(stream);
            Ok(format!("written={length}"))
        }
        "read_large" => {
            let files = pick_file(&provider, false).await?;
            let file = files.first().ok_or("no file")?;
            let mut stream = file.open_read_async().await.map_err(|e| e.to_string())?;
            let mut content = Vec::new();
            stream.read_to_end(&mut content).map_err(|e| e.to_string())?;
            let length: usize = argument.parse().map_err(|_| "the argument is the length")?;
            Ok(format!("length={};equal={}", content.len(), content == pattern(length)))
        }
        "write_then_read" => {
            // Writes the first picked file through the synchronous stream, drops it and reads the
            // first (`same`) or the second (`other`) picked file.
            let files = pick_file(&provider, true).await?;
            let written = files.first().ok_or("no file")?;
            write_bytes(written, &[b"written"]).await?;
            let read = if argument == "same" { written } else { files.get(1).ok_or("no second file")? };
            Ok(format!("content={}", read_text(read).await?))
        }
                "folder" => {
            let folders =
                provider.open_folder_picker_async(FolderPickerOpenOptions::new()).await.map_err(|e| e.to_string())?;
            let folder = folders.first().ok_or("no folder")?;
            let sub = folder.create_folder_async("sub").await.map_err(|e| e.to_string())?.ok_or("no sub folder")?;
            let file = folder.create_file_async("a.txt").await.map_err(|e| e.to_string())?.ok_or("no file")?;
            write_bytes(&file, &[b"A"]).await?;
            let nested = sub.create_file_async("b.txt").await.map_err(|e| e.to_string())?.ok_or("no nested file")?;
            write_bytes(&nested, &[b"B"]).await?;
            let items = folder.get_items_async().await.map_err(|e| e.to_string())?;
            let kinds: Vec<String> = {
                let mut kinds: Vec<String> = items
                    .iter()
                    .map(|item| {
                        let kind = if item.clone().as_storage_folder().is_some() { "folder" } else { "file" };
                        format!("{}:{}", item.name(), kind)
                    })
                    .collect();
                kinds.sort();
                kinds
            };
            let a = folder.get_file_async("a.txt").await.ok_or("a.txt not found")?;
            // Let the close of the writes above settle before reading back.
            let a_text = read_text(&a).await?;
            let missing = folder.get_file_async("missing.txt").await.is_none();
            let mismatch = folder.get_folder_async("a.txt").await.is_none();
            let sub_again = folder.get_folder_async("sub").await.ok_or("sub not found")?;
            let sub_items = sub_again.get_items_async().await.map_err(|e| e.to_string())?;
            a.delete_async().await.map_err(|e| e.to_string())?;
            let after_delete = folder.get_items_async().await.map_err(|e| e.to_string())?;
            Ok(format!(
                "folder={};items={};a={};missing={};mismatch={};sub={};after_delete={}",
                folder.name(),
                kinds.join(","),
                a_text,
                missing,
                mismatch,
                names(&sub_items),
                names(&after_delete)
            ))
        }
        "move" => {
            let folders =
                provider.open_folder_picker_async(FolderPickerOpenOptions::new()).await.map_err(|e| e.to_string())?;
            let folder = folders.first().ok_or("no folder")?;
            let file = folder.create_file_async("moved.txt").await.map_err(|e| e.to_string())?.ok_or("no file")?;
            write_bytes(&file, &[b"M"]).await?;
            let target = folder.create_folder_async("target").await.map_err(|e| e.to_string())?.ok_or("no target")?;
            let moved = file.move_async(target.clone()).await.map_err(|e| e.to_string())?;
            let moved = moved.ok_or("move returned no item")?;
            let in_target = target.get_items_async().await.map_err(|e| e.to_string())?;
            let left = folder.get_file_async("moved.txt").await.is_none();
            Ok(format!("moved={};target={};left={}", moved.name(), names(&in_target), left))
        }
        "bookmark_save" => {
            let files = pick_file(&provider, false).await?;
            let file = files.first().ok_or("no file")?;
            let bookmark = if file.can_bookmark() { file.save_bookmark_async().await } else { None };
            Ok(format!("can_bookmark={};bookmark={}", file.can_bookmark(), bookmark.unwrap_or_default()))
        }
        "bookmark_open" => {
            let as_folder = provider.open_folder_bookmark_async(&argument).await.is_some();
            let Some(file) = provider.open_file_bookmark_async(&argument).await else {
                return Ok(format!("file=none;as_folder={as_folder}"));
            };
            let file: Rc<dyn IStorageFile> = file;
            Ok(format!("file={};content={};as_folder={as_folder}", file.name(), read_text(&file).await?))
        }
        "bookmark_release" => {
            let file = provider.open_file_bookmark_async(&argument).await.ok_or("bookmark not found")?;
            file.release_bookmark_async().await;
            Ok(format!("reopened={}", provider.open_file_bookmark_async(&argument).await.is_some()))
        }
        "well_known" => {
            let folder: Rc<dyn IStorageFolder> = provider
                .try_get_well_known_folder_async(WellKnownFolder::Documents)
                .await
                .ok_or("no well-known folder")?;
            let by_path = provider.try_get_file_from_path_async(&folder.path()).await.is_none();
            Ok(format!("name={};path_lookup_none={}", folder.name(), by_path))
        }
        "pick_folder_start_in_documents" => {
            let documents = provider
                .try_get_well_known_folder_async(WellKnownFolder::Documents)
                .await
                .ok_or("no well-known folder")?;
            let mut options = FolderPickerOpenOptions::new();
            options.set_suggested_start_location(Some(documents));
            let folders = provider.open_folder_picker_async(options).await.map_err(|e| e.to_string())?;
            Ok(format!("count={}", folders.len()))
        }
        _ => Err(format!("unknown scenario {name}")),
    }
}

fn start(name: String, future: impl Future<Output = Result<String, String>> + 'static) {
    RESULTS.with(|results| results.borrow_mut().remove(&name));
    Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
        let result = match future.await {
            Ok(result) => result,
            Err(error) => format!("error={error}"),
        };
        RESULTS.with(|results| results.borrow_mut().insert(name, result));
    });
}

/// Starts the scenario `name` with `argument`; its outcome is read with
/// [`storage_view_result`].
#[wasm_bindgen(js_name = storageViewRun)]
pub fn storage_view_run(name: &str, argument: &str) {
    start(name.to_string(), scenario(name.to_string(), argument.to_string()));
}

/// The outcome of the last run of scenario `name`: a line of `name=value`
/// pairs, `error=...` when it failed, empty while it runs.
#[wasm_bindgen(js_name = storageViewResult)]
pub fn storage_view_result(name: &str) -> String {
    RESULTS.with(|results| results.borrow().get(name).cloned().unwrap_or_default())
}

/// The value of `name` in a query string (`?a=1&b=2`), ignoring the case of
/// the name.
fn query_value(query: &str, name: &str) -> Option<String> {
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.to_string())
}

/// The entry point the host page calls once the module is created and
/// registered with the script side. `query` is the query string of the page.
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str) {
    let options = BrowserPlatformOptions {
        prefer_file_dialog_polyfill: query_value(query, "PreferPolyfill").is_some_and(|value| value == "true"),
        ..Default::default()
    };

    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".to_string()),
            ..Default::default()
        }))
        .start_browser_app("out", Some(options));
}

/// The example only does something in a web page; elsewhere it builds so
/// that the workspace checks cover it.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("storage_view runs in a browser: build it with scripts/build-browser.sh storage_view");
}
