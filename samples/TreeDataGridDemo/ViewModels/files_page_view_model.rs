//! Port of `ViewModels/FilesPageViewModel.cs`.

use crate::models::{FileTreeNodeComparison, FileTreeNodeModel};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingBase, BindingError, ReflectionBinding};
use ferroui_base::ferro_markup_type;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::IImage;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::{CultureInfo, StringComparison, Uri};
use ferroui_base::BoxedValue;
use ferroui_controls::{GridLength, GridUnitType, ItemsSource};
use ferroui_controls_treedatagrid::{
    HierarchicalTreeDataGridSourceExtensions, ObjectComparison, TreeDataGridSourceExtensions, TreeDataGridSourceRef,
    TreeDataGridTemplateColumn,
};
use ferroui_treedatagrid_core::models::Comparison;
use ferroui_treedatagrid_core::selection::{
    downcast_selection, ITreeDataGridSelection, TreeDataGridCellSelectionModel, TreeDataGridRowSelectionModel,
    TreeSelectionModelSelectionChangedEventArgsOf,
};
use ferroui_treedatagrid_core::{
    Accessor, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, ITreeDataGridSourceOf, IndexPath,
    Models,
};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, OnceCell, RefCell};
use std::path::{Component, Path};
use std::rc::{Rc, Weak};

ferroui_base::ferro_thread_local! {
    // `s_iconConverter`: created on first use, on the UI thread.
    static ICON_CONVERTER: OnceCell<Rc<IconConverter>> = const { OnceCell::new() };
}

/// The files page: the file system as a tree (or the entries of the selected drive as a flat
/// list), with check boxes, an editable name with an icon, the size and the modification time of
/// the files, the path of the selected entry and a choice of row or cell selection.
pub struct FilesPageViewModel {
    base: ViewModelBase,
    weak_self: Weak<FilesPageViewModel>,
    tree_source: Rc<HierarchicalTreeDataGridSource<FileTreeNodeModel>>,
    flat_source: RefCell<Option<Rc<FlatTreeDataGridSource<FileTreeNodeModel>>>>,
    // Whether the source is the flat source (`_source` of the reference, which is one of the two).
    source_is_flat: Cell<bool>,
    cell_selection: Cell<bool>,
    root: RefCell<Option<Rc<FileTreeNodeModel>>>,
    drives: Vec<String>,
    drive_items: ItemsSource,
    selected_drive: RefCell<String>,
    selected_path: RefCell<Option<String>>,
}

