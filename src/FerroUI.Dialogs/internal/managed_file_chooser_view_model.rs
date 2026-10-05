use super::{
    BclMountedVolumeInfoProvider, FerroDialogsInternalViewModelBase, ManagedFileChooserFilterViewModel,
    ManagedFileChooserItemType, ManagedFileChooserItemViewModel, ManagedFileChooserSources,
};
use crate::ManagedFileDialogOptions;
use ferroui_base::animation::TimeSpan;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::storage::file_io::path;
use ferroui_base::platform::storage::{FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFolder, IStorageItem};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{DateTime, DateTimeOffset, HandlerList, StringComparison};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_controls::platform::IMountedVolumeInfoProvider;
use ferroui_controls::primitives::SelectedItemsList;
use ferroui_controls::SelectionMode;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The state of the managed file chooser: the folder it shows, its
/// entries, the selection, the filters and the quick links.
pub struct ManagedFileChooserViewModel {
    base: FerroDialogsInternalViewModelBase,
    options: ManagedFileDialogOptions,
    cancel_requested: Rc<HandlerList<dyn Fn()>>,
    complete_requested: Rc<HandlerList<dyn Fn(&[String])>>,
    overwrite_prompt: Rc<HandlerList<dyn Fn(&str)>>,

    quick_links: FerroList<Rc<ManagedFileChooserItemViewModel>>,
    items: FerroList<Rc<ManagedFileChooserItemViewModel>>,
    filters: FerroList<Rc<ManagedFileChooserFilterViewModel>>,
    selected_items: SelectedItemsList,

    location: RefCell<Option<String>>,
    file_name: RefCell<Option<String>>,
    show_hidden_files: Cell<bool>,
    selected_filter: RefCell<Option<Rc<ManagedFileChooserFilterViewModel>>>,
    selecting_directory: Cell<bool>,
    saving_file: Cell<bool>,
    scheduled_selection_validation: Cell<bool>,
    already_cancelled: Cell<bool>,
    default_extension: RefCell<Option<String>>,
    overwrite_prompt_enabled: Cell<bool>,
    disposables: CompositeDisposable,

    show_filters: Cell<bool>,
    selection_mode: Cell<SelectionMode>,
    title: RefCell<Option<String>>,
}

impl PartialEq for ManagedFileChooserViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ManagedFileChooserViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

/// The view model of an entry of the selection (the selection holds the
/// untyped items of the list box).
fn item_of(value: &Option<BoxedValue>) -> Option<Rc<ManagedFileChooserItemViewModel>> {
    from_markup_value::<Rc<ManagedFileChooserItemViewModel>>(value)
}

fn boxed(item: &Rc<ManagedFileChooserItemViewModel>) -> Option<BoxedValue> {
    Some(item.clone() as BoxedValue)
}

fn is_null_or_white_space(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

/// `Path.Combine(path1, path2)`: `path2` when it is rooted, else the two
/// joined with a separator.
fn combine(path1: &str, path2: &str) -> String {
    if path1.is_empty() || std::path::Path::new(path2).has_root() || path2.starts_with('/') {
        return path2.to_string();
    }
    if path1.ends_with(path::is_directory_separator) {
        format!("{path1}{path2}")
    } else {
        format!("{path1}{}{path2}", std::path::MAIN_SEPARATOR)
    }
}

/// The current directory of the process (`Directory.GetCurrentDirectory()`).
fn get_current_directory() -> String {
    std::env::current_dir().map(|path| path.to_string_lossy().into_owned()).unwrap_or_default()
}

fn directory_exists(path: Option<&str>) -> bool {
    path.is_some_and(|path| !path.is_empty() && std::path::Path::new(path).is_dir())
}

fn file_exists(path: Option<&str>) -> bool {
    path.is_some_and(|path| !path.is_empty() && std::path::Path::new(path).is_file())
}

/// The local time of the last write of an entry (`LastWriteTime`).
fn last_write_time(metadata: &std::fs::Metadata) -> DateTime {
    const NANOSECONDS_PER_TICK: u128 = 100;

    let Ok(time) = metadata.modified() else {
        return DateTime::default();
    };
    let ticks = match time.duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(after) => (after.as_nanos() / NANOSECONDS_PER_TICK) as i64,
        Err(before) => -((before.duration().as_nanos() / NANOSECONDS_PER_TICK) as i64),
    };
    DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(ticks)).local_date_time()
}

