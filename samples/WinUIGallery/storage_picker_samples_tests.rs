//! The StoragePickers examples over a test storage provider: the options each handler gives the
//! picker, the outputs of the source and the pickers a platform does not have. The provider's
//! pickers answer with files and folders of a temporary directory.

use crate::storage_picker_samples::{
    file_type_choices, file_type_filter, requested_thumbnail_size, thumbnail_details,
    thumbnail_size, DEFAULT_THUMBNAIL_SIZE, NO_FILE_PICKER, NO_FOLDER_PICKER, NO_SAVE_PICKER,
    NO_STORAGE, PICKER_LOCATIONS, PICKER_VIEW_MODES, THUMBNAIL_MODES,
};
use crate::{examples, tests::start};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::platform::storage::file_io::{
    BclStorageFile, BclStorageFolder, BclStorageProvider, FileSystemInfo,
};
use ferroui_base::platform::storage::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFile,
    IStorageFolder, IStorageItem, IStorageProvider, OpenFilePickerResult, SaveFilePickerResult,
    WellKnownFolder,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Ref};
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{
    Button, CheckBox, ComboBox, Control, Image, NumberBox, TextBlock, TextBox, Window,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// What the page asked a picker for.
#[derive(Clone, Debug, Default, PartialEq)]
struct Request {
    title: Option<String>,
    start: Option<String>,
    allow_multiple: bool,
    /// The patterns of each file type of the filter or the choices.
    types: Option<Vec<Vec<String>>>,
    names: Vec<String>,
    default_extension: Option<String>,
    file_name: Option<String>,
}

/// A picker that does not answer until it is opened.
#[derive(Default)]
struct Gate {
    open: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

struct GateWait(Rc<Gate>);

impl Future for GateWait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.0.open.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(context.waker().clone());
            Poll::Pending
        }
    }
}

impl Gate {
    fn release(&self) {
        self.open.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

#[derive(Default)]
struct PickerHost {
    no_pickers: Cell<bool>,
    files: RefCell<Vec<Rc<dyn IStorageFile>>>,
    save: RefCell<Option<Rc<dyn IStorageFile>>>,
    folders: RefCell<Vec<Rc<dyn IStorageFolder>>>,
    well_known: RefCell<Vec<WellKnownFolder>>,
    well_known_folder: RefCell<Option<PathBuf>>,
    opened: RefCell<Vec<Request>>,
    saved: RefCell<Vec<Request>>,
    folder_requests: RefCell<Vec<Request>>,
    gate: RefCell<Option<Rc<Gate>>>,
}

fn local_path(folder: Option<&Rc<dyn IStorageFolder>>) -> Option<String> {
    folder.and_then(|folder| (&**folder as &dyn IStorageItem).try_get_local_path())
}

fn patterns(
    types: Option<&[Rc<ferroui_base::platform::storage::FilePickerFileType>]>,
) -> Option<Vec<Vec<String>>> {
    types.map(|types| {
        types
            .iter()
            .map(|file_type| file_type.patterns().unwrap_or_default().to_vec())
            .collect()
    })
}

fn names(types: Option<&[Rc<ferroui_base::platform::storage::FilePickerFileType>]>) -> Vec<String> {
    types
        .map(|types| {
            types
                .iter()
                .map(|file_type| file_type.name().to_string())
                .collect()
        })
        .unwrap_or_default()
}

impl PickerHost {
    fn answer<T: 'static>(&self, value: T) -> LocalBoxFuture<T> {
        let gate = self.gate.borrow().clone();
        Box::pin(async move {
            if let Some(gate) = gate {
                GateWait(gate).await;
            }
            value
        })
    }
}

impl BclStorageProvider for PickerHost {
    fn can_open(&self) -> bool {
        !self.no_pickers.get()
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        self.opened.borrow_mut().push(Request {
            title: options.title().map(str::to_string),
            start: local_path(options.suggested_start_location()),
            allow_multiple: options.allow_multiple(),
            types: patterns(options.file_type_filter()),
            names: names(options.file_type_filter()),
            ..Request::default()
        });
        let mut files = self.files.borrow().clone();
        if !options.allow_multiple() {
            files.truncate(1);
        }
        self.answer(Ok(OpenFilePickerResult {
            files,
            selected_file_type: None,
        }))
    }

