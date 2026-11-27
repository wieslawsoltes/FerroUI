//! The code behind of the StoragePickers page (the handlers of `StoragePickersPage.xaml.cs`) over
//! the storage provider of the window (`TopLevel::storage_provider`).
//!
//! The source's `FileOpenPicker`, `FileSavePicker` and `FolderPicker` are the provider's file,
//! save and folder pickers. A picker the provider does not have (`can_open`, `can_save`,
//! `can_pick_folder` false: a window without a platform storage provider, or a portal without
//! folder selection) is reported in the example's output instead of opening. The pickers are the
//! platform's own dialogs; nothing of this page is drawn outside the window's renderer.

use ferroui_base::media::imaging::{Bitmap, BitmapInterpolationMode};
use ferroui_base::media::IImage;
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerFileTypes, FilePickerOpenOptions, FilePickerSaveOptions,
    FolderPickerOpenOptions, IStorageFile, IStorageFolder, IStorageItem, IStorageProvider,
    WellKnownFolder,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Ref, WeakRef};
use ferroui_controls::{
    Button, CheckBox, ComboBox, Control, Image, ItemsSource, NumberBox, TextBlock, TextBox,
    TopLevel,
};
use std::future::Future;
use std::io::{Read, Write};
use std::rc::Rc;

/// The members of `PickerLocationId`, the source's `pickerLocationIds`, with the well-known folder
/// of the storage provider that each one is; the others leave the start location to the platform.
pub(crate) const PICKER_LOCATIONS: &[(&str, Option<WellKnownFolder>)] = &[
    ("DocumentsLibrary", Some(WellKnownFolder::Documents)),
    ("ComputerFolder", None),
    ("Desktop", Some(WellKnownFolder::Desktop)),
    ("Downloads", Some(WellKnownFolder::Downloads)),
    ("MusicLibrary", Some(WellKnownFolder::Music)),
    ("PicturesLibrary", Some(WellKnownFolder::Pictures)),
    ("VideosLibrary", Some(WellKnownFolder::Videos)),
    ("Objects", None),
    ("Unspecified", None),
];

/// The members of `PickerViewMode`, the source's `pickerViewModes`. FerroUI's pickers have no view
/// mode: the choice feeds the source viewer only.
pub(crate) const PICKER_VIEW_MODES: &[&str] = &["List", "Thumbnail"];

/// The members of `ThumbnailMode`, the source's `thumbnailModes`.
pub(crate) const THUMBNAIL_MODES: &[&str] = &[
    "PicturesView",
    "VideosView",
    "MusicView",
    "DocumentsView",
    "ListView",
    "SingleItem",
];

/// The message of a page whose window has no storage service.
pub(crate) const NO_STORAGE: &str = "This window has no storage service.";
/// The message of a platform without a file picker.
pub(crate) const NO_FILE_PICKER: &str = "This platform has no file picker.";
/// The message of a platform without a save picker.
pub(crate) const NO_SAVE_PICKER: &str = "This platform has no file save picker.";
/// The message of a platform without a folder picker.
pub(crate) const NO_FOLDER_PICKER: &str = "This platform has no folder picker.";

/// The size the thumbnail example asks for when its box is empty or not positive (the source's
/// fallback for `NumberBox.Value` NaN).
pub(crate) const DEFAULT_THUMBNAIL_SIZE: u32 = 200;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id != "StoragePickers" {
        return;
    }
    let locations =
        || ItemsSource::from_values(PICKER_LOCATIONS.iter().map(|(name, _)| (*name).to_string()));
    let view_modes =
        || ItemsSource::from_values(PICKER_VIEW_MODES.iter().map(|name| (*name).to_string()));
    for index in 1..=4 {
        fill(root, &format!("PickerLocationComboBox{index}"), locations());
    }
    for index in 1..=3 {
        fill(
            root,
            &format!("PickerViewModeComboBox{index}"),
            view_modes(),
        );
    }
    fill(
        root,
        "ThumbnailModeComboBox",
        ItemsSource::from_values(THUMBNAIL_MODES.iter().map(|name| (*name).to_string())),
    );

    on_pick(
        root,
        "PickSingleFileButton",
        "PickedSingleFileTextBlock",
        Picker::Open,
        |root, provider| pick_files(root, provider, false),
    );
    on_pick(
        root,
        "PickMultipleFilesButton",
        "PickedMultipleFilesTextBlock",
        Picker::Open,
        |root, provider| pick_files(root, provider, true),
    );
    on_pick(
        root,
        "SaveFileButton",
        "SavedFileTextBlock",
        Picker::Save,
        save_file,
    );
    on_pick(
        root,
        "PickFolderButton",
        "PickedFolderTextBlock",
        Picker::Folder,
        pick_folder,
    );
    on_pick(
        root,
        "PickFileForThumbnailButton",
        "ThumbnailDetailsTextBlock",
        Picker::Open,
        pick_thumbnail,
    );
    on_pick(
        root,
        "SelectSuggestedFolderButton",
        "SuggestedFolderStatus",
        Picker::Folder,
        select_suggested_folder,
    );
}