impl PartialEq for FilesPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for FilesPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl FilesPageViewModel {
    pub fn new() -> Rc<Self> {
        // `DriveInfo.GetDrives().Select(x => x.Name)`.
        let drives = get_drives();

        let selected_drive =
            if cfg!(windows) { "C:\\".to_string() } else { drives.first().cloned().unwrap_or_else(|| "/".to_string()) };

        let drive_items = ItemsSource::from_values(drives.iter().cloned());

        let this = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: ViewModelBase::new(),
            weak_self: weak.clone(),
            tree_source: Self::create_tree_source(weak.clone()),
            flat_source: RefCell::new(None),
            source_is_flat: Cell::new(false),
            cell_selection: Cell::new(false),
            root: RefCell::new(None),
            drives,
            drive_items,
            selected_drive: RefCell::new(selected_drive),
            selected_path: RefCell::new(None),
        });

        // `WhenAnyValue(x => x.SelectedDrive)` emits the current drive when it is subscribed.
        this.on_selected_drive_changed();
        this
    }

    pub fn cell_selection(&self) -> bool {
        self.cell_selection.get()
    }

    pub fn set_cell_selection(&self, value: bool) {
        if self.cell_selection.get() != value {
            self.cell_selection.set(value);
            let source = self.source();
            let selection: Rc<dyn ITreeDataGridSelection> = if value {
                let selection = TreeDataGridCellSelectionModel::new(&source);
                selection.set_single_select(false);
                selection
            } else {
                let selection = TreeDataGridRowSelectionModel::new(&source);
                selection.base().set_single_select(false);
                selection
            };
            source.set_selection(Some(selection));
            self.base.raise_property_changed("CellSelection");
        }
    }

    pub fn drives(&self) -> &[String] {
        &self.drives
    }

    pub fn flat_list(&self) -> bool {
        self.source_is_flat.get()
    }

    pub fn set_flat_list(&self, value: bool) {
        if value != self.flat_list() {
            if value && self.flat_source.borrow().is_none() {
                let flat_source = self.create_flat_source();
                *self.flat_source.borrow_mut() = Some(flat_source);
            }
            self.source_is_flat.set(value);
            self.base.raise_property_changed("Source");
        }
    }

    pub fn selected_drive(&self) -> String {
        self.selected_drive.borrow().clone()
    }

    pub fn set_selected_drive(&self, value: String) {
        if *self.selected_drive.borrow() != value {
            *self.selected_drive.borrow_mut() = value;
            // The subscription of the constructor to `SelectedDrive`, which is the first to hear
            // of the change.
            self.on_selected_drive_changed();
            self.base.raise_property_changed("SelectedDrive");
        }
    }

    pub fn selected_path(&self) -> Option<String> {
        self.selected_path.borrow().clone()
    }

    pub fn set_selected_path(&self, value: Option<String>) {
        self.set_selected_path_core(value);
    }

    /// The source the grid shows: the hierarchical source, or the flat source of the entries of
    /// the drive.
    pub fn source(&self) -> Rc<dyn ITreeDataGridSourceOf<FileTreeNodeModel>> {
        if self.source_is_flat.get() {
            if let Some(flat_source) = self.flat_source.borrow().clone() {
                return flat_source as Rc<dyn ITreeDataGridSourceOf<FileTreeNodeModel>>;
            }
        }
        self.tree_source.clone() as Rc<dyn ITreeDataGridSourceOf<FileTreeNodeModel>>
    }

    /// The source as the grid's `Source`.
    pub fn source_ref(&self) -> TreeDataGridSourceRef {
        if self.source_is_flat.get() {
            if let Some(flat_source) = self.flat_source.borrow().clone() {
                return TreeDataGridSourceRef::of::<FileTreeNodeModel, _>(flat_source);
            }
        }
        TreeDataGridSourceRef::of::<FileTreeNodeModel, _>(self.tree_source.clone())
    }

    /// `FileIconConverter`: the icon of a node from whether it is a directory and whether it is
    /// expanded, with the bitmaps of the assets of the demo, loaded on first use.
    ///
    /// # Panics
    /// When an icon cannot be opened or decoded (an exception of the reference's getter).
    pub fn file_icon_converter() -> Rc<dyn IMultiValueConverter> {
        ICON_CONVERTER.with(|converter| {
            converter
                .get_or_init(|| {
                    let file_icon = load_bitmap("ferres://TreeDataGridDemo/Assets/file.png");
                    let folder_icon = load_bitmap("ferres://TreeDataGridDemo/Assets/folder.png");
                    let folder_open_icon = load_bitmap("ferres://TreeDataGridDemo/Assets/folder-open.png");

                    Rc::new(IconConverter::new(file_icon, folder_open_icon, folder_icon))
                })
                .clone()
        }) as Rc<dyn IMultiValueConverter>
    }

    fn on_selected_drive_changed(&self) {
        let root = FileTreeNodeModel::new(self.selected_drive(), true, true);
        *self.root.borrow_mut() = Some(root.clone());

        // As the reference: the hierarchical source always exists, so it is the one that gets
        // the new root, and the flat source keeps the children of the root it was created with.
        self.tree_source.set_items(Rc::new(vec![root]) as Models<FileTreeNodeModel>);
    }

    fn create_flat_source(&self) -> Rc<FlatTreeDataGridSource<FileTreeNodeModel>> {
        let root = self.root.borrow().clone().expect("the root of the drive");
        let result = FlatTreeDataGridSource::new(root.children() as Models<FileTreeNodeModel>)
            .with_check_box_column(None, is_checked(), |o| {
                o.can_user_resize = Some(false);
            })
            .with_template_column_from_resource_keys(
                header("Name"),
                ResourceKey::from("FileNameCell"),
                Some(ResourceKey::from("FileNameEditCell")),
                |o| {
                    o.width = GridLength::new(1.0, GridUnitType::Star);
                    o.compare_ascending = Some(compare_typed(FileTreeNodeModel::sort_ascending(|x| x.name())));
                    o.compare_descending = Some(compare_typed(FileTreeNodeModel::sort_descending(|x| x.name())));
                    // `CompiledBinding.Create((FileTreeNodeModel x) => x.Name)`: the binding reads
                    // the property through the markup metadata of the model.
                    o.text_search_binding = Some(ReflectionBinding::new("Name") as Rc<dyn BindingBase>);
                },
            )
            .with_text_column(None, Accessor::member("Size", |x: &Rc<FileTreeNodeModel>| x.size()), |o| {
                o.compare_ascending = Some(compare_typed(FileTreeNodeModel::sort_ascending(|x| x.size())));
                o.compare_descending = Some(compare_typed(FileTreeNodeModel::sort_descending(|x| x.size())));
            })
            .with_text_column(None, Accessor::member("Modified", |x: &Rc<FileTreeNodeModel>| x.modified()), |o| {
                o.compare_ascending = Some(compare_typed(FileTreeNodeModel::sort_ascending(|x| x.modified())));
                o.compare_descending = Some(compare_typed(FileTreeNodeModel::sort_descending(|x| x.modified())));
            });

        let row_selection = result.row_selection().expect("the row selection of a new source");
        row_selection.base().set_single_select(false);
        row_selection.add_selection_changed(Self::selection_changed_handler(self.weak_self.clone()));
        result
    }

    fn create_tree_source(weak_self: Weak<Self>) -> Rc<HierarchicalTreeDataGridSource<FileTreeNodeModel>> {
        let name_column = TreeDataGridTemplateColumn::with_resource_keys(
            None,
            ResourceKey::from("FileNameCell"),
            Some(ResourceKey::from("FileNameEditCell")),
        );
        name_column.set_compare_ascending(Some(compare(FileTreeNodeModel::sort_ascending(|x| x.name()))));
        name_column.set_compare_descending(Some(compare(FileTreeNodeModel::sort_descending(|x| x.name()))));
        name_column.set_text_search_binding(Some(ReflectionBinding::new("Name") as Rc<dyn BindingBase>));

        let result =
            HierarchicalTreeDataGridSource::new(Rc::new(Vec::<Rc<FileTreeNodeModel>>::new()) as Models<FileTreeNodeModel>)
                .with_check_box_column(None, is_checked(), |o| {
                    o.can_user_resize = Some(false);
                })
                .with_hierarchical_expander_column(
                    header("Name"),
                    &name_column,
                    // A file has no children (the reference's selector throws for a file, which its
                    // source never asks for children: a file has none).
                    |x: &Rc<FileTreeNodeModel>| {
                        if x.is_directory() {
                            Some(x.children() as Models<FileTreeNodeModel>)
                        } else {
                            None
                        }
                    },
                    |o| {
                        o.width = GridLength::new(1.0, GridUnitType::Star);
                        o.has_children = Some(Accessor::member("HasChildren", |x: &Rc<FileTreeNodeModel>| x.has_children()));
                        // `o.IsExpanded = x => x.IsExpanded`, with the setter the reference
                        // synthesizes from the expression.
                        o.is_expanded = Some(
                            Accessor::member("IsExpanded", |x: &Rc<FileTreeNodeModel>| x.is_expanded())
                                .with_setter(|x: &Rc<FileTreeNodeModel>, value: bool| x.set_is_expanded(value)),
                        );
                    },
                )
                .with_text_column(None, Accessor::member("Size", |x: &Rc<FileTreeNodeModel>| x.size()), |o| {
                    o.compare_ascending = Some(compare_typed(FileTreeNodeModel::sort_ascending(|x| x.size())));
                    o.compare_descending = Some(compare_typed(FileTreeNodeModel::sort_descending(|x| x.size())));
                })
                .with_text_column(None, Accessor::member("Modified", |x: &Rc<FileTreeNodeModel>| x.modified()), |o| {
                    o.compare_ascending = Some(compare_typed(FileTreeNodeModel::sort_ascending(|x| x.modified())));
                    o.compare_descending = Some(compare_typed(FileTreeNodeModel::sort_descending(|x| x.modified())));
                });

        let row_selection = result.row_selection().expect("the row selection of a new source");
        row_selection.base().set_single_select(false);
        row_selection.add_selection_changed(Self::selection_changed_handler(weak_self));
        result
    }

    /// `SetSelectedPath`: selects the row of the entry `value`, expanding the directories on the
    /// way to it, and selects the drive of the entry; an empty path clears the selection.
    fn set_selected_path_core(&self, value: Option<String>) {
        let Some(path) = value.filter(|value| !value.is_empty()) else {
            Self::get_row_selection(&self.source()).base().clear();
            return;
        };

        // The names from the root of the drive down to the entry (the stack of the reference,
        // popped from the drive down).
        let path = Path::new(&path);
        let components = if path.is_file() || path.is_dir() { full_path_components(path) } else { Vec::new() };

        let mut index = IndexPath::UNSELECTED;

        if let Some((drive, components)) = components.split_first() {
            let drive_index = self
                .drives
                .iter()
                .position(|x| StringComparison::OrdinalIgnoreCase.equals(Some(x.as_str()), Some(drive.as_str())));

            if let Some(drive_index) = drive_index {
                self.set_selected_drive(self.drives[drive_index].clone());
            }

            let mut node = self.root.borrow().clone();
            index = IndexPath::new(0);

            for component in components {
                let Some(current) = node else {
                    break;
                };
                current.set_is_expanded(true);

                let children = current.children();
                let i = children
                    .to_vec()
                    .iter()
                    .position(|x| StringComparison::OrdinalIgnoreCase.equals(Some(x.name().as_str()), Some(component.as_str())));
                node = i.map(|i| children.get(i));
                index = match i {
                    Some(i) => index.append(i as i32),
                    None => IndexPath::UNSELECTED,
                };
            }
        }

        Self::get_row_selection(&self.source()).base().set_selected_index(index);
    }

    /// The row selection of `source`.
    ///
    /// # Panics
    /// When the selection of the source is not a row selection (a cell selection;
    /// `InvalidOperationException`).
    fn get_row_selection(
        source: &Rc<dyn ITreeDataGridSourceOf<FileTreeNodeModel>>,
    ) -> Rc<TreeDataGridRowSelectionModel<FileTreeNodeModel>> {
        source
            .selection()
            .and_then(|selection| downcast_selection::<TreeDataGridRowSelectionModel<FileTreeNodeModel>>(&selection))
            .unwrap_or_else(|| panic!("Expected a row selection model."))
    }

    fn selection_changed_handler(
        weak_self: Weak<Self>,
    ) -> Rc<dyn Fn(&TreeSelectionModelSelectionChangedEventArgsOf<FileTreeNodeModel>)> {
        Rc::new(move |e: &TreeSelectionModelSelectionChangedEventArgsOf<FileTreeNodeModel>| {
            if let Some(this) = weak_self.upgrade() {
                this.selection_changed(e);
            }
        })
    }

    fn selection_changed(&self, e: &TreeSelectionModelSelectionChangedEventArgsOf<FileTreeNodeModel>) {
        let selected_path = Self::get_row_selection(&self.source()).base().selected_item().map(|x| x.path());
        self.base.raise_and_set_if_changed(&self.selected_path, selected_path, "SelectedPath");

        // `System.Diagnostics.Trace.WriteLine`: in builds with debug assertions.
        if cfg!(debug_assertions) {
            for i in e.deselected_items().iter() {
                eprintln!("Deselected '{}'", i.map(|i| i.path()).unwrap_or_default());
            }
            for i in e.selected_items().iter() {
                eprintln!("Selected '{}'", i.map(|i| i.path()).unwrap_or_default());
            }
        }
    }
}

