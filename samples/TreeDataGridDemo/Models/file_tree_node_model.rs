//! Port of `Models/FileTreeNodeModel.cs`.
//!
//! A node of the file tree: a file or a directory, whose children are read from the file system
//! when they are first asked for, and kept up to date while the directory is shown.
//!
//! The reference watches the directory of loaded children with a `FileSystemWatcher` and applies
//! its events (changed, created, deleted, renamed) on the UI thread. Neither the framework nor the
//! workspace has a file system watcher, so the port polls: once the children of a directory are
//! loaded, a dispatcher timer rescans the directory while its row is expanded and applies the
//! differences to the children as the reference's handlers apply the events: the size and the
//! modification time of a changed file are updated, a new entry is appended, a removed entry is
//! removed, and (on Unix, where an entry keeps its file identity when it is renamed) a renamed
//! entry gets its new path and name in place.

use ferroui_base::animation::TimeSpan;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, ModelTypes};
use ferroui_base::ferro_markup_type;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::{DateTimeOffset, StringComparison};
use ferroui_controls_treedatagrid::models::{register_editable_model, IEditableObject};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// How often a directory whose children are shown is rescanned (the stand-in of the events of
/// the reference's `FileSystemWatcher`).
const WATCH_INTERVAL: Duration = Duration::from_millis(1000);

/// The identity of a file system entry that survives a rename: the device and the inode of the
/// entry on Unix. Other platforms have none in the standard library, and a rename there is a
/// removal and an addition.
type FileId = (u64, u64);

/// A comparison of two nodes, either of which may be null (`Comparison<FileTreeNodeModel?>`).
pub type FileTreeNodeComparison = Rc<dyn Fn(Option<&FileTreeNodeModel>, Option<&FileTreeNodeModel>) -> Ordering>;

/// The default comparison of the values a sort reads (`Comparer<T>.Default`): text by the rules of
/// the current culture, nullable values with null first.
pub trait DefaultComparer {
    fn default_compare(&self, other: &Self) -> Ordering;
}

impl DefaultComparer for String {
    fn default_compare(&self, other: &Self) -> Ordering {
        StringComparison::CurrentCulture.compare(self, other)
    }
}

impl<T: PartialOrd> DefaultComparer for Option<T> {
    fn default_compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(x), Some(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        }
    }
}

/// A file or a directory of the file tree.
pub struct FileTreeNodeModel {
    weak_self: Weak<FileTreeNodeModel>,
    property_changed: Event<str>,
    path: RefCell<String>,
    name: RefCell<String>,
    undo_name: RefCell<Option<String>>,
    size: Cell<Option<i64>>,
    modified: Cell<Option<DateTimeOffset>>,
    // The polling timer that stands in for the reference's `FileSystemWatcher`.
    watcher: RefCell<Option<Rc<dyn IDisposable>>>,
    children: RefCell<Option<Rc<FerroList<Rc<FileTreeNodeModel>>>>>,
    has_children: Cell<bool>,
    is_expanded: Cell<bool>,
    is_checked: Cell<bool>,
    is_directory: bool,
    // The identity of the entry, which matches a renamed entry with its node.
    file_id: Cell<Option<FileId>>,
}

impl PartialEq for FileTreeNodeModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for FileTreeNodeModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl Drop for FileTreeNodeModel {
    fn drop(&mut self) {
        if let Some(watcher) = self.watcher.get_mut().take() {
            watcher.dispose();
        }
    }
}

