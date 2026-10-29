//! Port of `ViewModels/FileSystemViewModel.cs` of the reference sample.

use super::items::{text_list, ItemList};
use super::{Command, ViewModelBase};
use ferroui_base::input::ICommand;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use ferroui_xaml_interactions::file_system::FileSystemEventArgs;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The view model of the file system page: the folder its actions and its watcher work on, the
/// text the page writes, and the log of the watcher.
pub struct FileSystemViewModel {
    base: ViewModelBase,
    target_path: RefCell<Option<String>>,
    file_content: RefCell<Option<String>>,
    logs: RefCell<ItemList>,
    log_event_command: RefCell<Option<Rc<dyn ICommand>>>,
}
reactive_object!(FileSystemViewModel, base);
super::reactive_accessors!(FileSystemViewModel {
    target_path, set_target_path, "TargetPath": Option<String>;
    file_content, set_file_content, "FileContent": Option<String>;
    logs, set_logs, "Logs": ItemList;
});

impl PartialEq for FileSystemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FileSystemViewModel {
    /// `FileSystemViewModel()`.
    pub fn new() -> Rc<Self> {
        // Deviation (DEVIATIONS.md, Behaviors gallery): the folder and the text name FerroUI
        // where the reference names its framework.
        let target_path = std::env::temp_dir().join("FerroUIBehaviorsTest");
        let view_model = Rc::new(Self {
            base: ViewModelBase::new(),
            target_path: RefCell::new(Some(target_path.to_string_lossy().into_owned())),
            file_content: RefCell::new(Some("Hello FerroUI Behaviors!".to_string())),
            logs: RefCell::new(text_list([])),
            log_event_command: RefCell::new(None),
        });
        let weak: Weak<Self> = Rc::downgrade(&view_model);
        *view_model.log_event_command.borrow_mut() = Some(Command::create_with(move |args| {
            if let Some(view_model) = weak.upgrade() {
                view_model.log_event(args);
            }
        }));
        view_model
    }

    /// `LogEventCommand`.
    pub fn log_event_command(&self) -> Option<Rc<dyn ICommand>> {
        self.log_event_command.borrow().clone()
    }

    /// `LogEvent(object?)`: the newest entry first.
    ///
    /// The watcher of the port reports a renaming as a deletion and a creation (no
    /// `RenamedEventArgs`), so the branch of the reference for a renaming has no counterpart.
    fn log_event(&self, args: Option<&BoxedValue>) {
        let entry = match args.and_then(|args| args.downcast_ref::<FileSystemEventArgs>()) {
            Some(e) => format!("{:?}: {}", e.change_type, e.full_path),
            None => match args.and_then(|args| args.downcast_ref::<String>()) {
                Some(text) => format!("Event: {text}"),
                None => "Event: ".to_string(),
            },
        };
        self.logs().insert(0, Rc::new(entry) as BoxedValue);
    }
}

type Vm = Rc<FileSystemViewModel>;

ferro_markup_type!(class FileSystemViewModel {
    this: Rc<FileSystemViewModel>,
    handles: [FileSystemViewModel, Rc<FileSystemViewModel>, Option<Rc<FileSystemViewModel>>],
    constructors: [() => FileSystemViewModel::new],
    properties: [
        TargetPath: Option<String> { get: |this: &Vm| this.target_path(), set: |this: &Vm, value: Option<String>| this.set_target_path(value) },
        FileContent: Option<String> { get: |this: &Vm| this.file_content(), set: |this: &Vm, value: Option<String>| this.set_file_content(value) },
        Logs: ItemsSource { get: |this: &Vm| ItemsSource::from(this.logs()) },
        LogEventCommand: Option<Rc<dyn ICommand>> { get: |this: &Vm| this.log_event_command() },
    ],
    notify_property_changed: FileSystemViewModel,
});