/// `x => x.IsChecked`, with the setter the reference synthesizes from the expression.
fn is_checked() -> Accessor<FileTreeNodeModel, bool> {
    Accessor::member("IsChecked", |x: &Rc<FileTreeNodeModel>| x.is_checked())
        .with_setter(|x: &Rc<FileTreeNodeModel>, value: bool| x.set_is_checked(value))
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// `Compare(comparison)`: the comparison of the nodes as a comparison of untyped values.
fn compare(comparison: FileTreeNodeComparison) -> ObjectComparison {
    Rc::new(move |x: Option<&BoxedValue>, y: Option<&BoxedValue>| {
        comparison(
            x.and_then(|x| x.downcast_ref::<FileTreeNodeModel>()),
            y.and_then(|y| y.downcast_ref::<FileTreeNodeModel>()),
        )
    })
}

/// The comparison of the nodes as the comparison of the models of a column created by the
/// helpers (which the reference passes through `Compare` as well).
fn compare_typed(comparison: FileTreeNodeComparison) -> Comparison<FileTreeNodeModel> {
    Rc::new(move |x: &Rc<FileTreeNodeModel>, y: &Rc<FileTreeNodeModel>| comparison(Some(&**x), Some(&**y)))
}

/// Loads an icon of the assets of the demo (`new Bitmap(AssetLoader.Open(uri))`).
fn load_bitmap(uri: &str) -> Rc<dyn IImage> {
    let uri = Uri::absolute(uri).unwrap_or_else(|e| panic!("{e}"));
    let mut stream = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
    let bitmap = Bitmap::from_stream(&mut stream).unwrap_or_else(|e| panic!("{e}"));
    Rc::new(bitmap) as Rc<dyn IImage>
}

/// The root of the path and the names below it, from the full path of `path`
/// (`Path.GetFullPath`, then `DirectoryInfo.Name` of each directory up to the root): the root is
/// `/` on Unix and the drive (`C:\`) on Windows, as the names of the drives.
fn full_path_components(path: &Path) -> Vec<String> {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut root = String::new();
    let mut names: Vec<String> = Vec::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => root.push_str(&prefix.as_os_str().to_string_lossy()),
            Component::RootDir => root.push(std::path::MAIN_SEPARATOR),
            Component::CurDir => {}
            Component::ParentDir => {
                names.pop();
            }
            Component::Normal(name) => names.push(name.to_string_lossy().into_owned()),
        }
    }
    let mut components = Vec::with_capacity(names.len() + 1);
    components.push(root);
    components.extend(names);
    components
}