/// Whether an entry is hidden from the listing when hidden files are not
/// shown.
#[cfg(windows)]
fn is_hidden(_name: &str, metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    metadata.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0
}

#[cfg(not(windows))]
fn is_hidden(name: &str, _metadata: &std::fs::Metadata) -> bool {
    name.starts_with('.')
}

/// An entry of a directory: its name, full path, kind and metadata.
struct FileSystemEntry {
    name: String,
    full_name: String,
    is_directory: bool,
    metadata: std::fs::Metadata,
}

/// The entries of a directory (`DirectoryInfo.EnumerateFileSystemInfos()`).
/// A link is an entry of the kind of its target; a link without target is a
/// file. Entries that no longer exist are left out (`Exists`).
fn enumerate_file_system_infos(directory: &str) -> std::io::Result<Vec<FileSystemEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let full_path = entry.path();
        let Ok(metadata) = std::fs::metadata(&full_path).or_else(|_| std::fs::symlink_metadata(&full_path)) else {
            continue;
        };
        entries.push(FileSystemEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            full_name: full_path.to_string_lossy().into_owned(),
            is_directory: metadata.is_dir(),
            metadata,
        });
    }
    Ok(entries)
}

impl ManagedFileChooserViewModel {
    /// The constructor all the others start with.
    pub fn new(options: ManagedFileDialogOptions) -> Rc<Self> {
        let this = Rc::new(Self {
            base: FerroDialogsInternalViewModelBase::new(),
            options,
            cancel_requested: Rc::new(HandlerList::new()),
            complete_requested: Rc::new(HandlerList::new()),
            overwrite_prompt: Rc::new(HandlerList::new()),
            quick_links: FerroList::new(),
            items: FerroList::new(),
            filters: FerroList::new(),
            selected_items: SelectedItemsList::new(),
            location: RefCell::new(None),
            file_name: RefCell::new(None),
            show_hidden_files: Cell::new(false),
            selected_filter: RefCell::new(None),
            selecting_directory: Cell::new(false),
            saving_file: Cell::new(false),
            scheduled_selection_validation: Cell::new(false),
            already_cancelled: Cell::new(false),
            default_extension: RefCell::new(None),
            overwrite_prompt_enabled: Cell::new(false),
            disposables: CompositeDisposable::new(),
            show_filters: Cell::new(false),
            selection_mode: Cell::new(SelectionMode::SINGLE),
            title: RefCell::new(None),
        });

        let quick_sources =
            FerroLocator::current().get_service::<ManagedFileChooserSources>().unwrap_or_else(|| Rc::new(ManagedFileChooserSources::new()));

        let sub1 = FerroLocator::current()
            .get_service::<dyn IMountedVolumeInfoProvider>()
            .unwrap_or_else(|| Rc::new(BclMountedVolumeInfoProvider))
            .listen(ManagedFileChooserSources::mounted_volumes());

        // Holds the view model weakly, as the weak collection change
        // subscription of the original.
        let sub2 = {
            let mounted_volumes = ManagedFileChooserSources::mounted_volumes();
            let weak = Rc::downgrade(&this);
            let quick_sources = quick_sources.clone();
            let token = mounted_volumes.add_collection_changed(Rc::new(move |_| {
                let weak = weak.clone();
                let quick_sources = quick_sources.clone();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.refresh_quick_links(&quick_sources);
                        }
                    },
                    DispatcherPriority::DEFAULT,
                );
            }));
            Disposable::create(move || {
                mounted_volumes.remove_collection_changed(token);
            })
        };

        this.disposables.add(sub1);
        this.disposables.add(sub2);

        let weak = Rc::downgrade(&this);
        this.complete_requested.add(Rc::new(move |_: &[String]| {
            if let Some(this) = weak.upgrade() {
                this.disposables.dispose();
            }
        }));
        let weak = Rc::downgrade(&this);
        this.cancel_requested.add(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.disposables.dispose();
            }
        }));

        this.refresh_quick_links(&quick_sources);

        let weak = Rc::downgrade(&this);
        this.selected_items.add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed_async();
            }
        }));

        this
    }

    /// The chooser of an open file picker.
    pub fn new_open_file(file_picker_open: &FilePickerOpenOptions, options: ManagedFileDialogOptions) -> Rc<Self> {
        let this = Self::new(options);
        *this.title.borrow_mut() = Some(file_picker_open.title().unwrap_or("Open file").to_string());

        if let Some(file_type_filter) = file_picker_open.file_type_filter().filter(|filter| !filter.is_empty()) {
            this.filters.add_range(
                file_type_filter
                    .iter()
                    .enumerate()
                    .map(|(i, f)| ManagedFileChooserFilterViewModel::new_with_index(f, i as i32)),
            );
            *this.selected_filter.borrow_mut() = Some(this.filters.get(0));
            this.show_filters.set(true);
        }

        if file_picker_open.allow_multiple() {
            this.selection_mode.set(SelectionMode::MULTIPLE);
        }

        this.navigate_folder(file_picker_open.suggested_start_location(), None);
        this
    }

    /// The chooser of a save file picker.
    pub fn new_save_file(file_picker_save: &FilePickerSaveOptions, options: ManagedFileDialogOptions) -> Rc<Self> {
        let this = Self::new(options);
        *this.title.borrow_mut() = Some(file_picker_save.title().unwrap_or("Save file").to_string());

        if let Some(file_type_choices) = file_picker_save.file_type_choices().filter(|choices| !choices.is_empty()) {
            this.filters.add_range(
                file_type_choices
                    .iter()
                    .enumerate()
                    .map(|(i, f)| ManagedFileChooserFilterViewModel::new_with_index(f, i as i32)),
            );
            *this.selected_filter.borrow_mut() = Some(this.filters.get(0));
            this.show_filters.set(true);
        }

        this.saving_file.set(true);
        *this.default_extension.borrow_mut() = file_picker_save.default_extension().map(str::to_string);
        this.overwrite_prompt_enabled.set(file_picker_save.show_overwrite_prompt().unwrap_or(true));
        this.set_file_name(file_picker_save.suggested_file_name().map(str::to_string));

        let file_name = this.file_name();
        this.navigate_folder(file_picker_save.suggested_start_location(), file_name.as_deref());
        this
    }

    /// The chooser of a folder picker.
    pub fn new_open_folder(folder_picker_open: &FolderPickerOpenOptions, options: ManagedFileDialogOptions) -> Rc<Self> {
        let this = Self::new(options);
        *this.title.borrow_mut() = Some(folder_picker_open.title().unwrap_or("Select directory").to_string());

        this.selecting_directory.set(true);

        if folder_picker_open.allow_multiple() {
            this.selection_mode.set(SelectionMode::MULTIPLE);
        }

        this.navigate_folder(folder_picker_open.suggested_start_location(), None);
        this
    }

    // --- events ---

    /// Raised when the chooser is canceled.
    pub fn cancel_requested(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.cancel_requested.add(Rc::new(handler));
        let handlers = self.cancel_requested.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    /// Raised with the chosen paths when the chooser completes.
    pub fn complete_requested(&self, handler: impl Fn(&[String]) + 'static) -> Rc<dyn IDisposable> {
        let token = self.complete_requested.add(Rc::new(handler));
        let handlers = self.complete_requested.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    /// Raised with the full path of an existing file the user chose to
    /// save to, when overwriting needs a confirmation.
    pub fn overwrite_prompt(&self, handler: impl Fn(&str) + 'static) -> Rc<dyn IDisposable> {
        let token = self.overwrite_prompt.add(Rc::new(handler));
        let handlers = self.overwrite_prompt.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn raise_cancel_requested(&self) {
        for (_, handler) in self.cancel_requested.snapshot().iter() {
            handler();
        }
    }

    fn raise_complete_requested(&self, items: &[String]) {
        for (_, handler) in self.complete_requested.snapshot().iter() {
            handler(items);
        }
    }

    fn raise_overwrite_prompt(&self, file_name: &str) {
        for (_, handler) in self.overwrite_prompt.snapshot().iter() {
            handler(file_name);
        }
    }

    // --- properties ---

    pub fn quick_links(&self) -> FerroList<Rc<ManagedFileChooserItemViewModel>> {
        self.quick_links.clone()
    }

    pub fn items(&self) -> FerroList<Rc<ManagedFileChooserItemViewModel>> {
        self.items.clone()
    }

    pub fn filters(&self) -> FerroList<Rc<ManagedFileChooserFilterViewModel>> {
        self.filters.clone()
    }

    /// The selected entries, as the untyped list the list box of the
    /// chooser fills: each item is a [`ManagedFileChooserItemViewModel`].
    pub fn selected_items(&self) -> SelectedItemsList {
        self.selected_items.clone()
    }

    /// The selected entries as view models.
    pub fn selected_item_view_models(&self) -> Vec<Rc<ManagedFileChooserItemViewModel>> {
        self.selected_items.to_vec().iter().filter_map(item_of).collect()
    }

    pub fn location(&self) -> Option<String> {
        self.location.borrow().clone()
    }

    pub fn set_location(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.location, value, "Location");
    }

    pub fn file_name(&self) -> Option<String> {
        self.file_name.borrow().clone()
    }

    pub fn set_file_name(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.file_name, value, "FileName");
    }

    pub fn selecting_folder(&self) -> bool {
        self.selecting_directory.get()
    }

    pub fn show_filters(&self) -> bool {
        self.show_filters.get()
    }

    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode.get()
    }

    pub fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    /// The index of the quick link of the current location, or -1.
    pub fn quick_links_selected_index(&self) -> i32 {
        let location = self.location();
        for (index, i) in self.quick_links.snapshot().iter().enumerate() {
            if i.path() == location {
                return index as i32;
            }
        }

        -1
    }

    /// Setting the index only notifies a change: the index follows the
    /// location.
    pub fn set_quick_links_selected_index(&self, _value: i32) {
        self.base.raise_property_changed("QuickLinksSelectedIndex");
    }

    pub fn selected_filter(&self) -> Option<Rc<ManagedFileChooserFilterViewModel>> {
        self.selected_filter.borrow().clone()
    }

    pub fn set_selected_filter(&self, value: Option<Rc<ManagedFileChooserFilterViewModel>>) {
        self.base.raise_and_set_if_changed(&self.selected_filter, value, "SelectedFilter");
        self.refresh();
    }

    pub fn show_hidden_files(&self) -> bool {
        self.show_hidden_files.get()
    }

    pub fn set_show_hidden_files(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.show_hidden_files, value, "ShowHiddenFiles");
        self.refresh();
    }

    fn refresh_quick_links(&self, quick_sources: &ManagedFileChooserSources) {
        self.quick_links.clear();
        self.quick_links.add_range(
            quick_sources.get_all_items().iter().map(ManagedFileChooserItemViewModel::from_navigation_item),
        );
    }

    // --- operations ---

    pub fn enter_pressed(&self) {
        let location = self.location();
        if directory_exists(location.as_deref()) {
            self.navigate(location.as_deref(), None);
        } else if file_exists(location.as_deref()) {
            self.raise_complete_requested(&[location.unwrap_or_default()]);
        }
    }

    /// Validates the selection once the list box has finished changing it,
    /// from a dispatcher job: folders are removed when they cannot be
    /// chosen, and the file name shows the selected entries.
    fn on_selection_changed_async(self: &Rc<Self>) {
        if self.scheduled_selection_validation.get() {
            return;
        }

        self.scheduled_selection_validation.set(true);
        let weak: Weak<Self> = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(this) = weak.upgrade() else { return };

                struct Reset<'a>(&'a Cell<bool>);
                impl Drop for Reset<'_> {
                    fn drop(&mut self) {
                        self.0.set(false);
                    }
                }
                let _reset = Reset(&this.scheduled_selection_validation);

                if this.selecting_directory.get() {
                    this.selected_items.clear();
                } else {
                    if !this.options.allow_directory_selection() {
                        let invalid_items: Vec<Option<BoxedValue>> = this
                            .selected_items
                            .to_vec()
                            .into_iter()
                            .filter(|i| item_of(i).is_some_and(|i| i.item_type() == ManagedFileChooserItemType::Folder))
                            .collect();
                        for item in invalid_items {
                            this.selected_items.remove(&item);
                        }
                    }

                    if !this.selecting_directory.get() {
                        let selected = this.selected_item_view_models();
                        if selected.len() > 1 {
                            let mut sb = String::new();

                            for (i, item) in selected.iter().enumerate() {
                                sb.push('"');
                                sb.push_str(item.display_name().as_deref().unwrap_or_default());
                                sb.push('"');
                                if i + 1 < selected.len() {
                                    sb.push(' ');
                                }
                            }

                            this.set_file_name(Some(sb));
                        } else {
                            this.set_file_name(selected.first().and_then(|item| item.display_name()));
                        }
                    }
                }
                this.base.raise_property_changed("SelectedItems");
            },
            DispatcherPriority::DEFAULT,
        );
    }

    fn navigate_root(&self, initial_selection_name: Option<&str>) {
        if cfg!(windows) {
            // The root of the system folder.
            let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
            let root = std::path::Path::new(&system)
                .ancestors()
                .last()
                .map(|root| root.to_string_lossy().into_owned())
                .unwrap_or(system);
            self.navigate(Some(&root), initial_selection_name);
        } else {
            self.navigate(Some("/"), initial_selection_name);
        }
    }

    pub fn refresh(&self) {
        let location = self.location();
        self.navigate(location.as_deref(), None);
    }

    /// Navigates to a folder, or to the current directory without one.
    pub fn navigate_folder(&self, path: Option<&Rc<dyn IStorageFolder>>, initial_selection_name: Option<&str>) {
        let full_directory_path = path
            .and_then(|path| {
                let item: &dyn IStorageItem = &**path;
                item.try_get_local_path()
            })
            .unwrap_or_else(get_current_directory);

        self.navigate(Some(&full_directory_path), initial_selection_name);
    }

    /// Navigates to the directory `path`, or to the root of the file system
    /// when it does not exist, and selects the file named
    /// `initial_selection_name` there.
    pub fn navigate(&self, path: Option<&str>, initial_selection_name: Option<&str>) {
        if !directory_exists(path) {
            self.navigate_root(initial_selection_name);
        } else {
            let path = path.unwrap_or_default();
            self.set_location(Some(path.to_string()));
            self.items.clear();
            self.selected_items.clear();

            // The original ignores a denied access; any failure to read the directory leaves it empty.
            let Ok(mut infos) = enumerate_file_system_infos(path) else {
                return;
            };

            if !self.show_hidden_files() {
                infos.retain(|i| !is_hidden(&i.name, &i.metadata));
            }

            if let Some(selected_filter) = self.selected_filter() {
                infos.retain(|i| i.is_directory || selected_filter.is_match(&i.name));
            }

            let selecting_directory = self.selecting_directory.get();
            let mut items: Vec<Rc<ManagedFileChooserItemViewModel>> = infos
                .into_iter()
                .filter(|x| !selecting_directory || x.is_directory)
                .map(|info| {
                    let item = ManagedFileChooserItemViewModel::new();
                    item.set_display_name(Some(info.name.clone()));
                    item.set_path(Some(info.full_name.clone()));
                    item.set_type(Some(if info.is_directory {
                        "File Folder".to_string()
                    } else {
                        path::get_extension(&info.name).to_string()
                    }));
                    item.set_item_type(if info.is_directory {
                        ManagedFileChooserItemType::Folder
                    } else {
                        ManagedFileChooserItemType::File
                    });
                    item.set_size(if info.is_directory { 0 } else { info.metadata.len() as i64 });
                    item.set_modified(last_write_time(&info.metadata));
                    item
                })
                .collect();
            // Folders first, then by name; the sort is stable.
            items.sort_by(|a, b| {
                let a_folder = a.item_type() == ManagedFileChooserItemType::Folder;
                let b_folder = b.item_type() == ManagedFileChooserItemType::Folder;
                b_folder.cmp(&a_folder).then_with(|| {
                    StringComparison::InvariantCultureIgnoreCase.compare(
                        a.display_name().as_deref().unwrap_or_default(),
                        b.display_name().as_deref().unwrap_or_default(),
                    )
                })
            });
            self.items.add_range(items);

            if let Some(initial_selection_name) = initial_selection_name {
                let sel = self.items.snapshot().iter().find(|i| {
                    i.item_type() == ManagedFileChooserItemType::File
                        && i.display_name().as_deref() == Some(initial_selection_name)
                }).cloned();

                if let Some(sel) = sel {
                    self.selected_items.add(boxed(&sel));
                }
            }

            self.base.raise_property_changed("QuickLinksSelectedIndex");
        }
    }

    pub fn go_up(&self) {
        let location = self.location();
        let parent = location.as_deref().and_then(path::get_directory_name);

        if is_null_or_white_space(parent) {
            return;
        }

        self.navigate(parent, None);
    }

    pub fn cancel(&self) {
        if !self.already_cancelled.get() {
            // INFO: Don't misplace this check or it might cause
            //       a stack overflow because of recursive
            //       event invokes.
            self.already_cancelled.set(true);
            self.raise_cancel_requested();
        }
    }

    /// Whether [`ok`](Self::ok) can complete the chooser.
    pub fn can_ok(&self, _parameter: Option<BoxedValue>) -> bool {
        (self.selecting_directory.get() && !is_null_or_white_space(self.location().as_deref()))
            || (self.saving_file.get() && !is_null_or_white_space(self.file_name().as_deref()))
            || (!self.selected_items.is_empty()
                && self.selected_item_view_models().iter().any(|s| !is_null_or_white_space(s.path().as_deref())))
    }

    pub fn ok(&self) {
        if self.selecting_directory.get() {
            self.raise_complete_requested(&[self.location().unwrap_or_default()]);
        } else if self.saving_file.get() {
            if let Some(mut file_name) = self.file_name().filter(|name| !name.trim().is_empty()) {
                let default_extension = self.default_extension.borrow().clone();
                if !path::has_extension(&file_name) && !is_null_or_white_space(default_extension.as_deref()) {
                    file_name = path::change_extension(&file_name, default_extension.as_deref());
                    self.set_file_name(Some(file_name.clone()));
                }

                let full_name = combine(&self.location().unwrap_or_default(), &file_name);

                if self.overwrite_prompt_enabled.get() && file_exists(Some(&full_name)) {
                    self.raise_overwrite_prompt(&full_name);
                } else {
                    self.raise_complete_requested(&[full_name]);
                }
            }
        } else {
            let paths: Vec<String> =
                self.selected_item_view_models().iter().map(|i| i.path().unwrap_or_default()).collect();
            self.raise_complete_requested(&paths);
        }
    }

    pub fn select_single_file(&self, item: &ManagedFileChooserItemViewModel) {
        self.raise_complete_requested(&[item.path().unwrap_or_default()]);
    }
}