impl FileTreeNodeModel {
    pub fn new(path: String, is_directory: bool, is_root: bool) -> Rc<Self> {
        Self::register();

        let name = if is_root { path.clone() } else { file_name(&path) };
        let (size, modified) = if is_directory { (None, None) } else { file_info(&path) };
        let file_id = identity_of(&path);

        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            property_changed: Event::new(),
            path: RefCell::new(path),
            name: RefCell::new(name),
            undo_name: RefCell::new(None),
            size: Cell::new(size),
            modified: Cell::new(modified),
            watcher: RefCell::new(None),
            children: RefCell::new(None),
            // `HasChildren = isDirectory`.
            has_children: Cell::new(is_directory),
            is_expanded: Cell::new(is_root),
            is_checked: Cell::new(false),
            is_directory,
            file_id: Cell::new(file_id),
        })
    }

    /// States what the reference's runtime knows of the type by its interfaces: the source and
    /// its columns observe the nodes through their property change notifications
    /// (`INotifyPropertyChanged`), and a template cell forwards its edits to them
    /// (`IEditableObject`). Once per thread.
    fn register() {
        if !ModelTypes::is_registered(TypeId::of::<FileTreeNodeModel>()) {
            ModelTypes::register::<FileTreeNodeModel>(|builder| builder.notify_property_changed());
            register_editable_model::<FileTreeNodeModel>();
        }
    }

    pub fn path(&self) -> String {
        self.path.borrow().clone()
    }

    fn set_path(&self, value: String) {
        self.raise_and_set_if_changed(&self.path, value, "Path");
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// The reference's setter is private; the editing template of the name column writes it.
    pub fn set_name(&self, value: String) {
        self.raise_and_set_if_changed(&self.name, value, "Name");
    }

    pub fn size(&self) -> Option<i64> {
        self.size.get()
    }

    fn set_size(&self, value: Option<i64>) {
        if self.size.get() != value {
            self.size.set(value);
            self.raise_property_changed("Size");
        }
    }

    pub fn modified(&self) -> Option<DateTimeOffset> {
        self.modified.get()
    }

    fn set_modified(&self, value: Option<DateTimeOffset>) {
        // `RaiseAndSetIfChanged` compares with `EqualityComparer<T>.Default`, which for a
        // `DateTimeOffset` compares the instants.
        if self.modified.get() != value {
            self.modified.set(value);
            self.raise_property_changed("Modified");
        }
    }

    pub fn has_children(&self) -> bool {
        self.has_children.get()
    }

    fn set_has_children(&self, value: bool) {
        if self.has_children.get() != value {
            self.has_children.set(value);
            self.raise_property_changed("HasChildren");
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        if self.is_expanded.get() != value {
            self.is_expanded.set(value);
            self.raise_property_changed("IsExpanded");
        }
    }

    pub fn is_checked(&self) -> bool {
        self.is_checked.get()
    }

    pub fn set_is_checked(&self, value: bool) {
        self.is_checked.set(value);
    }

    pub fn is_directory(&self) -> bool {
        self.is_directory
    }

    /// The children, read from the file system on first access.
    ///
    /// # Panics
    /// When the node is a file (`NotSupportedException`).
    pub fn children(&self) -> Rc<FerroList<Rc<FileTreeNodeModel>>> {
        let children = self.children.borrow().clone();
        match children {
            Some(children) => children,
            None => self.load_children(),
        }
    }

    fn load_children(&self) -> Rc<FerroList<Rc<FileTreeNodeModel>>> {
        if !self.is_directory {
            panic!("Specified method is not supported.");
        }

        // `new EnumerationOptions { IgnoreInaccessible = true }`: the directories, then the files.
        let result = Rc::new(FerroList::from_items(
            enumerate(&self.path()).into_iter().map(|entry| FileTreeNodeModel::new(entry.path, entry.is_directory, false)),
        ));

        // The children are stored before the notification below: a handler of `HasChildren` may
        // read them.
        *self.children.borrow_mut() = Some(result.clone());
        self.start_watching();

        if result.count() == 0 {
            self.set_has_children(false);
        }

        result
    }

    /// `SortAscending<T>(selector)`: null first, then the directories before the files, each
    /// ordered by the value `selector` reads.
    pub fn sort_ascending<T: DefaultComparer>(selector: impl Fn(&FileTreeNodeModel) -> T + 'static) -> FileTreeNodeComparison {
        Rc::new(move |x: Option<&FileTreeNodeModel>, y: Option<&FileTreeNodeModel>| match (x, y) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.is_directory == y.is_directory {
                    selector(x).default_compare(&selector(y))
                } else if x.is_directory {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        })
    }

    /// `SortDescending<T>(selector)`: null last, then the directories before the files, each
    /// ordered by the value `selector` reads, descending.
    pub fn sort_descending<T: DefaultComparer>(selector: impl Fn(&FileTreeNodeModel) -> T + 'static) -> FileTreeNodeComparison {
        Rc::new(move |x: Option<&FileTreeNodeModel>, y: Option<&FileTreeNodeModel>| match (x, y) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(x), Some(y)) => {
                if x.is_directory == y.is_directory {
                    selector(y).default_compare(&selector(x))
                } else if x.is_directory {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        })
    }

    fn raise_and_set_if_changed(&self, field: &RefCell<String>, value: String, property_name: &str) {
        if *field.borrow() != value {
            // The borrow ends before the handlers run: they read the property.
            *field.borrow_mut() = value;
            self.raise_property_changed(property_name);
        }
    }

    fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }

    /// Starts the timer that keeps the children up to date (the reference creates and enables
    /// its `FileSystemWatcher` here). The timer holds the node weakly and stops with it.
    fn start_watching(&self) {
        let weak = self.weak_self.clone();
        let watcher = DispatcherTimer::run(
            move || {
                let Some(this) = weak.upgrade() else {
                    return false;
                };
                // Only a directory whose children are shown is rescanned; a collapsed directory
                // is brought up to date by the first tick after it is expanded again.
                if this.is_expanded() {
                    this.apply_file_system_changes();
                }
                true
            },
            WATCH_INTERVAL,
            DispatcherPriority::BACKGROUND,
        );
        if let Some(previous) = self.watcher.borrow_mut().replace(watcher) {
            previous.dispose();
        }
    }

    /// Rescans the directory and applies the differences to the children, as the handlers of
    /// the reference's watcher apply its events.
    fn apply_file_system_changes(&self) {
        let Some(children) = self.children.borrow().clone() else {
            return;
        };
        let path = self.path();
        // A directory that cannot be read any more (removed, or no longer accessible) is left
        // as it is: its parent removes it when it is gone.
        if !Path::new(&path).is_dir() {
            return;
        }
        let mut added = enumerate(&path);
        let existing = children.to_vec();

        // The children whose entry is gone, and the entries that have no child.
        let mut removed = Vec::new();
        for child in &existing {
            let child_path = child.path();
            match added.iter().position(|entry| entry.path == child_path && entry.is_directory == child.is_directory) {
                Some(index) => {
                    added.remove(index);
                    // `OnChanged`: the size and the modification time of a changed file.
                    if !child.is_directory && Path::new(&child_path).is_file() {
                        let (size, modified) = file_info(&child_path);
                        child.set_size(size);
                        child.set_modified(modified);
                    }
                }
                None => removed.push(child.clone()),
            }
        }

        // `OnRenamed`: a removed child whose entry appears under another name keeps its node.
        removed.retain(|child| {
            let Some(id) = child.file_id.get() else {
                return true;
            };
            let Some(index) = added.iter().position(|entry| entry.file_id == Some(id) && entry.is_directory == child.is_directory)
            else {
                return true;
            };
            let entry = added.remove(index);
            let name = file_name(&entry.path);
            child.set_path(entry.path);
            child.set_name(name);
            false
        });

        // `OnDeleted`.
        for child in removed {
            if let Some(index) = children.to_vec().iter().position(|x| Rc::ptr_eq(x, &child)) {
                children.remove_at(index);
                if cfg!(debug_assertions) {
                    eprintln!("Removed {}", child.path());
                }
            }
        }

        // `OnCreated`.
        for entry in added {
            children.add(FileTreeNodeModel::new(entry.path, entry.is_directory, false));
        }
    }
}