/// `DriveInfo.GetDrives().Select(x => x.Name)` on Windows: the root of each drive letter that
/// exists.
#[cfg(windows)]
fn get_drives() -> Vec<String> {
    (b'A'..=b'Z').map(|letter| format!("{}:\\", letter as char)).filter(|root| Path::new(root).exists()).collect()
}

/// `DriveInfo.GetDrives().Select(x => x.Name)` on Linux: the mount points of the mounted file
/// systems, in the order of the mount table.
#[cfg(target_os = "linux")]
fn get_drives() -> Vec<String> {
    let mut drives: Vec<String> = Vec::new();
    if let Ok(mounts) = std::fs::read_to_string("/proc/self/mounts") {
        for line in mounts.lines() {
            if let Some(mount_point) = line.split_whitespace().nth(1) {
                let mount_point = unescape_mount_point(mount_point);
                if !drives.contains(&mount_point) {
                    drives.push(mount_point);
                }
            }
        }
    }
    if drives.is_empty() {
        drives.push("/".to_string());
    }
    drives
}

/// A mount point of the mount table, whose spaces, tabs, line breaks and backslashes are octal
/// escapes (`\040`).
#[cfg(target_os = "linux")]
fn unescape_mount_point(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let is_escape = bytes[i] == b'\\' && i + 3 < bytes.len() && bytes[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b));
        if is_escape {
            let value = bytes[i + 1..i + 4].iter().fold(0u32, |value, b| value * 8 + u32::from(b - b'0'));
            result.push(value as u8);
            i += 4;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// `DriveInfo.GetDrives().Select(x => x.Name)` on the other Unix systems: the root, and on macOS
/// the volumes mounted below `/Volumes` (a link there, such as the one to the startup volume,
/// is not a mount point of its own).
#[cfg(all(not(windows), not(target_os = "linux")))]
fn get_drives() -> Vec<String> {
    let mut drives = vec!["/".to_string()];
    if cfg!(target_os = "macos") {
        if let Ok(volumes) = std::fs::read_dir("/Volumes") {
            let mut mounted: Vec<String> = volumes
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
                .map(|entry| entry.path().to_string_lossy().into_owned())
                .collect();
            mounted.sort();
            drives.extend(mounted);
        }
    }
    drives
}

/// `IconConverter`: the icon of a file, of an expanded directory or of a collapsed directory.
struct IconConverter {
    file: Rc<dyn IImage>,
    folder_expanded: Rc<dyn IImage>,
    folder_collapsed: Rc<dyn IImage>,
}

impl IconConverter {
    fn new(file: Rc<dyn IImage>, folder_expanded: Rc<dyn IImage>, folder_collapsed: Rc<dyn IImage>) -> Self {
        Self { file, folder_expanded, folder_collapsed }
    }
}

impl IMultiValueConverter for IconConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if values.len() == 2 {
            let is_directory = values[0].as_ref().and_then(|value| value.downcast_ref::<bool>()).copied();
            let is_expanded = values[1].as_ref().and_then(|value| value.downcast_ref::<bool>()).copied();
            if let (Some(is_directory), Some(is_expanded)) = (is_directory, is_expanded) {
                let icon = if !is_directory {
                    &self.file
                } else if is_expanded {
                    &self.folder_expanded
                } else {
                    &self.folder_collapsed
                };
                return Ok(Some(Rc::new(icon.clone()) as BoxedValue));
            }
        }

        Ok(None)
    }
}