ferro_markup_type!(class ManagedFileChooserViewModel {
    this: Rc<ManagedFileChooserViewModel>,
    handles: [ManagedFileChooserViewModel, Rc<ManagedFileChooserViewModel>, Option<Rc<ManagedFileChooserViewModel>>],
    properties: [
        QuickLinks: FerroList<Rc<ManagedFileChooserItemViewModel>> { get: ManagedFileChooserViewModel::quick_links },
        Items: FerroList<Rc<ManagedFileChooserItemViewModel>> { get: ManagedFileChooserViewModel::items },
        Filters: FerroList<Rc<ManagedFileChooserFilterViewModel>> { get: ManagedFileChooserViewModel::filters },
        SelectedItems: SelectedItemsList { get: ManagedFileChooserViewModel::selected_items },
        Location: Option<String> { get: ManagedFileChooserViewModel::location, set: ManagedFileChooserViewModel::set_location },
        FileName: Option<String> { get: ManagedFileChooserViewModel::file_name, set: ManagedFileChooserViewModel::set_file_name },
        SelectingFolder: bool { get: ManagedFileChooserViewModel::selecting_folder },
        ShowFilters: bool { get: ManagedFileChooserViewModel::show_filters },
        SelectionMode: SelectionMode { get: ManagedFileChooserViewModel::selection_mode },
        Title: Option<String> { get: ManagedFileChooserViewModel::title },
        QuickLinksSelectedIndex: i32 {
            get: ManagedFileChooserViewModel::quick_links_selected_index,
            set: ManagedFileChooserViewModel::set_quick_links_selected_index
        },
        SelectedFilter: Option<Rc<ManagedFileChooserFilterViewModel>> {
            get: ManagedFileChooserViewModel::selected_filter,
            set: ManagedFileChooserViewModel::set_selected_filter
        },
        ShowHiddenFiles: bool {
            get: ManagedFileChooserViewModel::show_hidden_files,
            set: ManagedFileChooserViewModel::set_show_hidden_files
        },
    ],
    methods: [
        fn EnterPressed() => ManagedFileChooserViewModel::enter_pressed,
        fn Refresh() => ManagedFileChooserViewModel::refresh,
        fn GoUp() => ManagedFileChooserViewModel::go_up,
        fn Cancel() => ManagedFileChooserViewModel::cancel,
        fn CanOk(Option<BoxedValue>) -> bool => ManagedFileChooserViewModel::can_ok
            [DependsOn("FileName"), DependsOn("Location"), DependsOn("SelectedItems")],
        fn Ok() => ManagedFileChooserViewModel::ok,
        fn SelectSingleFile(Rc<ManagedFileChooserItemViewModel>) =>
            (|this: &Rc<ManagedFileChooserViewModel>, item: Rc<ManagedFileChooserItemViewModel>| this.select_single_file(&item)),
    ],
    notify_property_changed: ManagedFileChooserViewModel,
});