impl IEditableObject for FileTreeNodeModel {
    fn begin_edit(&self) {
        *self.undo_name.borrow_mut() = Some(self.name());
    }

    fn cancel_edit(&self) {
        // As the reference, the name is restored without a notification.
        let undo_name = self.undo_name.borrow().clone();
        *self.name.borrow_mut() = undo_name.unwrap_or_default();
    }

    fn end_edit(&self) {
        *self.undo_name.borrow_mut() = None;
    }
}

/// An entry of a directory.
struct DirectoryEntry {
    path: String,
    is_directory: bool,
    file_id: Option<FileId>,
}

/// The entries of the directory `path`: its subdirectories, then its files
/// (`Directory.EnumerateDirectories` and `Directory.EnumerateFiles` with
/// `EnumerationOptions { IgnoreInaccessible = true }`). An entry that cannot be read is skipped,
/// a directory that cannot be read has none, and, as the default attributes to skip of the
/// options, hidden and system entries are left out.
fn enumerate(path: &str) -> Vec<DirectoryEntry> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in read_dir.flatten() {
        let entry_path = entry.path();
        // A link to a directory is a directory, a broken link a file.
        let metadata = fs::metadata(&entry_path).ok();
        if is_hidden_or_system(&entry.file_name().to_string_lossy(), metadata.as_ref()) {
            continue;
        }
        let entry_path = entry_path.to_string_lossy().into_owned();
        let is_directory = metadata.as_ref().is_some_and(fs::Metadata::is_dir);
        let file_id = identity_of(&entry_path);
        let entry = DirectoryEntry { path: entry_path, is_directory, file_id };
        if is_directory {
            directories.push(entry);
        } else {
            files.push(entry);
        }
    }
    directories.extend(files);
    directories
}

