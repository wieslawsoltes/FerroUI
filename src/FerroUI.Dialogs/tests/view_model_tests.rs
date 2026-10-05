//! Not from upstream: the upstream project has no tests. Navigation,
//! filtering and selection of the view model of the managed file chooser,
//! over a directory tree made for each test.

use crate::internal::{
    ManagedFileChooserItemType, ManagedFileChooserItemViewModel, ManagedFileChooserNavigationItem,
    ManagedFileChooserSources, ManagedFileChooserViewModel,
};
use crate::ManagedFileDialogOptions;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::platform::storage::file_io::{BclStorageFolder, FileSystemInfo};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFolder,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{BoxedValue, FerroLocator};
use ferroui_controls::platform::{IMountedVolumeInfoProvider, MountedVolumeInfo};
use ferroui_controls::SelectionMode;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A volume provider that reports no volumes.
struct NoVolumes;

impl IMountedVolumeInfoProvider for NoVolumes {
    fn listen(&self, _mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

/// A directory tree of a test, removed when dropped:
///
/// ```text
/// root/
///   b_folder/        nested.txt
///   A_folder/
///   b.txt  a.TXT  c.png  .hidden.txt
/// ```
struct Tree {
    root: PathBuf,
}

impl Tree {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ferroui-dialogs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("b_folder")).unwrap();
        std::fs::create_dir_all(root.join("A_folder")).unwrap();
        for file in ["b.txt", "a.TXT", "c.png", ".hidden.txt", "b_folder/nested.txt"] {
            std::fs::write(root.join(file), b"12345").unwrap();
        }
        Self { root }
    }