/// The source's `ItemsSource="{x:Bind ...}"` of the page's own list with `SelectedIndex="0"`.
fn fill(root: &Ref<Control>, name: &str, items: ItemsSource) {
    let combo = root.get_control::<ComboBox>(name);
    combo.set_items_source(Some(items));
    combo.set_selected_index(0);
}

#[derive(Clone, Copy)]
enum Picker {
    Open,
    Save,
    Folder,
}

impl Picker {
    fn is_supported(self, provider: &dyn IStorageProvider) -> bool {
        match self {
            Picker::Open => provider.can_open(),
            Picker::Save => provider.can_save(),
            Picker::Folder => provider.can_pick_folder(),
        }
    }

    fn missing(self) -> &'static str {
        match self {
            Picker::Open => NO_FILE_PICKER,
            Picker::Save => NO_SAVE_PICKER,
            Picker::Folder => NO_FOLDER_PICKER,
        }
    }
}

/// A click handler of the source: the button is disabled while its picker is open (the source's
/// guard against a double click), `operation` runs with the window's storage provider and its
/// message is the text of `output`, a polite live region (the source's announcement). Without a
/// window or a picker, `output` says so and nothing opens.
fn on_pick<F>(
    root: &Ref<Control>,
    button: &str,
    output: &str,
    picker: Picker,
    operation: impl Fn(Ref<Control>, Rc<dyn IStorageProvider>) -> F + 'static,
) where
    F: Future<Output = Option<String>> + 'static,
{
    let button = root.get_control::<Button>(button);
    let weak_root = root.downgrade();
    let weak_button = button.downgrade();
    let output = output.to_string();
    button.click(move |_, _| {
        let (Some(root), Some(button)) = (weak_root.upgrade(), weak_button.upgrade()) else {
            return;
        };
        let provider =
            match TopLevel::get_top_level(Some(&button)).map(|host| host.storage_provider()) {
                Some(provider) if picker.is_supported(&*provider) => provider,
                Some(_) => return set_text(&root.downgrade(), &output, picker.missing()),
                None => return set_text(&root.downgrade(), &output, NO_STORAGE),
            };
        button.set_is_enabled(false);
        let operation = operation(root.clone(), provider);
        let weak_root = root.downgrade();
        let weak_button = button.downgrade();
        let output = output.clone();
        drop(Dispatcher::ui_thread().start_local(async move {
            if let Some(message) = operation.await {
                set_text(&weak_root, &output, &message);
            }
            if let Some(button) = weak_button.upgrade() {
                button.set_is_enabled(true);
            }
        }));
    });
}

fn set_text(root: &WeakRef<Control>, name: &str, text: &str) {
    if let Some(root) = root.upgrade() {
        root.get_control::<TextBlock>(name).set_text(Some(text));
    }
}

fn text_of(root: &Ref<Control>, name: &str) -> String {
    root.get_control::<TextBox>(name).text().unwrap_or_default()
}

fn selected_index(root: &Ref<Control>, name: &str) -> usize {
    usize::try_from(root.get_control::<ComboBox>(name).selected_index()).unwrap_or(0)
}

