//! Port of `FileSystem/FileSystemWatcherTrigger.cs`.

use crate::background;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, Interaction, Trigger};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// How often the watcher looks at the file system.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// The kind of a change of the file system (`WatcherChangeTypes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WatcherChangeTypes {
    /// The creation of a file or folder.
    Created,
    /// The deletion of a file or folder.
    Deleted,
    /// The change of a file or folder (of its content or of its time of the last write).
    Changed,
    /// The renaming of a file or folder.
    Renamed,
}

ferroui_base::ferro_markup_enum!(WatcherChangeTypes { Created, Deleted, Changed, Renamed });

/// A change of the file system, the parameter of the actions of a [`FileSystemWatcherTrigger`]
/// (`FileSystemEventArgs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemEventArgs {
    /// The kind of the change.
    pub change_type: WatcherChangeTypes,
    /// The full path of the affected file or folder.
    pub full_path: String,
    /// The path of the affected file or folder relative to the watched folder.
    pub name: String,
}

/// What the watcher knows of an entry of the file system.
#[derive(Clone, Copy, PartialEq, Eq)]
struct EntryState {
    is_directory: bool,
    modified: Option<SystemTime>,
    length: u64,
}

/// A trigger that listens to file system events.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference uses the change notifications of the
/// operating system (`FileSystemWatcher`); the port compares snapshots of the watched folder that
/// a background thread takes every half second, so a renaming is reported as a deletion and a
/// creation, and only the changes of files are reported as changes. Behind the feature
/// `file-system` (the browser has no local file system).
#[repr(C)]
pub struct FileSystemWatcherTrigger {
    base: Trigger,
    watcher: RefCell<Option<(u64, Arc<AtomicBool>)>>,
}

ferro_class!(FileSystemWatcherTrigger: Trigger);
ferro_class_info!(FileSystemWatcherTrigger { new: FileSystemWatcherTrigger::new });

ferro_properties! {
    impl FileSystemWatcherTrigger {
        /// Identifies the `Path` property.
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<FileSystemWatcherTrigger, _>("Path", None)
        }

        /// Identifies the `Filter` property.
        pub fn filter_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<FileSystemWatcherTrigger, _>("Filter", Some("*.*".to_string()))
        }

        /// Identifies the `IncludeSubdirectories` property.
        pub fn include_subdirectories_property() -> StyledProperty<bool> {
            FerroProperty::register::<FileSystemWatcherTrigger, _>("IncludeSubdirectories", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for FileSystemWatcherTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.start_watcher();
        }

        fn on_detaching(this: &Self) {
            this.stop_watcher();
            Self::parent_on_detaching(this);
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for FileSystemWatcherTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::path_property().as_property()
                || property == Self::filter_property().as_property()
                || property == Self::include_subdirectories_property().as_property()
            {
                this.stop_watcher();
                this.start_watcher();
            }
        }
    }
}