ferro_markup_type!(class FilesPageViewModel {
    this: Rc<FilesPageViewModel>,
    handles: [FilesPageViewModel, Rc<FilesPageViewModel>, Option<Rc<FilesPageViewModel>>],
    properties: [
        CellSelection: bool {
            get: |this: &Rc<FilesPageViewModel>| this.cell_selection(),
            set: |this: &Rc<FilesPageViewModel>, value: bool| this.set_cell_selection(value)
        },
        Drives: ItemsSource { get: |this: &Rc<FilesPageViewModel>| this.drive_items.clone() },
        FlatList: bool {
            get: |this: &Rc<FilesPageViewModel>| this.flat_list(),
            set: |this: &Rc<FilesPageViewModel>, value: bool| this.set_flat_list(value)
        },
        SelectedDrive: String {
            get: |this: &Rc<FilesPageViewModel>| this.selected_drive(),
            set: |this: &Rc<FilesPageViewModel>, value: String| this.set_selected_drive(value)
        },
        SelectedPath: Option<String> {
            get: |this: &Rc<FilesPageViewModel>| this.selected_path(),
            set: |this: &Rc<FilesPageViewModel>, value: Option<String>| this.set_selected_path(value)
        },
        Source: TreeDataGridSourceRef { get: |this: &Rc<FilesPageViewModel>| this.source_ref() },
    ],
    static_properties: [
        FileIconConverter: Rc<dyn ferroui_base::data::converters::IMultiValueConverter> {
            get: || FilesPageViewModel::file_icon_converter()
        },
    ],
    notify_property_changed: FilesPageViewModel,
});