/// The text of a picked item: its local path, the source's `Path`, else (a browser's file) its
/// name.
pub(crate) fn path_of(item: &dyn IStorageItem) -> String {
    item.try_get_local_path().unwrap_or_else(|| item.name())
}

/// `picker.CommitButtonText`: the title of FerroUI's picker, which has no commit button text (an
/// empty text is no title).
fn commit_text(root: &Ref<Control>, name: &str) -> Option<String> {
    Some(text_of(root, name)).filter(|text| !text.is_empty())
}

/// `picker.SuggestedStartLocation`: the well-known folder of the location chosen in `combo`.
async fn start_location(
    provider: &Rc<dyn IStorageProvider>,
    root: &Ref<Control>,
    combo: &str,
) -> Option<Rc<dyn IStorageFolder>> {
    let folder = PICKER_LOCATIONS
        .get(selected_index(root, combo))
        .and_then(|(_, folder)| *folder)?;
    provider.try_get_well_known_folder_async(folder).await
}

/// The file type of a `FileTypeComboBox` item's Tag: `.txt` and `images` are the source's
/// `FileTypeFilter` entries, `*` (all files) none for the single file picker and `*` for the
/// multiple one, as in the source.
pub(crate) fn file_type_filter(tag: &str, multiple: bool) -> Option<Vec<Rc<FilePickerFileType>>> {
    match tag {
        ".txt" => Some(vec![
            FilePickerFileType::new(Some("Text Files")).with_patterns(&["*.txt"])
        ]),
        "images" => Some(vec![
            FilePickerFileType::new(Some("Images")).with_patterns(&["*.jpg", "*.png"])
        ]),
        "*" if multiple => Some(vec![FilePickerFileTypes::all()]),
        _ => None,
    }
}

fn selected_tag(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<ComboBox>(name)
        .selected_item()
        .and_then(|item| Control::from_boxed(&item))
        .and_then(|item| item.tag())
        .and_then(|tag| tag.downcast_ref::<String>().cloned())
}

/// `PickSingleFileButton_Click` and `PickMultipleFilesButton_Click`.
async fn pick_files(
    root: Ref<Control>,
    provider: Rc<dyn IStorageProvider>,
    multiple: bool,
) -> Option<String> {
    let (types, commit, location) = if multiple {
        (
            "FileTypeComboBox2",
            "CommitButtonTextTextBox2",
            "PickerLocationComboBox2",
        )
    } else {
        (
            "FileTypeComboBox1",
            "CommitButtonTextTextBox",
            "PickerLocationComboBox1",
        )
    };
    let mut options = FilePickerOpenOptions::new().with_allow_multiple(multiple);
    if let Some(tag) = selected_tag(&root, types) {
        options.set_file_type_filter(file_type_filter(&tag, multiple));
    }
    options.set_title(commit_text(&root, commit));
    options.set_suggested_start_location(start_location(&provider, &root, location).await);
    drop(root);
    let files = match provider.open_file_picker_async(options).await {
        Ok(files) => files,
        Err(error) => return Some(format!("The file picker failed: {error}")),
    };
    Some(if multiple {
        picked_files_text(&files)
    } else {
        match files.first() {
            Some(file) => format!("Picked: {}", path_of(&**file)),
            None => String::from("No file selected."),
        }
    })
}

/// The output of `PickMultipleFilesButton_Click`: a line per file, each ending with a new line as
/// the source's `Environment.NewLine`.
pub(crate) fn picked_files_text(files: &[Rc<dyn IStorageFile>]) -> String {
    if files.is_empty() {
        return String::from("No files selected.");
    }
    files
        .iter()
        .map(|file| format!("- Picked: {}\n", path_of(&**file)))
        .collect()
}

/// The `FileTypeChoices` of the checked file types of the save example.
pub(crate) fn file_type_choices(
    txt: bool,
    json: bool,
    xml: bool,
) -> Option<Vec<Rc<FilePickerFileType>>> {
    let choices: Vec<_> = [
        (txt, "Text Files", "*.txt"),
        (json, "JSON Files", "*.json"),
        (xml, "XML Files", "*.xml"),
    ]
    .into_iter()
    .filter(|(checked, _, _)| *checked)
    .map(|(_, name, pattern)| FilePickerFileType::new(Some(name)).with_patterns(&[pattern]))
    .collect();
    (!choices.is_empty()).then_some(choices)
}