    fn can_save(&self) -> bool {
        !self.no_pickers.get()
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        self.saved.borrow_mut().push(Request {
            title: options.title().map(str::to_string),
            start: local_path(options.suggested_start_location()),
            types: patterns(options.file_type_choices()),
            names: names(options.file_type_choices()),
            default_extension: options.default_extension().map(str::to_string),
            file_name: options.suggested_file_name().map(str::to_string),
            ..Request::default()
        });
        self.answer(Ok(SaveFilePickerResult {
            file: self.save.borrow().clone(),
            selected_file_type: None,
        }))
    }

    fn can_pick_folder(&self) -> bool {
        !self.no_pickers.get()
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        self.folder_requests.borrow_mut().push(Request {
            title: options.title().map(str::to_string),
            start: local_path(options.suggested_start_location()),
            allow_multiple: options.allow_multiple(),
            ..Request::default()
        });
        self.answer(Ok(self.folders.borrow().clone()))
    }

    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        self.well_known.borrow_mut().push(well_known_folder);
        let folder = self.well_known_folder.borrow().as_ref().map(|path| {
            BclStorageFolder::new(FileSystemInfo::directory(path)) as Rc<dyn IStorageFolder>
        });
        Box::pin(std::future::ready(folder))
    }
}

/// A new temporary directory of the test `name`.
fn directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ferroui-gallery-storage-pickers-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn file(path: PathBuf) -> Rc<dyn IStorageFile> {
    BclStorageFile::new(FileSystemInfo::file(path))
}

fn folder(path: PathBuf) -> Rc<dyn IStorageFolder> {
    BclStorageFolder::new(FileSystemInfo::directory(path))
}

fn full_name(path: &std::path::Path) -> String {
    FileSystemInfo::file(path).full_name()
}

/// The page in a window whose platform storage provider is `host` (none for `None`).
fn page(host: Option<&Rc<PickerHost>>) -> (Ref<Window>, Ref<Control>) {
    let root = examples::create(examples::find("StoragePickers").unwrap()).unwrap();
    let platform = MockWindowingPlatform::create_window_mock();
    if let Some(host) = host {
        platform.setup_feature::<dyn IStorageProvider>(host.clone() as Rc<dyn IStorageProvider>);
    }
    let window = Window::with_impl(platform);
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    (window, root)
}