    fn path(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    fn join(&self, name: &str) -> String {
        self.root.join(name).to_string_lossy().into_owned()
    }

    fn folder(&self) -> Rc<dyn IStorageFolder> {
        BclStorageFolder::new(FileSystemInfo::directory(&self.root))
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Isolates the dispatcher and the service locator, with no mounted volumes
/// and one quick link to `quick_link`.
struct Scope {
    locator: Rc<dyn IDisposable>,
    _dispatcher: UnitTestDispatcherScope,
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.locator.dispose();
    }
}

fn scope(quick_link: &str) -> Scope {
    let dispatcher = Dispatcher::unit_test_scope();
    crate::register_types();
    let locator = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IMountedVolumeInfoProvider>().to_constant(Rc::new(NoVolumes));
    let sources = ManagedFileChooserSources::new();
    let quick_link = quick_link.to_string();
    sources.set_get_all_items_delegate(Rc::new(move |_| {
        vec![ManagedFileChooserNavigationItem {
            display_name: Some("Tree".to_string()),
            path: Some(quick_link.clone()),
            item_type: ManagedFileChooserItemType::Folder,
        }]
    }));
    FerroLocator::current_mutable().bind::<ManagedFileChooserSources>().to_constant(Rc::new(sources));
    Scope { locator, _dispatcher: dispatcher }
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn names(items: &FerroList<Rc<ManagedFileChooserItemViewModel>>) -> Vec<String> {
    items.to_vec().iter().map(|item| item.display_name().unwrap()).collect()
}

fn item(model: &ManagedFileChooserViewModel, name: &str) -> Rc<ManagedFileChooserItemViewModel> {
    model.items().to_vec().into_iter().find(|item| item.display_name().as_deref() == Some(name)).unwrap()
}

fn select(model: &ManagedFileChooserViewModel, names: &[&str]) {
    for name in names {
        model.selected_items().add(Some(item(model, name) as BoxedValue));
    }
    run_jobs();
}

fn completions(model: &ManagedFileChooserViewModel) -> Rc<RefCell<Vec<Vec<String>>>> {
    let completed = Rc::new(RefCell::new(Vec::new()));
    let sink = completed.clone();
    model.complete_requested(move |items| sink.borrow_mut().push(items.to_vec()));
    completed
}

/// `options` starting in the root of `tree`.
fn open_options(tree: &Tree, mut options: FilePickerOpenOptions) -> FilePickerOpenOptions {
    options.set_suggested_start_location(Some(tree.folder()));
    options
}

fn file_type(name: &str, patterns: &[&str]) -> Rc<FilePickerFileType> {
    FilePickerFileType::new(Some(name)).with_patterns(patterns)
}

// --- navigation ---

#[test]
fn open_navigates_to_the_suggested_location_with_folders_first_sorted_ignoring_case() {
    let tree = Tree::new("open-navigates");
    let _scope = scope(&tree.path());

    let options = open_options(&tree, FilePickerOpenOptions::new());
    let model = ManagedFileChooserViewModel::for_open_file(&options, ManagedFileDialogOptions::new());

    assert_eq!(Some(tree.path()), model.location());
    assert_eq!(vec!["A_folder", "b_folder", "a.TXT", "b.txt", "c.png"], names(&model.items()));
    assert_eq!(Some("Open file".to_string()), model.title());
    assert_eq!(SelectionMode::SINGLE, model.selection_mode());
    assert!(!model.show_filters());

    let folder = item(&model, "A_folder");
    assert_eq!(ManagedFileChooserItemType::Folder, folder.item_type());
    assert_eq!(Some("File Folder".to_string()), folder.type_());
    assert_eq!(0, folder.size());
    assert_eq!("Icon_Folder", folder.icon_key());

    let file = item(&model, "c.png");
    assert_eq!(ManagedFileChooserItemType::File, file.item_type());
    assert_eq!(Some(".png".to_string()), file.type_());
    assert_eq!(5, file.size());
    assert_eq!(Some(tree.join("c.png")), file.path());
    assert_eq!("Icon_File", file.icon_key());
}

#[test]
fn navigating_to_a_missing_directory_shows_the_root() {
    let tree = Tree::new("missing-directory");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());

    model.navigate(Some(&tree.join("does-not-exist")), None);

    let root = if cfg!(windows) { None } else { Some("/".to_string()) };
    if root.is_some() {
        assert_eq!(root, model.location());
    }
}

#[test]
fn go_up_navigates_to_the_parent_and_stops_at_the_root() {
    let tree = Tree::new("go-up");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    model.navigate(Some(&tree.join("b_folder")), None);
    assert_eq!(vec!["nested.txt"], names(&model.items()));

    model.go_up();
    assert_eq!(Some(tree.path()), model.location());

    if !cfg!(windows) {
        model.navigate(Some("/"), None);
        model.go_up();
        assert_eq!(Some("/".to_string()), model.location());
    }
}

#[test]
fn enter_pressed_navigates_to_a_directory_and_completes_with_a_file() {
    let tree = Tree::new("enter-pressed");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    let completed = completions(&model);

    model.set_location(Some(tree.join("b_folder")));
    model.enter_pressed();
    assert_eq!(vec!["nested.txt"], names(&model.items()));
    assert!(completed.borrow().is_empty());

    model.set_location(Some(tree.join("c.png")));
    model.enter_pressed();
    assert_eq!(vec![vec![tree.join("c.png")]], *completed.borrow());
}

#[test]
fn quick_links_come_from_the_sources_and_follow_the_location() {
    let tree = Tree::new("quick-links");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    let changed = Rc::new(RefCell::new(Vec::new()));
    let sink = changed.clone();
    model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

    assert_eq!(vec!["Tree"], names(&model.quick_links()));
    model.navigate(Some(&tree.path()), None);
    assert_eq!(0, model.quick_links_selected_index());
    assert!(changed.borrow().iter().any(|name| name == "QuickLinksSelectedIndex"));

    model.navigate(Some(&tree.join("b_folder")), None);
    assert_eq!(-1, model.quick_links_selected_index());
}

#[test]
fn quick_links_are_refreshed_when_the_mounted_volumes_change() {
    let tree = Tree::new("volumes-change");
    let _scope = scope(&tree.path());
    let sources = ManagedFileChooserSources::new();
    sources.set_get_user_directories(Rc::new(Vec::new));
    FerroLocator::current_mutable().bind::<ManagedFileChooserSources>().to_constant(Rc::new(sources));
    let volumes = ManagedFileChooserSources::mounted_volumes();
    volumes.clear();

    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    assert!(model.quick_links().is_empty());

    volumes.add(MountedVolumeInfo { volume_label: Some("Data".to_string()), volume_path: Some(tree.path()), volume_size_bytes: 0 });
    // Refreshed from a dispatcher job.
    assert!(model.quick_links().is_empty());
    run_jobs();
    assert_eq!(vec!["Data"], names(&model.quick_links()));
    assert_eq!(ManagedFileChooserItemType::Volume, model.quick_links().get(0).item_type());

    // Completing disposes the subscription.
    model.ok();
    volumes.clear();
    run_jobs();
    assert_eq!(vec!["Data"], names(&model.quick_links()));
}

// --- filtering ---

#[test]
fn hidden_files_are_shown_on_request() {
    let tree = Tree::new("hidden-files");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    model.navigate(Some(&tree.path()), None);
    assert!(!names(&model.items()).contains(&".hidden.txt".to_string()));

    model.set_show_hidden_files(true);

    if !cfg!(windows) {
        assert!(names(&model.items()).contains(&".hidden.txt".to_string()));
    }
    assert!(model.show_hidden_files());
}

#[test]
fn the_first_filter_is_selected_and_applies_to_files_only() {
    let tree = Tree::new("first-filter");
    let _scope = scope(&tree.path());
    let options = open_options(
        &tree,
        FilePickerOpenOptions::new()
            .with_file_type_filter(vec![file_type("Text", &["*.txt"]), file_type("Images", &["*.png"])]),
    );
    let model = ManagedFileChooserViewModel::for_open_file(&options, ManagedFileDialogOptions::new());

    assert!(model.show_filters());
    assert_eq!(2, model.filters().count());
    assert_eq!("Text", model.selected_filter().unwrap().name());
    assert_eq!(vec!["A_folder", "b_folder", "a.TXT", "b.txt"], names(&model.items()));

    model.set_selected_filter(Some(model.filters().get(1)));
    assert_eq!(vec!["A_folder", "b_folder", "c.png"], names(&model.items()));

    model.set_selected_filter(None);
    assert_eq!(vec!["A_folder", "b_folder", "a.TXT", "b.txt", "c.png"], names(&model.items()));
}

#[test]
fn the_all_files_pattern_shows_every_file() {
    let tree = Tree::new("all-files");
    let _scope = scope(&tree.path());
    let options = open_options(&tree, FilePickerOpenOptions::new().with_file_type_filter(vec![file_type("All", &["*.txt", "*.*"])]));
    let model = ManagedFileChooserViewModel::for_open_file(&options, ManagedFileDialogOptions::new());

    assert_eq!(vec!["A_folder", "b_folder", "a.TXT", "b.txt", "c.png"], names(&model.items()));
}

#[test]
fn a_folder_picker_lists_folders_only() {
    let tree = Tree::new("folder-picker");
    let _scope = scope(&tree.path());
    let options = FolderPickerOpenOptions::new().with_allow_multiple(true);
    let mut options = options;
    options.set_suggested_start_location(Some(tree.folder()));
    let model = ManagedFileChooserViewModel::for_open_folder(&options, ManagedFileDialogOptions::new());

    assert_eq!(vec!["A_folder", "b_folder"], names(&model.items()));
    assert!(model.selecting_folder());
    assert_eq!(SelectionMode::MULTIPLE, model.selection_mode());
    assert_eq!(Some("Select directory".to_string()), model.title());
}

// --- selection ---

#[test]
fn selecting_one_file_shows_its_name_and_ok_completes_with_its_path() {
    let tree = Tree::new("select-one");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    model.navigate(Some(&tree.path()), None);
    let completed = completions(&model);
    assert!(!model.can_ok(None));

    select(&model, &["b.txt"]);

    assert_eq!(Some("b.txt".to_string()), model.file_name());
    assert!(model.can_ok(None));
    model.ok();
    assert_eq!(vec![vec![tree.join("b.txt")]], *completed.borrow());
}

#[test]
fn selecting_several_files_quotes_their_names() {
    let tree = Tree::new("select-several");
    let _scope = scope(&tree.path());
    let options = open_options(&tree, FilePickerOpenOptions::new().with_allow_multiple(true));
    let model = ManagedFileChooserViewModel::for_open_file(&options, ManagedFileDialogOptions::new());
    let completed = completions(&model);

    assert_eq!(SelectionMode::MULTIPLE, model.selection_mode());
    select(&model, &["b.txt", "c.png"]);

    assert_eq!(Some("\"b.txt\" \"c.png\"".to_string()), model.file_name());
    model.ok();
    assert_eq!(vec![vec![tree.join("b.txt"), tree.join("c.png")]], *completed.borrow());
}

#[test]
fn selected_folders_are_removed_unless_directory_selection_is_allowed() {
    let tree = Tree::new("select-folders");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    model.navigate(Some(&tree.path()), None);

    select(&model, &["A_folder", "b.txt"]);
    let selected: Vec<_> = model.selected_item_view_models().iter().map(|i| i.display_name().unwrap()).collect();
    assert_eq!(vec!["b.txt"], selected);

    let mut options = ManagedFileDialogOptions::new();
    options.set_allow_directory_selection(true);
    let model = ManagedFileChooserViewModel::new(options);
    model.navigate(Some(&tree.path()), None);

    select(&model, &["A_folder"]);
    assert_eq!(1, model.selected_items().count());
    assert_eq!(Some("A_folder".to_string()), model.file_name());
}

#[test]
fn a_folder_picker_clears_the_selection_and_completes_with_the_location() {
    let tree = Tree::new("folder-picker-ok");
    let _scope = scope(&tree.path());
    let mut options = FolderPickerOpenOptions::new();
    options.set_suggested_start_location(Some(tree.folder()));
    let model = ManagedFileChooserViewModel::for_open_folder(&options, ManagedFileDialogOptions::new());
    let completed = completions(&model);

    select(&model, &["A_folder"]);
    assert!(model.selected_items().is_empty());
    assert!(model.can_ok(None));

    model.ok();
    assert_eq!(vec![vec![tree.path()]], *completed.borrow());
}

#[test]
fn saving_preselects_the_suggested_file_and_adds_the_default_extension() {
    let tree = Tree::new("save");
    let _scope = scope(&tree.path());
    let mut options = FilePickerSaveOptions::new().with_default_extension("txt").with_show_overwrite_prompt(false);
    options.set_suggested_start_location(Some(tree.folder()));
    options.set_suggested_file_name(Some("c.png".to_string()));
    let model = ManagedFileChooserViewModel::for_save_file(&options, ManagedFileDialogOptions::new());
    let completed = completions(&model);

    assert_eq!(Some("Save file".to_string()), model.title());
    let preselected: Vec<_> = model.selected_item_view_models().iter().map(|i| i.display_name().unwrap()).collect();
    assert_eq!(vec!["c.png"], preselected);

    model.set_file_name(Some("new".to_string()));
    assert!(model.can_ok(None));
    model.ok();

    assert_eq!(Some("new.txt".to_string()), model.file_name());
    assert_eq!(vec![vec![tree.join("new.txt")]], *completed.borrow());
}

#[test]
fn saving_over_an_existing_file_asks_first() {
    let tree = Tree::new("overwrite");
    let _scope = scope(&tree.path());
    let mut options = FilePickerSaveOptions::new();
    options.set_suggested_start_location(Some(tree.folder()));
    let model = ManagedFileChooserViewModel::for_save_file(&options, ManagedFileDialogOptions::new());
    let completed = completions(&model);
    let prompted = Rc::new(RefCell::new(Vec::new()));
    let sink = prompted.clone();
    model.overwrite_prompt(move |file_name| sink.borrow_mut().push(file_name.to_string()));

    model.set_file_name(Some("b.txt".to_string()));
    model.ok();
    assert_eq!(vec![tree.join("b.txt")], *prompted.borrow());
    assert!(completed.borrow().is_empty());

    model.set_file_name(Some("other.txt".to_string()));
    model.ok();
    assert_eq!(vec![vec![tree.join("other.txt")]], *completed.borrow());
}

#[test]
fn select_single_file_completes_with_the_file() {
    let tree = Tree::new("single-file");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    model.navigate(Some(&tree.path()), None);
    let completed = completions(&model);

    model.select_single_file(&item(&model, "a.TXT"));

    assert_eq!(vec![vec![tree.join("a.TXT")]], *completed.borrow());
}

#[test]
fn cancel_is_raised_once() {
    let tree = Tree::new("cancel");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    let cancelled = Rc::new(RefCell::new(0));
    let sink = cancelled.clone();
    let weak = Rc::downgrade(&model);
    model.cancel_requested(move || {
        *sink.borrow_mut() += 1;
        // A handler that cancels again (a window closing on cancel) does not recurse.
        if let Some(model) = weak.upgrade() {
            model.cancel();
        }
    });

    model.cancel();
    model.cancel();

    assert_eq!(1, *cancelled.borrow());
}

#[test]
fn the_view_model_is_not_kept_alive_by_the_mounted_volumes() {
    let tree = Tree::new("lifetime");
    let _scope = scope(&tree.path());
    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    let weak = Rc::downgrade(&model);

    drop(model);

    assert!(weak.upgrade().is_none());
    assert!(Path::new(&tree.path()).is_dir());
}