/// `SaveFileButton_Click`: the file content is written to the picked file. `SuggestedFolder`,
/// when one is selected, is the start location in place of `SuggestedStartLocation`; the default
/// extension is given without its dot, as FerroUI's save options take it.
async fn save_file(root: Ref<Control>, provider: Rc<dyn IStorageProvider>) -> Option<String> {
    let checked = |name: &str| root.get_control::<CheckBox>(name).is_checked() == Some(true);
    let mut options = FilePickerSaveOptions::new();
    options.set_file_type_choices(file_type_choices(
        checked("TxtCheckBox"),
        checked("JsonCheckBox"),
        checked("XmlCheckBox"),
    ));
    let extension = root
        .get_control::<ComboBox>("DefaultExtensionComboBox")
        .selected_item()
        .and_then(|item| item.downcast_ref::<String>().cloned())
        .unwrap_or_default();
    options.set_default_extension(
        Some(extension.trim_start_matches('.').to_string()).filter(|ext| !ext.is_empty()),
    );
    options.set_suggested_file_name(
        Some(text_of(&root, "SuggestedFileNameTextBox")).filter(|name| !name.is_empty()),
    );
    options.set_title(commit_text(&root, "CommitButtonTextTextBox3"));
    let suggested_folder = text_of(&root, "SuggestedFolderTextBox");
    let folder = match suggested_folder.is_empty() {
        false => {
            provider
                .try_get_folder_from_path_str_async(&suggested_folder)
                .await
        }
        true => None,
    };
    let folder = match folder {
        Some(folder) => Some(folder),
        None => start_location(&provider, &root, "PickerLocationComboBox3").await,
    };
    options.set_suggested_start_location(folder);
    let content = text_of(&root, "FileContentTextBox");
    drop(root);
    let file = match provider.save_file_picker_async(options).await {
        Ok(Some(file)) => file,
        Ok(None) => return Some(String::from("File save canceled.")),
        Err(error) => return Some(format!("The save picker failed: {error}")),
    };
    match write_all(&file, content.as_bytes()).await {
        Ok(()) => Some(format!("File saved to: {}", path_of(&*file))),
        Err(error) => Some(format!("The file couldn't be saved: {error}")),
    }
}

async fn write_all(file: &Rc<dyn IStorageFile>, bytes: &[u8]) -> std::io::Result<()> {
    let mut stream = file.open_write_async().await?;
    stream.write_all(bytes)?;
    stream.flush()
}

/// `PickFolderButton_Click`.
async fn pick_folder(root: Ref<Control>, provider: Rc<dyn IStorageProvider>) -> Option<String> {
    let mut options = FolderPickerOpenOptions::new();
    options.set_title(commit_text(&root, "CommitButtonTextTextBox4"));
    options.set_suggested_start_location(
        start_location(&provider, &root, "PickerLocationComboBox4").await,
    );
    drop(root);
    Some(match provider.open_folder_picker_async(options).await {
        Ok(folders) => match folders.first() {
            Some(folder) => format!("Picked: {}", path_of(&**folder)),
            None => String::from("No folder selected."),
        },
        Err(error) => format!("The folder picker failed: {error}"),
    })
}

/// `SelectSuggestedFolderButton_Click`: the picked folder is the save example's suggested folder.
async fn select_suggested_folder(
    root: Ref<Control>,
    provider: Rc<dyn IStorageProvider>,
) -> Option<String> {
    let mut options = FolderPickerOpenOptions::new();
    options.set_title(Some(String::from("Select folder")));
    let weak_root = root.downgrade();
    drop(root);
    let folder = match provider.open_folder_picker_async(options).await {
        Ok(folders) => folders.into_iter().next(),
        Err(error) => return Some(format!("The folder picker failed: {error}")),
    };
    let root = weak_root.upgrade()?;
    let path = folder
        .map(|folder| path_of(&*folder))
        .filter(|path| !path.is_empty());
    Some(match path {
        Some(path) => {
            root.get_control::<TextBox>("SuggestedFolderTextBox")
                .set_text(Some(&path));
            format!("Folder selected: {path}")
        }
        None => String::from("No folder selected"),
    })
}

