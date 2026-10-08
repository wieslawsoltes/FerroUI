//! Port of `Pages/DialogsPage.xaml.cs`: the class of the document
//! `Pages/DialogsPage.xaml`.
//!
//! The local variables and local functions the constructor of the managed
//! original shares between its event handlers are fields and methods of
//! the page here: every handler holds the page weakly and reaches them
//! through it.

use crate::markup::xaml_class;
use crate::DecoratedWindow;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerFileTypes, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions,
    IStorageFile, IStorageFolder, IStorageItem, IStorageProvider, WellKnownFolder,
};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    AutoCompleteBox, Button, CheckBox, ComboBox, ComboBoxItem, ContentPage, Control, ControlImpl, ItemsControl,
    ItemsSource, PageImpl, StackPanel, TextBlock, TextBox, TopLevel, Window, WindowStartupLocation,
};
// The managed storage provider works on the local file system, which the browser does not have.
#[cfg(not(target_arch = "wasm32"))]
use ferroui_dialogs::ManagedStorageProvider;
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::rc::Rc;

/// `Environment.NewLine`.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// The message of the exception `GetWindow()` and `GetTopLevel()` throw.
const INVALID_OWNER: &str = "Invalid Owner";

#[repr(C)]
pub struct DialogsPage {
    base: ContentPage,
    last_selected_directory: RefCell<Option<Rc<dyn IStorageFolder>>>,
    last_selected_item: RefCell<Option<Rc<dyn IStorageItem>>>,
    ignore_text_changed: Cell<bool>,
}

ferro_class!(DialogsPage: ContentPage);
ferro_impl_classes!(
    DialogsPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(DialogsPage { new: DialogsPage::new });
xaml_class!(DialogsPage, "/Pages/DialogsPage.xaml");

impl VisualImpl for DialogsPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let opened_file_content = this.opened_file_content();
        match this.try_get_storage_provider() {
            Ok(storage_provider) => opened_file_content.set_text(Some(&format!(
                "CanOpen: {}{NEW_LINE}CanSave: {}{NEW_LINE}CanPickFolder: {}",
                bool_text(storage_provider.can_open()),
                bool_text(storage_provider.can_save()),
                bool_text(storage_provider.can_pick_folder())
            ))),
            Err(message) => {
                opened_file_content.set_text(Some(&format!("Storage provider is not available: {message}")));
            }
        }
    }
}