/// Whether an entry is hidden or a system entry (`FileAttributes.Hidden | FileAttributes.System`).
#[cfg(windows)]
fn is_hidden_or_system(_name: &str, metadata: Option<&fs::Metadata>) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    metadata.is_some_and(|metadata| metadata.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0)
}

/// Whether an entry is hidden (`FileAttributes.Hidden`: a name that starts with a dot).
#[cfg(not(windows))]
fn is_hidden_or_system(name: &str, _metadata: Option<&fs::Metadata>) -> bool {
    name.starts_with('.')
}

/// The identity of the entry `path` itself (not of the target of a link).
#[cfg(unix)]
fn identity_of(path: &str) -> Option<FileId> {
    use std::os::unix::fs::MetadataExt;
    fs::symlink_metadata(path).ok().map(|metadata| (metadata.dev(), metadata.ino()))
}

/// No identity outside Unix: a rename is a removal and an addition.
#[cfg(not(unix))]
fn identity_of(_path: &str) -> Option<FileId> {
    None
}

/// `Path.GetFileName(path)`: the last component of the path.
fn file_name(path: &str) -> String {
    Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// `FileInfo.Length` and `FileInfo.LastWriteTimeUtc` of the file `path`, of the target of a link;
/// none when the file cannot be read.
fn file_info(path: &str) -> (Option<i64>, Option<DateTimeOffset>) {
    match fs::metadata(path).or_else(|_| fs::symlink_metadata(path)) {
        Ok(metadata) => (Some(metadata.len() as i64), metadata.modified().ok().map(to_date_time_offset)),
        Err(_) => (None, None),
    }
}

/// The UTC instant of a system time as a `DateTimeOffset` of offset zero (the conversion of a
/// UTC `DateTime` to `DateTimeOffset`).
fn to_date_time_offset(time: SystemTime) -> DateTimeOffset {
    const NANOSECONDS_PER_TICK: u128 = 100;

    let ticks = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => (after.as_nanos() / NANOSECONDS_PER_TICK) as i64,
        Err(before) => -((before.duration().as_nanos() / NANOSECONDS_PER_TICK) as i64),
    };
    DateTimeOffset::UNIX_EPOCH.add(TimeSpan::from_ticks(ticks))
}

ferro_markup_type!(class FileTreeNodeModel {
    this: Rc<FileTreeNodeModel>,
    handles: [FileTreeNodeModel, Rc<FileTreeNodeModel>, Option<Rc<FileTreeNodeModel>>],
    properties: [
        Path: String { get: |this: &Rc<FileTreeNodeModel>| this.path() },
        Name: String {
            get: |this: &Rc<FileTreeNodeModel>| this.name(),
            set: |this: &Rc<FileTreeNodeModel>, value: String| this.set_name(value)
        },
        Size: Option<i64> { get: |this: &Rc<FileTreeNodeModel>| this.size() },
        Modified: Option<DateTimeOffset> { get: |this: &Rc<FileTreeNodeModel>| this.modified() },
        HasChildren: bool { get: |this: &Rc<FileTreeNodeModel>| this.has_children() },
        IsExpanded: bool {
            get: |this: &Rc<FileTreeNodeModel>| this.is_expanded(),
            set: |this: &Rc<FileTreeNodeModel>, value: bool| this.set_is_expanded(value)
        },
        IsChecked: bool {
            get: |this: &Rc<FileTreeNodeModel>| this.is_checked(),
            set: |this: &Rc<FileTreeNodeModel>, value: bool| this.set_is_checked(value)
        },
        IsDirectory: bool { get: |this: &Rc<FileTreeNodeModel>| this.is_directory() },
    ],
    notify_property_changed: FileTreeNodeModel,
});