/// The size the thumbnail example asks for: the box's value, or the source's fallback of 200 when
/// it is empty (NaN) or not positive.
pub(crate) fn requested_thumbnail_size(value: f64) -> u32 {
    if value.is_nan() || value <= 0.0 {
        DEFAULT_THUMBNAIL_SIZE
    } else {
        value.min(f64::from(u32::MAX)) as u32
    }
}

/// The size of the thumbnail of an image of `size` pixels: the image within `requested` pixels on
/// its longer side, keeping its aspect ratio; a smaller image keeps its size.
pub(crate) fn thumbnail_size(size: PixelSize, requested: u32) -> PixelSize {
    let longer = size.width.max(size.height);
    let requested = i32::try_from(requested).unwrap_or(i32::MAX);
    if longer <= requested || longer <= 0 {
        return size;
    }
    let scale = f64::from(requested) / f64::from(longer);
    let side = |value: i32| ((f64::from(value) * scale).round() as i32).max(1);
    PixelSize::new(side(size.width), side(size.height))
}

/// The details of a thumbnail, as the source's.
pub(crate) fn thumbnail_details(
    name: &str,
    mode: &str,
    requested: u32,
    returned: PixelSize,
) -> String {
    format!(
        "File: {name}\nMode: ThumbnailMode.{mode}\nRequested size: {requested}\nReturned size: {} x {}",
        returned.width, returned.height
    )
}

/// `PickFileForThumbnailButton_Click`. FerroUI has no shell thumbnails (`GetThumbnailAsync`): the
/// thumbnail of an image file is the image decoded and scaled within the requested size, the same
/// for every `ThumbnailMode`; a file that is not an image has none.
async fn pick_thumbnail(root: Ref<Control>, provider: Rc<dyn IStorageProvider>) -> Option<String> {
    let mode = THUMBNAIL_MODES
        .get(selected_index(&root, "ThumbnailModeComboBox"))
        .copied()
        .unwrap_or("PicturesView");
    let requested = requested_thumbnail_size(
        root.get_control::<NumberBox>("ThumbnailSizeNumberBox")
            .value(),
    );
    let weak_root = root.downgrade();
    drop(root);
    let options =
        FilePickerOpenOptions::new().with_file_type_filter(vec![FilePickerFileTypes::all()]);
    let set_image = |image: Option<Rc<dyn IImage>>| {
        if let Some(root) = weak_root.upgrade() {
            root.get_control::<Image>("ThumbnailImage")
                .set_source(image);
        }
    };
    let file = match provider.open_file_picker_async(options).await {
        Ok(files) => files.into_iter().next(),
        Err(error) => {
            set_image(None);
            return Some(format!("The file picker failed: {error}"));
        }
    };
    let Some(file) = file else {
        set_image(None);
        return Some(String::from("No file selected."));
    };
    let name = file.name();
    let bitmap = match read_all(&file).await {
        Ok(bytes) => Bitmap::from_stream(&mut &bytes[..]).ok(),
        Err(error) => {
            set_image(None);
            return Some(format!(
                "Could not retrieve a thumbnail for \"{name}\" using ThumbnailMode.{mode}.\n\
                 Try a different mode (for example, SingleItem) or a different file.\n({error})"
            ));
        }
    };
    let Some(bitmap) = bitmap else {
        set_image(None);
        return Some(String::from(
            "No thumbnail available for the selected file.",
        ));
    };
    let size = thumbnail_size(bitmap.pixel_size(), requested);
    let thumbnail = if size == bitmap.pixel_size() {
        bitmap
    } else {
        bitmap.create_scaled_bitmap(size, BitmapInterpolationMode::HighQuality)
    };
    set_image(Some(Rc::new(thumbnail) as Rc<dyn IImage>));
    Some(thumbnail_details(&name, mode, requested, size))
}

async fn read_all(file: &Rc<dyn IStorageFile>) -> std::io::Result<Vec<u8>> {
    let mut stream = file.open_read_async().await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    Ok(bytes)
}