impl FileSystemWatcherTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Trigger::construct(), watcher: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the path to watch.
    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, value: Option<String>) {
        self.set_value(Self::path_property(), value)
    }

    /// Gets or sets the filter string used to determine what files are monitored in a directory
    /// (`*` and `?` wildcards; `*.*` monitors every file).
    pub fn filter(&self) -> Option<String> {
        self.get_value(Self::filter_property())
    }

    pub fn set_filter(&self, value: Option<String>) {
        self.set_value(Self::filter_property(), value)
    }

    /// Gets or sets a value indicating whether subdirectories within the specified path should be
    /// monitored.
    pub fn include_subdirectories(&self) -> bool {
        self.get_value(Self::include_subdirectories_property())
    }

    pub fn set_include_subdirectories(&self, value: bool) {
        self.set_value(Self::include_subdirectories_property(), value)
    }

    fn start_watcher(&self) {
        if self.watcher.borrow().is_some() {
            return;
        }

        let Some(path) = self.path().filter(|path| !path.is_empty() && Path::new(path).is_dir()) else {
            return;
        };

        let filter = self.filter().filter(|filter| !filter.is_empty()).unwrap_or_else(|| "*.*".to_string());
        let include_subdirectories = self.include_subdirectories();

        let id = background::register(self);
        let stop = Arc::new(AtomicBool::new(false));
        let dispatcher = Dispatcher::ui_thread();
        let stopped = stop.clone();
        let spawned = std::thread::Builder::new().name("FileSystemWatcherTrigger".to_string()).spawn(move || {
            let root = PathBuf::from(&path);
            let mut previous = snapshot(&root, include_subdirectories);
            while !stopped.load(Ordering::Relaxed) {
                std::thread::sleep(POLL_INTERVAL);
                if stopped.load(Ordering::Relaxed) {
                    break;
                }

                let current = snapshot(&root, include_subdirectories);
                for event_args in changes(&root, &previous, &current, &filter) {
                    background::post_to(&dispatcher, id, move |target| {
                        if let Some(trigger) = target.downcast_ref::<FileSystemWatcherTrigger>() {
                            trigger.on_changed(event_args);
                        }
                    });
                }
                previous = current;
            }
        });

        if spawned.is_err() {
            background::unregister(id);
            return;
        }

        *self.watcher.borrow_mut() = Some((id, stop));
    }

    fn stop_watcher(&self) {
        let watcher = self.watcher.borrow_mut().take();
        let Some((id, stop)) = watcher else {
            return;
        };

        stop.store(true, Ordering::Relaxed);
        background::unregister(id);
    }

    fn on_changed(&self, event_args: FileSystemEventArgs) {
        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        let parameter: BoxedValue = Rc::new(event_args);
        Interaction::execute_actions(sender.as_ref(), Some(&self.actions()), Some(&parameter));
    }
}

/// The entries of the watched folder (and of its subfolders, when they are watched).
fn snapshot(root: &Path, include_subdirectories: bool) -> HashMap<PathBuf, EntryState> {
    let mut entries = HashMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(read_dir) = std::fs::read_dir(&directory) else {
            continue;
        };

        for entry in read_dir.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            let path = entry.path();
            let is_directory = metadata.is_dir();
            if is_directory && include_subdirectories {
                pending.push(path.clone());
            }

            entries.insert(path, EntryState { is_directory, modified: metadata.modified().ok(), length: metadata.len() });
        }
    }
    entries
}

/// The changes between two snapshots of the entries whose names match `filter`.
fn changes(
    root: &Path,
    previous: &HashMap<PathBuf, EntryState>,
    current: &HashMap<PathBuf, EntryState>,
    filter: &str,
) -> Vec<FileSystemEventArgs> {
    let mut result = Vec::new();
    let mut event = |change_type: WatcherChangeTypes, path: &Path| {
        let file_name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if !matches_filter(&file_name, filter) {
            return;
        }

        let name = path.strip_prefix(root).unwrap_or(path).to_string_lossy().into_owned();
        result.push(FileSystemEventArgs { change_type, full_path: path.to_string_lossy().into_owned(), name });
    };

    for (path, state) in current {
        match previous.get(path) {
            None => event(WatcherChangeTypes::Created, path),
            Some(previous_state) if !state.is_directory && previous_state != state => {
                event(WatcherChangeTypes::Changed, path)
            }
            _ => {}
        }
    }

    for path in previous.keys() {
        if !current.contains_key(path) {
            event(WatcherChangeTypes::Deleted, path);
        }
    }

    result
}

/// Whether a file name matches a filter with the wildcards `*` (any characters) and `?` (one
/// character); `*` and `*.*` match every name, as they do for the watcher of the reference.
fn matches_filter(name: &str, filter: &str) -> bool {
    if filter == "*" || filter == "*.*" {
        return true;
    }

    fn matches(name: &[char], pattern: &[char]) -> bool {
        match pattern.split_first() {
            None => name.is_empty(),
            Some((&'*', rest)) => (0..=name.len()).any(|skip| matches(&name[skip..], rest)),
            Some((&'?', rest)) => !name.is_empty() && matches(&name[1..], rest),
            Some((c, rest)) => {
                name.first().is_some_and(|first| first.eq_ignore_ascii_case(c)) && matches(&name[1..], rest)
            }
        }
    }

    let name: Vec<char> = name.chars().collect();
    let pattern: Vec<char> = filter.chars().collect();
    matches(&name, &pattern)
}
