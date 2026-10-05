//! Not from upstream: the upstream project has no tests. The managed storage
//! provider shows the chooser in a dialog window or in a popup and resolves
//! with what the chooser completes with.

use crate::internal::ManagedFileChooserViewModel;
use crate::{ManagedFileDialogOptions, ManagedStorageProvider};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::storage::file_io::{BclStorageFolder, FileSystemInfo};
use ferroui_base::platform::storage::{FolderPickerOpenOptions, IStorageFolder, IStorageItem, IStorageProvider};
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::primitives::Popup;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Canvas, ContentControl, Control, Panel, TopLevel, Window, WindowBase};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::RefCell;
use std::rc::Rc;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    crate::register_types();
    let scope = UnitTestApplication::start(TestServices::styled_window());
    FerroRuntimeXamlLoader::register();
    scope
}

fn temp_folder(name: &str) -> (std::path::PathBuf, Rc<dyn IStorageFolder>) {
    let root = std::env::temp_dir().join(format!("ferroui-dialogs-provider-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let folder = BclStorageFolder::new(FileSystemInfo::directory(&root)) as Rc<dyn IStorageFolder>;
    (root, folder)
}

/// Opens a folder picker through `provider` and returns where its result
/// lands once the dispatcher has run.
fn open_folder_picker(
    provider: &ManagedStorageProvider,
    folder: Rc<dyn IStorageFolder>,
) -> Rc<RefCell<Option<Vec<String>>>> {
    let mut options = FolderPickerOpenOptions::new();
    options.set_suggested_start_location(Some(folder));
    let picked = provider.open_folder_picker_async(options);
    let result = Rc::new(RefCell::new(None));
    let sink = result.clone();
    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        let folders = picked.await.expect("the picker shows");
        let paths = folders
            .iter()
            .map(|folder| {
                let item: &dyn IStorageItem = &**folder;
                item.try_get_local_path().unwrap()
            })
            .collect();
        *sink.borrow_mut() = Some(paths);
    });
    Dispatcher::ui_thread().run_jobs(None);
    result
}

fn model_of(root: &ContentControl) -> Rc<ManagedFileChooserViewModel> {
    from_markup_value::<Rc<ManagedFileChooserViewModel>>(&root.data_context()).expect("the chooser view model")
}

#[test]
fn a_dialog_window_resolves_with_the_completed_folder() {
    let _app = start();
    let (root, folder) = temp_folder("window");
    let parent = Window::new();
    WindowBase::show(&parent);

    let dialog: Rc<RefCell<Option<Ref<Window>>>> = Rc::new(RefCell::new(None));
    let mut options = ManagedFileDialogOptions::new();
    {
        let dialog = dialog.clone();
        options.set_content_root_factory(Some(Rc::new(move || {
            let window = Window::new();
            *dialog.borrow_mut() = Some(window.clone());
            window.upcast::<ContentControl>()
        })));
    }
    let parent_top_level: Ref<TopLevel> = parent.clone().upcast();
    let provider = ManagedStorageProvider::new(Some(&parent_top_level), Some(options));

    let result = open_folder_picker(&provider, folder);
    let window = dialog.borrow().clone().expect("the dialog window");
    assert!(window.is_visible());
    assert_eq!(Some("Select directory".to_string()), window.title());
    assert!(result.borrow().is_none());

    model_of(&window).ok();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(!window.is_visible());
    assert_eq!(Some(vec![root.to_string_lossy().into_owned()]), *result.borrow());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn closing_the_dialog_window_cancels_with_nothing() {
    let _app = start();
    let (root, folder) = temp_folder("cancel");
    let parent = Window::new();
    WindowBase::show(&parent);

    let dialog: Rc<RefCell<Option<Ref<Window>>>> = Rc::new(RefCell::new(None));
    let mut options = ManagedFileDialogOptions::new();
    {
        let dialog = dialog.clone();
        options.set_content_root_factory(Some(Rc::new(move || {
            let window = Window::new();
            *dialog.borrow_mut() = Some(window.clone());
            window.upcast::<ContentControl>()
        })));
    }
    let parent_top_level: Ref<TopLevel> = parent.clone().upcast();
    let provider = ManagedStorageProvider::new(Some(&parent_top_level), Some(options));

    let result = open_folder_picker(&provider, folder);
    dialog.borrow().clone().expect("the dialog window").close();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(Some(Vec::<String>::new()), *result.borrow());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_content_root_is_shown_in_a_popup_over_the_parent() {
    let _app = start();
    let (root, folder) = temp_folder("popup");
    let parent = Window::new();
    let panel = Canvas::new();
    parent.set_content(Some(Control::boxed(&panel)));
    WindowBase::show(&parent);

    let content: Rc<RefCell<Option<Ref<ContentControl>>>> = Rc::new(RefCell::new(None));
    let mut options = ManagedFileDialogOptions::new();
    {
        let content = content.clone();
        options.set_content_root_factory(Some(Rc::new(move || {
            let control = ContentControl::new();
            *content.borrow_mut() = Some(control.clone());
            control
        })));
    }
    let parent_top_level: Ref<TopLevel> = parent.clone().upcast();
    let provider = ManagedStorageProvider::new(Some(&parent_top_level), Some(options));

    let result = open_folder_picker(&provider, folder);
    // The popup is added to the first panel under the top level.
    let popup = parent.find_descendant_of_type::<Popup>(false).expect("the popup");
    let root_panel = parent.find_descendant_of_type::<Panel>(false).expect("a panel");
    assert!(root_panel.children().to_vec().iter().any(|child| child.ptr_eq(&popup.clone().upcast::<Control>())));
    assert!(popup.is_open());
    let content = content.borrow().clone().expect("the content root");
    assert!(popup.child().is_some_and(|child| child.ptr_eq(&content.clone().upcast::<Control>())));

    model_of(&content).ok();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(!popup.is_open());
    assert!(parent.find_descendant_of_type::<Popup>(false).is_none());
    assert_eq!(Some(vec![root.to_string_lossy().into_owned()]), *result.borrow());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn without_parent_or_windows_a_content_root_cannot_be_shown() {
    let _app = start();
    let (root, folder) = temp_folder("no-parent");
    let mut options = ManagedFileDialogOptions::new();
    options.set_content_root_factory(Some(Rc::new(ContentControl::new)));
    let provider = ManagedStorageProvider::new(None, Some(options));

    let mut picker_options = FolderPickerOpenOptions::new();
    picker_options.set_suggested_start_location(Some(folder));
    let picked = provider.open_folder_picker_async(picker_options);
    let failed = Rc::new(RefCell::new(None));
    let sink = failed.clone();
    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        *sink.borrow_mut() = Some(picked.await.is_err());
    });
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(Some(true), *failed.borrow());
    let _ = std::fs::remove_dir_all(&root);
}