fn click(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn text(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<TextBlock>(name).text()
}

#[test]
fn storage_pickers_page_fills_the_option_lists_of_the_source() {
    let _application = start();
    let (window, root) = page(None);
    let names = |combo: &str| {
        let combo = root.get_control::<ComboBox>(combo);
        assert_eq!(combo.selected_index(), 0);
        let items = combo.items();
        (0..items.count())
            .map(|index| {
                items
                    .get_at(index)
                    .and_then(|item| item.downcast_ref::<String>().cloned())
                    .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let locations: Vec<_> = PICKER_LOCATIONS
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    for index in 1..=4 {
        assert_eq!(names(&format!("PickerLocationComboBox{index}")), locations);
    }
    for index in 1..=3 {
        assert_eq!(
            names(&format!("PickerViewModeComboBox{index}")),
            PICKER_VIEW_MODES
        );
    }
    assert_eq!(names("ThumbnailModeComboBox"), THUMBNAIL_MODES);
    assert_eq!(
        text(&root, "PickedSingleFileTextBlock").as_deref(),
        Some("No file picked")
    );
    assert_eq!(
        text(&root, "SavedFileTextBlock").as_deref(),
        Some("No file saved")
    );
    assert_eq!(
        root.get_control::<NumberBox>("ThumbnailSizeNumberBox")
            .value(),
        200.0
    );
    window.close();
}

#[test]
fn storage_pickers_pick_files_with_the_filter_title_and_start_location() {
    let _application = start();
    let directory = directory("files");
    let host = Rc::new(PickerHost::default());
    *host.well_known_folder.borrow_mut() = Some(directory.clone());
    let first = directory.join("first.txt");
    let second = directory.join("second.png");
    *host.files.borrow_mut() = vec![file(first.clone()), file(second.clone())];
    let (window, root) = page(Some(&host));

    // PickSingleFileButton_Click: All Files adds no filter; the commit text is the title and the
    // DocumentsLibrary location the documents folder.
    click(&root, "PickSingleFileButton");
    assert_eq!(
        text(&root, "PickedSingleFileTextBlock"),
        Some(format!("Picked: {}", full_name(&first)))
    );
    let request = host.opened.borrow()[0].clone();
    assert_eq!(request.title.as_deref(), Some("Pick File"));
    assert_eq!(request.types, None);
    assert!(!request.allow_multiple);
    assert_eq!(
        request.start,
        Some(FileSystemInfo::directory(&directory).full_name())
    );
    assert_eq!(
        host.well_known.borrow().as_slice(),
        [WellKnownFolder::Documents]
    );
    assert!(root
        .get_control::<Button>("PickSingleFileButton")
        .is_enabled());

    // The text and image filters of the Tags, a location without a well-known folder and an
    // empty commit text (no title).
    root.get_control::<ComboBox>("FileTypeComboBox1")
        .set_selected_index(2);
    root.get_control::<ComboBox>("PickerLocationComboBox1")
        .set_selected_index(1);
    root.get_control::<TextBox>("CommitButtonTextTextBox")
        .set_text(Some(""));
    click(&root, "PickSingleFileButton");
    let request = host.opened.borrow()[1].clone();
    assert_eq!(
        request.types,
        Some(vec![vec![String::from("*.jpg"), String::from("*.png")]])
    );
    assert_eq!(request.title, None);
    assert_eq!(request.start, None);
    assert_eq!(host.well_known.borrow().len(), 1);

    // PickMultipleFilesButton_Click: All Files is the `*` filter, a line per file.
    root.get_control::<ComboBox>("PickerLocationComboBox2")
        .set_selected_index(5);
    click(&root, "PickMultipleFilesButton");
    assert_eq!(
        text(&root, "PickedMultipleFilesTextBlock"),
        Some(format!(
            "- Picked: {}\n- Picked: {}\n",
            full_name(&first),
            full_name(&second)
        ))
    );
    let request = host.opened.borrow()[2].clone();
    assert!(request.allow_multiple);
    assert_eq!(request.title.as_deref(), Some("Pick Files"));
    assert_eq!(request.names, ["All"]);
    assert_eq!(
        host.well_known.borrow().last(),
        Some(&WellKnownFolder::Pictures)
    );

    // A canceled picker.
    host.files.borrow_mut().clear();
    click(&root, "PickSingleFileButton");
    assert_eq!(
        text(&root, "PickedSingleFileTextBlock").as_deref(),
        Some("No file selected.")
    );
    click(&root, "PickMultipleFilesButton");
    assert_eq!(
        text(&root, "PickedMultipleFilesTextBlock").as_deref(),
        Some("No files selected.")
    );
    window.close();
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn storage_pickers_save_the_content_with_the_choices_and_the_suggested_folder() {
    let _application = start();
    let directory = directory("save");
    let suggested = directory.join("suggested");
    std::fs::create_dir_all(&suggested).unwrap();
    let target = directory.join("NewDocument.json");
    let host = Rc::new(PickerHost::default());
    *host.save.borrow_mut() = Some(file(target.clone()));
    *host.folders.borrow_mut() = vec![folder(suggested.clone())];
    let (window, root) = page(Some(&host));

    // SelectSuggestedFolderButton_Click: the picked folder is the suggested folder.
    click(&root, "SelectSuggestedFolderButton");
    let suggested_name = FileSystemInfo::directory(&suggested).full_name();
    assert_eq!(
        host.folder_requests.borrow()[0].title.as_deref(),
        Some("Select folder")
    );
    assert_eq!(
        root.get_control::<TextBox>("SuggestedFolderTextBox").text(),
        Some(suggested_name.clone())
    );
    assert_eq!(
        text(&root, "SuggestedFolderStatus"),
        Some(format!("Folder selected: {suggested_name}"))
    );

    // SaveFileButton_Click with the checked choices and the second default extension.
    root.get_control::<CheckBox>("TxtCheckBox")
        .set_is_checked(Some(true));
    root.get_control::<CheckBox>("XmlCheckBox")
        .set_is_checked(Some(true));
    root.get_control::<ComboBox>("DefaultExtensionComboBox")
        .set_selected_index(1);
    root.get_control::<TextBox>("FileContentTextBox")
        .set_text(Some("Saved by the gallery"));
    click(&root, "SaveFileButton");
    assert_eq!(
        text(&root, "SavedFileTextBlock"),
        Some(format!("File saved to: {}", full_name(&target)))
    );
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "Saved by the gallery"
    );
    let request = host.saved.borrow()[0].clone();
    assert_eq!(request.names, ["Text Files", "XML Files"]);
    assert_eq!(
        request.types,
        Some(vec![
            vec![String::from("*.txt")],
            vec![String::from("*.xml")]
        ])
    );
    assert_eq!(request.default_extension.as_deref(), Some("json"));
    assert_eq!(request.file_name.as_deref(), Some("NewDocument"));
    assert_eq!(request.title.as_deref(), Some("Save File"));
    // The suggested folder is the start location, in place of the documents folder.
    assert_eq!(request.start, Some(suggested_name));
    assert!(host.well_known.borrow().is_empty());

    // A canceled save.
    host.save.borrow_mut().take();
    click(&root, "SaveFileButton");
    assert_eq!(
        text(&root, "SavedFileTextBlock").as_deref(),
        Some("File save canceled.")
    );
    window.close();
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn storage_pickers_pick_a_folder_and_keep_the_button_disabled_while_it_is_open() {
    let _application = start();
    let directory = directory("folder");
    let host = Rc::new(PickerHost::default());
    *host.folders.borrow_mut() = vec![folder(directory.clone())];
    let gate = Rc::new(Gate::default());
    *host.gate.borrow_mut() = Some(gate.clone());
    let (window, root) = page(Some(&host));
    root.get_control::<ComboBox>("PickerLocationComboBox4")
        .set_selected_index(3);

    click(&root, "PickFolderButton");
    let button = root.get_control::<Button>("PickFolderButton");
    assert!(!button.is_enabled(), "disabled while the picker is open");
    assert_eq!(
        text(&root, "PickedFolderTextBlock").as_deref(),
        Some("No folder picked")
    );
    let request = host.folder_requests.borrow()[0].clone();
    assert_eq!(request.title.as_deref(), Some("Pick Folder"));
    assert!(!request.allow_multiple);
    assert_eq!(
        host.well_known.borrow().as_slice(),
        [WellKnownFolder::Downloads]
    );

    gate.release();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(button.is_enabled());
    assert_eq!(
        text(&root, "PickedFolderTextBlock"),
        Some(format!(
            "Picked: {}",
            FileSystemInfo::directory(&directory).full_name()
        ))
    );

    // A canceled picker.
    *host.gate.borrow_mut() = None;
    host.folders.borrow_mut().clear();
    click(&root, "PickFolderButton");
    assert_eq!(
        text(&root, "PickedFolderTextBlock").as_deref(),
        Some("No folder selected.")
    );
    window.close();
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn storage_pickers_thumbnail_reports_the_file_without_an_image_and_a_failed_read() {
    let _application = start();
    let directory = directory("thumbnail");
    let notes = directory.join("notes.txt");
    std::fs::write(&notes, "not an image").unwrap();
    let host = Rc::new(PickerHost::default());
    *host.files.borrow_mut() = vec![file(notes)];
    let (window, root) = page(Some(&host));

    click(&root, "PickFileForThumbnailButton");
    assert_eq!(host.opened.borrow()[0].names, ["All"]);
    assert_eq!(
        text(&root, "ThumbnailDetailsTextBlock").as_deref(),
        Some("No thumbnail available for the selected file.")
    );
    assert!(root
        .get_control::<Image>("ThumbnailImage")
        .source()
        .is_none());

    // A file that cannot be read: the source's catch.
    *host.files.borrow_mut() = vec![file(directory.join("missing.png"))];
    root.get_control::<ComboBox>("ThumbnailModeComboBox")
        .set_selected_index(5);
    click(&root, "PickFileForThumbnailButton");
    let details = text(&root, "ThumbnailDetailsTextBlock").unwrap();
    assert!(
        details.starts_with(
            "Could not retrieve a thumbnail for \"missing.png\" using ThumbnailMode.SingleItem.\n"
        ),
        "{details}"
    );

    host.files.borrow_mut().clear();
    click(&root, "PickFileForThumbnailButton");
    assert_eq!(
        text(&root, "ThumbnailDetailsTextBlock").as_deref(),
        Some("No file selected.")
    );
    window.close();
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn storage_pickers_report_a_window_or_a_platform_without_pickers() {
    let _application = start();
    // Not in a window: no storage service.
    let root = examples::create(examples::find("StoragePickers").unwrap()).unwrap();
    click(&root, "PickSingleFileButton");
    assert_eq!(
        text(&root, "PickedSingleFileTextBlock").as_deref(),
        Some(NO_STORAGE)
    );

    // A window whose platform has no storage provider has no pickers.
    let (window, root) = page(None);
    for (button, output, message) in [
        (
            "PickSingleFileButton",
            "PickedSingleFileTextBlock",
            NO_FILE_PICKER,
        ),
        (
            "PickMultipleFilesButton",
            "PickedMultipleFilesTextBlock",
            NO_FILE_PICKER,
        ),
        ("SaveFileButton", "SavedFileTextBlock", NO_SAVE_PICKER),
        (
            "PickFolderButton",
            "PickedFolderTextBlock",
            NO_FOLDER_PICKER,
        ),
        (
            "SelectSuggestedFolderButton",
            "SuggestedFolderStatus",
            NO_FOLDER_PICKER,
        ),
        (
            "PickFileForThumbnailButton",
            "ThumbnailDetailsTextBlock",
            NO_FILE_PICKER,
        ),
    ] {
        click(&root, button);
        assert_eq!(text(&root, output).as_deref(), Some(message), "{button}");
        assert!(root.get_control::<Button>(button).is_enabled(), "{button}");
    }
    window.close();

    // A provider that reports no pickers is not asked to open one.
    let host = Rc::new(PickerHost::default());
    host.no_pickers.set(true);
    let (window, root) = page(Some(&host));
    click(&root, "SaveFileButton");
    assert_eq!(
        text(&root, "SavedFileTextBlock").as_deref(),
        Some(NO_SAVE_PICKER)
    );
    assert!(host.saved.borrow().is_empty());
    window.close();
}

#[test]
fn storage_pickers_map_the_file_types_and_the_thumbnail_size() {
    assert_eq!(file_type_filter("*", false), None);
    let all = file_type_filter("*", true).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name(), "All");
    let text = file_type_filter(".txt", false).unwrap();
    assert_eq!(text[0].patterns().unwrap().to_vec(), ["*.txt"]);
    assert_eq!(
        file_type_choices(false, false, false).map(|choices| choices.len()),
        None
    );
    let choices = file_type_choices(false, true, false).unwrap();
    assert_eq!(choices[0].name(), "JSON Files");
    assert_eq!(choices[0].patterns().unwrap().to_vec(), ["*.json"]);

    // NumberBox.Value is NaN when the box is cleared: the source's fallback of 200.
    assert_eq!(requested_thumbnail_size(f64::NAN), DEFAULT_THUMBNAIL_SIZE);
    assert_eq!(requested_thumbnail_size(-4.0), DEFAULT_THUMBNAIL_SIZE);
    assert_eq!(requested_thumbnail_size(96.7), 96);
    // The image within the requested size on its longer side; a smaller image keeps its size.
    assert_eq!(
        thumbnail_size(PixelSize::new(800, 600), 200),
        PixelSize::new(200, 150)
    );
    assert_eq!(
        thumbnail_size(PixelSize::new(300, 1200), 200),
        PixelSize::new(50, 200)
    );
    assert_eq!(
        thumbnail_size(PixelSize::new(120, 80), 200),
        PixelSize::new(120, 80)
    );
    assert_eq!(
        thumbnail_size(PixelSize::new(5000, 1), 16),
        PixelSize::new(16, 1)
    );
    assert_eq!(
        thumbnail_details("rainier.jpg", "PicturesView", 200, PixelSize::new(200, 133)),
        "File: rainier.jpg\nMode: ThumbnailMode.PicturesView\nRequested size: 200\nReturned size: 200 x 133"
    );
}