/// The text of a `bool` (`Boolean.ToString()`).
fn bool_text(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

/// The text of a nullable value in an interpolated string: nothing for null.
fn nullable_text<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// `Enum.TryParse<WellKnownFolder>(text, ignoreCase: true, out var folder)`:
/// the name of a member, whatever its case, or the number of a member, with
/// white space around it. (A number that is no member is not parsed here:
/// the managed original passes it on as an undefined value.)
fn try_parse_well_known_folder(text: Option<&str>) -> Option<WellKnownFolder> {
    const MEMBERS: [(&str, WellKnownFolder); 6] = [
        ("Desktop", WellKnownFolder::Desktop),
        ("Documents", WellKnownFolder::Documents),
        ("Downloads", WellKnownFolder::Downloads),
        ("Music", WellKnownFolder::Music),
        ("Pictures", WellKnownFolder::Pictures),
        ("Videos", WellKnownFolder::Videos),
    ];

    let text = text?.trim();
    if let Ok(number) = text.parse::<i32>() {
        return MEMBERS.iter().find(|(_, member)| *member as i32 == number).map(|(_, member)| *member);
    }
    MEMBERS.iter().find(|(name, _)| name.eq_ignore_ascii_case(text)).map(|(_, member)| *member)
}

/// `FullPathOrName(item)`, for an item that is not null (no caller passes null).
fn full_path_or_name(item: &dyn IStorageItem) -> String {
    let path = item.path();
    if path.is_absolute_uri() {
        path.to_string()
    } else {
        item.name()
    }
}

/// A text as the untyped content of a control.
fn text_content(text: &str) -> BoxedValue {
    Rc::new(text.to_string())
}

/// The outcome of a storage operation awaited by an `async void` handler:
/// an exception of such a method is raised on the dispatcher.
///
/// # Panics
/// Panics if the operation failed.
fn raise<T>(result: std::io::Result<T>) -> T {
    result.unwrap_or_else(|error| panic!("{error}"))
}

impl DialogsPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            last_selected_directory: RefCell::new(None),
            last_selected_item: RefCell::new(None),
            ignore_text_changed: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The controls are descendants of the page: their handlers hold the page weakly.
        {
            let weak = this.downgrade();
            this.current_folder_box().text_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.current_folder_box_text_changed();
                }
            });
        }

        {
            let weak = this.downgrade();
            this.use_suggested_filter().is_checked_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.update_suggested_filter_selector_state();
                }
            });
        }
        this.update_suggested_filter_selector_state();

        {
            let weak = this.downgrade();
            this.filter_selector().selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.update_suggested_filter_selector(this.build_file_types().as_deref());
                }
            });
        }
        this.update_suggested_filter_selector(this.build_file_types().as_deref());

        this.on_click("DecoratedWindow", |_| {
            DecoratedWindow::new().show();
        });
        this.on_click("DecoratedWindowDialog", |this| {
            drop(DecoratedWindow::new().show_dialog(&this.get_window()));
        });
        this.on_click("Dialog", |this| {
            let window = Self::create_sample_window();
            window.set_height(200.0);
            drop(window.show_dialog(&this.get_window()));
        });
        this.on_click("DialogNoTaskbar", |this| {
            let window = Self::create_sample_window();
            window.set_height(200.0);
            window.set_show_in_taskbar(false);
            drop(window.show_dialog(&this.get_window()));
        });
        this.on_click("OwnedWindow", |this| {
            let window = Self::create_sample_window();

            window.show_with_owner(&this.get_window());
        });
        this.on_click("OwnedWindowNoTaskbar", |this| {
            let window = Self::create_sample_window();

            window.set_show_in_taskbar(false);

            window.show_with_owner(&this.get_window());
        });

        this.on_click("OpenFilePicker", |this| {
            let this = this.clone();
            drop(start_async(async move { this.open_file_picker_click().await }));
        });
        this.on_click("SaveFilePicker", |this| {
            let this = this.clone();
            drop(start_async(async move { this.save_file_picker_click().await }));
        });
        this.on_click("SaveFilePickerWithResult", |this| {
            let this = this.clone();
            drop(start_async(async move { this.save_file_picker_with_result_click().await }));
        });
        this.on_click("OpenFolderPicker", |this| {
            let this = this.clone();
            drop(start_async(async move { this.open_folder_picker_click().await }));
        });
        this.on_click("OpenFileFromBookmark", |this| {
            let this = this.clone();
            drop(start_async(async move { this.open_file_from_bookmark_click().await }));
        });
        this.on_click("OpenFolderFromBookmark", |this| {
            let this = this.clone();
            drop(start_async(async move { this.open_folder_from_bookmark_click().await }));
        });

        this.on_click("LaunchUri", |this| {
            let this = this.clone();
            drop(start_async(async move { this.launch_uri_click().await }));
        });
        this.on_click("LaunchFile", |this| {
            let this = this.clone();
            drop(start_async(async move { this.launch_file_click().await }));
        });

        this
    }

    /// Adds `handler` to the click of the button named `name`. The button
    /// is a descendant of the page: the handler holds the page weakly.
    fn on_click(&self, name: &str, handler: impl Fn(&Ref<DialogsPage>) + 'static) {
        let weak = self.to_ref().downgrade();
        self.get_control::<Button>(name).click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                handler(&this);
            }
        });
    }

    fn picker_last_results(&self) -> Ref<ItemsControl> {
        self.get_control::<ItemsControl>("PickerLastResults")
    }

    fn picker_last_results_visible(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("PickerLastResultsVisible")
    }

    fn bookmark_container(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("BookmarkContainer")
    }

    fn opened_file_content(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("OpenedFileContent")
    }

    fn open_multiple(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("OpenMultiple")
    }

    fn current_folder_box(&self) -> Ref<AutoCompleteBox> {
        self.get_control::<AutoCompleteBox>("CurrentFolderBox")
    }

    fn use_suggested_filter(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("UseSuggestedFilter")
    }

    fn suggested_filter_selector(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("SuggestedFilterSelector")
    }

    fn filter_selector(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("FilterSelector")
    }

    fn force_managed(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("ForceManaged")
    }

    fn uri_to_launch(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("UriToLaunch")
    }

    fn launch_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LaunchStatus")
    }

    /// The item `combo_box` has selected, when it is a `ComboBoxItem`
    /// (`SelectedItem as ComboBoxItem`).
    fn selected_combo_box_item(combo_box: &ComboBox) -> Option<Ref<ComboBoxItem>> {
        combo_box.selected_item().and_then(|item| Control::from_boxed(&item))?.cast::<ComboBoxItem>()
    }

    /// The file type the tag of `item` holds (`Tag as FilePickerFileType`).
    fn file_type_of(item: &ComboBoxItem) -> Option<Rc<FilePickerFileType>> {
        item.tag().and_then(|tag| tag.downcast_ref::<Rc<FilePickerFileType>>().cloned())
    }

    /// The handler of `CurrentFolderBox.TextChanged` (`async void`).
    fn current_folder_box_text_changed(&self) {
        if self.ignore_text_changed.get() {
            return;
        }

        let text = self.current_folder_box().text();
        let this = self.to_ref();
        if let Some(folder_enum) = try_parse_well_known_folder(text.as_deref()) {
            let pending = self.get_storage_provider().try_get_well_known_folder_async(folder_enum);
            drop(start_async(async move {
                let folder = pending.await;
                *this.last_selected_directory.borrow_mut() = folder;
            }));
        } else if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
            let folder_link = Uri::try_create(&text, UriKind::Absolute)
                .or_else(|| Uri::try_create(&format!("file://{text}"), UriKind::Absolute));

            if let Some(folder_link) = folder_link {
                let pending = self.get_storage_provider().try_get_folder_from_path_async(&folder_link);
                drop(start_async(async move {
                    let folder = pending.await;
                    *this.last_selected_directory.borrow_mut() = folder;
                }));
            }
        }
    }

    /// `BuildFileTypes()`.
    fn build_file_types(&self) -> Option<Vec<Rc<FilePickerFileType>>> {
        let selected_item = Self::selected_combo_box_item(&self.filter_selector())
            .and_then(|item| item.content())
            .and_then(|content| content.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| String::from("None"));

        let bin_log_type = FilePickerFileType::new(Some("Binary Log"))
            .with_patterns(&["*.binlog", "*.buildlog"])
            .with_mime_types(&["application/binlog", "application/buildlog"])
            .with_apple_uniform_type_identifiers(&["public.data"]);

        match selected_item.as_str() {
            "All + TXT + BinLog" => {
                Some(vec![FilePickerFileTypes::all(), FilePickerFileTypes::text_plain(), bin_log_type])
            }
            "Binlog" => Some(vec![bin_log_type]),
            "TXT extension only" => {
                let type_ = FilePickerFileType::new(Some("TXT"));
                type_.set_patterns(FilePickerFileTypes::text_plain().patterns().map(|patterns| patterns.to_vec()));
                Some(vec![type_])
            }
            "TXT mime only" => {
                let type_ = FilePickerFileType::new(Some("TXT"));
                type_.set_mime_types(FilePickerFileTypes::text_plain().mime_types().map(|types| types.to_vec()));
                Some(vec![type_])
            }
            "TXT apple type id only" => {
                let type_ = FilePickerFileType::new(Some("TXT"));
                type_.set_apple_uniform_type_identifiers(
                    FilePickerFileTypes::text_plain()
                        .apple_uniform_type_identifiers()
                        .map(|identifiers| identifiers.to_vec()),
                );
                Some(vec![type_])
            }
            _ => None,
        }
    }

    /// `GetFileTypes()`.
    fn get_file_types(&self) -> Option<Vec<Rc<FilePickerFileType>>> {
        let types = self.build_file_types();
        self.update_suggested_filter_selector(types.as_deref());
        types
    }

    /// `UpdateSuggestedFilterSelector(types)`.
    fn update_suggested_filter_selector(&self, types: Option<&[Rc<FilePickerFileType>]>) {
        let suggested_filter_selector = self.suggested_filter_selector();
        let previously_selected =
            Self::selected_combo_box_item(&suggested_filter_selector).and_then(|item| Self::file_type_of(&item));
        let items = suggested_filter_selector.items();
        items.clear();
        {
            let item = ComboBoxItem::new();
            item.set_content(Some(text_content("First filter")));
            item.set_tag(None);
            items.add(Some(Control::boxed(item)));
        }

        let mut desired_index = 0;
        if let Some(types) = types {
            for (i, type_) in types.iter().enumerate() {
                let item = ComboBoxItem::new();
                item.set_content(Some(text_content(type_.name())));
                item.set_tag(Some(Rc::new(type_.clone()) as BoxedValue));
                items.add(Some(Control::boxed(item)));

                if previously_selected.as_ref().is_some_and(|previous| Rc::ptr_eq(previous, type_)) {
                    desired_index = i as i32 + 1;
                }
            }
        }

        suggested_filter_selector.set_selected_index(desired_index);
    }

    /// `GetSuggestedFileType(types)`.
    fn get_suggested_file_type(&self, types: Option<&[Rc<FilePickerFileType>]>) -> Option<Rc<FilePickerFileType>> {
        let types = types.filter(|types| !types.is_empty())?;
        if self.use_suggested_filter().is_checked() != Some(true) {
            return None;
        }

        let selected_type =
            Self::selected_combo_box_item(&self.suggested_filter_selector()).and_then(|item| Self::file_type_of(&item));
        if let Some(selected_type) = selected_type {
            if types.iter().any(|type_| Rc::ptr_eq(type_, &selected_type)) {
                return Some(selected_type);
            }
        }

        types.first().cloned()
    }

    /// `UpdateSuggestedFilterSelectorState()`.
    fn update_suggested_filter_selector_state(&self) {
        self.suggested_filter_selector().set_is_enabled(self.use_suggested_filter().is_checked() == Some(true));
    }

    /// The handler of `OpenFilePicker.Click`.
    async fn open_file_picker_click(&self) {
        let file_types = self.get_file_types();
        let mut options = FilePickerOpenOptions::new();
        options.set_title(Some(String::from("Open file")));
        options.set_suggested_file_type(self.get_suggested_file_type(file_types.as_deref()));
        options.set_file_type_filter(file_types);
        options.set_suggested_file_name(Some(String::from("FileName")));
        options.set_suggested_start_location(self.last_selected_directory.borrow().clone());
        options.set_allow_multiple(self.open_multiple().is_checked() == Some(true));
        let pending = self.get_storage_provider().open_file_picker_async(options);
        let result = raise(pending.await);

        let items = result
            .into_iter()
            .map(|file| {
                let item: Rc<dyn IStorageItem> = file;
                item
            })
            .collect();
        self.set_picker_result(items, None).await;
    }

    /// The handler of `SaveFilePicker.Click`.
    async fn save_file_picker_click(&self) {
        let file_types = self.get_file_types();
        let suggested_type = self.get_suggested_file_type(file_types.as_deref());
        let mut options = FilePickerSaveOptions::new();
        options.set_title(Some(String::from("Save file")));
        options.set_file_type_choices(file_types);
        options.set_suggested_file_type(suggested_type);
        options.set_suggested_start_location(self.last_selected_directory.borrow().clone());
        options.set_suggested_file_name(Some(String::from("FileName")));
        options.set_show_overwrite_prompt(Some(true));
        let pending = self.get_storage_provider().save_file_picker_async(options);
        let file = raise(pending.await);

        if let Some(file) = &file {
            let text = self.opened_file_content().text().unwrap_or_default();
            match Self::write_line(file, &text).await {
                Ok(()) => {
                    let parent = file.get_parent_async().await;
                    self.set_folder(parent);
                }
                Err(error) => self.opened_file_content().set_text(Some(&error.to_string())),
            }
        }

        let items = file
            .into_iter()
            .map(|file| {
                let item: Rc<dyn IStorageItem> = file;
                item
            })
            .collect();
        self.set_picker_result(items, None).await;
    }

    /// The handler of `SaveFilePickerWithResult.Click`.
    async fn save_file_picker_with_result_click(&self) {
        let save_file_types = vec![FilePickerFileTypes::json(), FilePickerFileTypes::xml()];
        let mut options = FilePickerSaveOptions::new();
        options.set_title(Some(String::from("Save file")));
        options.set_suggested_file_type(self.get_suggested_file_type(Some(save_file_types.as_slice())));
        options.set_file_type_choices(Some(save_file_types));
        options.set_suggested_start_location(self.last_selected_directory.borrow().clone());
        options.set_suggested_file_name(Some(String::from("FileName")));
        options.set_show_overwrite_prompt(Some(true));
        let pending = self.get_storage_provider().save_file_picker_with_result_async(options);
        let result = raise(pending.await);

        if let Some(file) = &result.file {
            let is_xml = result
                .selected_file_type
                .as_ref()
                .is_some_and(|selected| Rc::ptr_eq(selected, &FilePickerFileTypes::xml()));
            let text = if is_xml { "<sample>Test</sample>" } else { r#"{ "sample": "Test" }"# };
            match Self::write_line(file, text).await {
                Ok(()) => {
                    let parent = file.get_parent_async().await;
                    self.set_folder(parent);
                }
                Err(error) => self.opened_file_content().set_text(Some(&error.to_string())),
            }
        }

        let items = result
            .file
            .clone()
            .into_iter()
            .map(|file| {
                let item: Rc<dyn IStorageItem> = file;
                item
            })
            .collect();
        self.set_picker_result(items, result.selected_file_type.clone()).await;
    }

    /// The handler of `OpenFolderPicker.Click`.
    async fn open_folder_picker_click(&self) {
        let mut options = FolderPickerOpenOptions::new();
        options.set_title(Some(String::from("Folder file")));
        options.set_suggested_start_location(self.last_selected_directory.borrow().clone());
        options.set_suggested_file_name(Some(String::from("FileName")));
        options.set_allow_multiple(self.open_multiple().is_checked() == Some(true));
        let pending = self.get_storage_provider().open_folder_picker_async(options);
        let folders = raise(pending.await);

        let items = folders
            .into_iter()
            .map(|folder| {
                let item: Rc<dyn IStorageItem> = folder;
                item
            })
            .collect();
        self.set_picker_result(items, None).await;
    }

    /// The handler of `OpenFileFromBookmark.Click`.
    async fn open_file_from_bookmark_click(&self) {
        let file = match self.bookmark_container().text() {
            Some(bookmark) => {
                let pending = self.get_storage_provider().open_file_bookmark_async(&bookmark);
                pending.await
            }
            None => None,
        };

        let items = file
            .into_iter()
            .map(|file| {
                let file: Rc<dyn IStorageFile> = file;
                let item: Rc<dyn IStorageItem> = file;
                item
            })
            .collect();
        self.set_picker_result(items, None).await;
    }

    /// The handler of `OpenFolderFromBookmark.Click`.
    async fn open_folder_from_bookmark_click(&self) {
        let folder = match self.bookmark_container().text() {
            Some(bookmark) => {
                let pending = self.get_storage_provider().open_folder_bookmark_async(&bookmark);
                pending.await
            }
            None => None,
        };

        let items = folder
            .into_iter()
            .map(|folder| {
                let folder: Rc<dyn IStorageFolder> = folder;
                let item: Rc<dyn IStorageItem> = folder;
                item
            })
            .collect();
        self.set_picker_result(items, None).await;
    }

    /// The handler of `LaunchUri.Click`.
    ///
    /// # Panics
    /// Panics if the page is not attached to a top level (a null reference
    /// in the managed original).
    async fn launch_uri_click(&self) {
        let status_block = self.launch_status();
        let uri = self.uri_to_launch().text().and_then(|text| Uri::try_create(&text, UriKind::Absolute));
        match uri {
            Some(uri) => {
                let top_level = TopLevel::get_top_level(Some(self)).expect("the page is attached to a top level");
                let pending = top_level.launcher().launch_uri_async(&uri);
                let result = pending.await;
                status_block.set_text(Some(&format!("LaunchUriAsync returned {}", bool_text(result))));
            }
            None => status_block.set_text(Some("Can't parse the Uri")),
        }
    }

    /// The handler of `LaunchFile.Click`.
    ///
    /// # Panics
    /// Panics if the page is not attached to a top level (a null reference
    /// in the managed original).
    async fn launch_file_click(&self) {
        let status_block = self.launch_status();
        let last_selected_item = self.last_selected_item.borrow().clone();
        match last_selected_item {
            Some(last_selected_item) => {
                let top_level = TopLevel::get_top_level(Some(self)).expect("the page is attached to a top level");
                let pending = top_level.launcher().launch_file_async(last_selected_item);
                let result = pending.await;
                status_block.set_text(Some(&format!("LaunchFileAsync returned {}", bool_text(result))));
            }
            None => status_block.set_text(Some("Please select any file or folder first")),
        }
    }

    /// Writes `text` and a line break to `file`, as the stream writer of the
    /// managed original does (`WriteLineAsync`), and closes the stream.
    async fn write_line(file: &Rc<dyn IStorageFile>, text: &str) -> std::io::Result<()> {
        let mut stream = file.open_write_async().await?;
        stream.write_all(text.as_bytes())?;
        stream.write_all(NEW_LINE.as_bytes())?;
        stream.flush()
    }

    /// `SetFolder(folder)`.
    fn set_folder(&self, folder: Option<Rc<dyn IStorageFolder>>) {
        self.ignore_text_changed.set(true);
        *self.last_selected_directory.borrow_mut() = folder.clone();
        *self.last_selected_item.borrow_mut() = folder.clone().map(|folder| {
            let item: Rc<dyn IStorageItem> = folder;
            item
        });
        let text = folder.map(|folder| {
            let path = folder.path();
            if path.is_absolute_uri() {
                path.local_path()
            } else {
                path.to_string()
            }
        });
        self.current_folder_box().set_text(text.as_deref());
        self.ignore_text_changed.set(false);
    }

    /// `SetPickerResult(items, selectedType)`.
    async fn set_picker_result(
        &self,
        items: Vec<Rc<dyn IStorageItem>>,
        selected_type: Option<Rc<FilePickerFileType>>,
    ) {
        let bookmark = match items.iter().find(|item| item.can_bookmark()) {
            Some(item) => item.save_bookmark_async().await,
            None => Some(String::from("Can't bookmark")),
        };
        self.bookmark_container().set_text(bookmark.as_deref());
        let mut mapped_results: Vec<String> = Vec::new();

        let mut result_text = String::new();
        if let Some(item) = items.first().cloned() {
            let file = item.clone().as_storage_file();
            let folder = item.clone().as_storage_folder();

            result_text += if file.is_some() { "File:" } else { "Folder:" };
            result_text += NEW_LINE;

            // The text of the managed original is a verbatim literal: its lines keep
            // the indentation of the source.
            let props = item.get_basic_properties_async().await;
            result_text += &format!(
                "Size: {}\n            DateCreated: {}\n            DateModified: {}\n            CanBookmark: {}\n            ",
                nullable_text(props.size()),
                nullable_text(props.date_created()),
                nullable_text(props.date_modified()),
                bool_text(item.can_bookmark())
            );
            if let Some(file) = file {
                result_text += "\n            Content:\n            ";

                match Self::read_text_from_file(file, 500).await {
                    Ok(content) => result_text += &content,
                    Err(error) => result_text += &error.to_string(),
                }
            }

            if let Some(storage_folder) = folder {
                self.set_folder(Some(storage_folder));
            } else {
                let parent = item.get_parent_async().await;
                self.set_folder(parent.clone());
                if let Some(parent) = parent {
                    mapped_results.push(full_path_or_name(&*parent));
                }
            }

            for selected_item in &items {
                mapped_results.push(format!("+> {}", full_path_or_name(&**selected_item)));
                if let Some(folder) = selected_item.clone().as_storage_folder() {
                    for inner_item in raise(folder.get_items_async().await) {
                        mapped_results.push(format!("++> {}", full_path_or_name(&*inner_item)));
                    }
                }
            }
            *self.last_selected_item.borrow_mut() = Some(item);
        }

        if let Some(selected_type) = selected_type {
            result_text += NEW_LINE;
            result_text += "Selected type: ";
            result_text += selected_type.name();
        }

        self.opened_file_content().set_text(Some(&result_text));
        let any = !mapped_results.is_empty();
        self.picker_last_results().set_items_source(Some(ItemsSource::from_values(mapped_results)));
        self.picker_last_results_visible().set_is_visible(any);
    }

    /// `ReadTextFromFile(file, length)`: at most `length` characters of the
    /// start of the file, read as UTF-8.
    pub(crate) async fn read_text_from_file(file: Rc<dyn IStorageFile>, length: usize) -> std::io::Result<String> {
        let mut stream = file.open_read_async().await?;

        // 4GB file test, shouldn't load more than 10000 chars into a memory.
        // A character takes at most four bytes.
        let mut buffer = vec![0u8; length * 4];
        let mut bytes_read = 0;
        while bytes_read < buffer.len() {
            let count = stream.read(&mut buffer[bytes_read..])?;
            if count == 0 {
                break;
            }
            bytes_read += count;
        }

        let text = String::from_utf8_lossy(&buffer[..bytes_read]);
        // The reader of the managed original skips the byte order mark.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        Ok(text.chars().take(length).collect())
    }

    /// `CreateSampleWindow()`.
    fn create_sample_window() -> Ref<Window> {
        let window = Window::new();
        window.set_height(200.0);
        window.set_width(200.0);

        let panel = StackPanel::new();
        panel.set_spacing(4.0);

        let text_block = TextBlock::new();
        text_block.set_text(Some("Hello world!"));
        panel.children().add(text_block);

        let button = Button::new();
        button.set_horizontal_alignment(HorizontalAlignment::Center);
        button.set_content(Some(text_content("Click to close")));
        button.set_is_default(true);
        panel.children().add(&button);

        let dialog_button = Button::new();
        dialog_button.set_horizontal_alignment(HorizontalAlignment::Center);
        dialog_button.set_content(Some(text_content("Dialog")));
        dialog_button.set_is_default(false);
        panel.children().add(&dialog_button);

        window.set_content(Some(Control::boxed(panel)));
        window.set_window_startup_location(WindowStartupLocation::CenterOwner);

        // The buttons are descendants of the window: their handlers hold the window weakly.
        {
            let weak = window.downgrade();
            button.click(move |_, _| {
                if let Some(window) = weak.upgrade() {
                    window.close();
                }
            });
        }
        {
            let weak = window.downgrade();
            dialog_button.click(move |_, _| {
                if let Some(window) = weak.upgrade() {
                    let dialog = Self::create_sample_window();
                    dialog.set_height(200.0);
                    drop(dialog.show_dialog(&window));
                }
            });
        }

        window
    }

    /// `GetStorageProvider()`, with the message of the exception of
    /// `GetWindow()` and `GetTopLevel()` as the error.
    fn try_get_storage_provider(&self) -> Result<Rc<dyn IStorageProvider>, &'static str> {
        // Deviation: in the browser the managed storage provider does not exist (it works on
        // the local file system), so "force managed" has no effect there and the storage
        // provider of the top-level is used. Upstream compiles the managed provider for the
        // browser too, where it finds no drives.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let force_managed = self.force_managed().is_checked().unwrap_or(false);
            if force_managed {
                // NOTE: In your production App use 'AppBuilder.UseManagedSystemDialogs()'
                let window: Ref<TopLevel> = self.try_get_window().ok_or(INVALID_OWNER)?.upcast();
                let storage_provider: Rc<dyn IStorageProvider> =
                    Rc::new(ManagedStorageProvider::new(Some(&window), None));
                return Ok(storage_provider);
            }
        }

        let top_level = TopLevel::get_top_level(Some(self)).ok_or(INVALID_OWNER)?;
        Ok(top_level.storage_provider())
    }

    /// `GetStorageProvider()`.
    ///
    /// # Panics
    /// Panics if the page has no owner (the exception of the managed original).
    fn get_storage_provider(&self) -> Rc<dyn IStorageProvider> {
        self.try_get_storage_provider().unwrap_or_else(|message| panic!("{message}"))
    }

    fn try_get_window(&self) -> Option<Ref<Window>> {
        TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.cast::<Window>())
    }

    /// `GetWindow()`.
    ///
    /// # Panics
    /// Panics if the page is not in a window (the exception of the managed original).
    fn get_window(&self) -> Ref<Window> {
        self.try_get_window().unwrap_or_else(|| panic!("{INVALID_OWNER}"))
    }
}
