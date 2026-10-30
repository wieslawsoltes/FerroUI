//! End-to-end compiled menu -> native provider -> actual file -> original model restoration.
use super::{drain, run};
use crate::{infrastructure::NativeLayoutFileStorage, view_models::MainWindowViewModel, App};
use ferrodock_ferro::controls::DockControl;
use ferrodock_model::{
    controls::IDocumentDock,
    core::{IDock, IDockable},
};
use ferroui_base::{
    input::LocalBoxFuture,
    interactivity::RoutedEventArgs,
    platform::storage::{
        file_io::{BclStorageFile, BclStorageProvider, FileSystemInfo},
        FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFolder,
        IStorageProvider, OpenFilePickerResult, SaveFilePickerResult,
    },
    FerroLocator, Ref, WeakRef,
};
use ferroui_controls::{platform::IStorageProviderFactory, MenuItem, TextBlock, TopLevel, Window};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    io,
    path::PathBuf,
    rc::Rc,
    task::{Poll, Waker},
};

#[derive(Default)]
struct PickerProvider {
    opens: RefCell<VecDeque<io::Result<OpenFilePickerResult>>>,
    saves: RefCell<VecDeque<io::Result<SaveFilePickerResult>>>,
    open_options: RefCell<Vec<FilePickerOpenOptions>>,
    save_options: RefCell<Vec<FilePickerSaveOptions>>,
    ready: Rc<Cell<bool>>,
    waker: Rc<RefCell<Option<Waker>>>,
}
impl PickerProvider {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            ready: Rc::new(Cell::new(true)),
            ..Self::default()
        })
    }
    fn completion<T: 'static>(&self, result: io::Result<T>) -> LocalBoxFuture<io::Result<T>> {
        let mut result = Some(result);
        let ready = self.ready.clone();
        let waker = self.waker.clone();
        Box::pin(std::future::poll_fn(move |cx| {
            if ready.get() {
                Poll::Ready(result.take().expect("one picker completion"))
            } else {
                *waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }))
    }
    fn resume(&self) {
        self.ready.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
    fn open_file(&self, path: &std::path::Path) {
        self.opens.borrow_mut().push_back(Ok(OpenFilePickerResult {
            files: vec![BclStorageFile::new(FileSystemInfo::file(path))],
            selected_file_type: None,
        }));
    }
    fn save_file(&self, path: &std::path::Path) {
        self.saves.borrow_mut().push_back(Ok(SaveFilePickerResult {
            file: Some(BclStorageFile::new(FileSystemInfo::file(path))),
            selected_file_type: None,
        }));
    }
}
impl BclStorageProvider for PickerProvider {
    fn can_open(&self) -> bool {
        true
    }
    fn can_save(&self) -> bool {
        true
    }
    fn can_pick_folder(&self) -> bool {
        false
    }
    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
        self.open_options.borrow_mut().push(options);
        self.completion(
            self.opens
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(OpenFilePickerResult::default())),
        )
    }
    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
        self.save_options.borrow_mut().push(options);
        self.completion(
            self.saves
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(SaveFilePickerResult::default())),
        )
    }
    fn open_folder_picker_async(
        &self,
        _: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        Box::pin(std::future::ready(Ok(Vec::new())))
    }
}
struct ProviderFactory {
    provider: Rc<PickerProvider>,
    parents: RefCell<Vec<WeakRef<TopLevel>>>,
}
impl IStorageProviderFactory for ProviderFactory {
    fn create_provider(&self, parent: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
        self.parents.borrow_mut().push(parent.downgrade());
        self.provider.clone()
    }
}
struct FileFixture {
    directory: PathBuf,
    locator_scope: Rc<dyn ferroui_system::IDisposable>,
    factory: Rc<ProviderFactory>,
}
impl FileFixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "ferrodock-native-files-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let locator_scope = FerroLocator::enter_scope();
        let factory = Rc::new(ProviderFactory {
            provider: PickerProvider::new(),
            parents: RefCell::new(Vec::new()),
        });
        FerroLocator::current_mutable()
            .bind::<dyn IStorageProviderFactory>()
            .to_constant(factory.clone() as Rc<dyn IStorageProviderFactory>);
        Self {
            directory,
            locator_scope,
            factory,
        }
    }
    fn window(&self) -> (Rc<MainWindowViewModel>, Ref<crate::views::MainWindow>) {
        let model = App::create_main_view_model();
        let window = crate::views::MainWindow::new();
        window.set_data_context(Some(Rc::new(model.clone())));
        window.show();
        drain(&window);
        // Attach after the view has bound its initially-null file model; this exercises
        // compiled nested-path notifications as well as native ancestor resolution.
        let view = window
            .find_descendant_of_type::<crate::views::MainView>(false)
            .unwrap();
        model.attach_layout_files(NativeLayoutFileStorage::new(view.upcast()));
        drain(&window);
        (model, window)
    }
}
impl Drop for FileFixture {
    fn drop(&mut self) {
        self.locator_scope.dispose();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn menu(window: &Window, name: &str) -> Ref<MenuItem> {
    window
        .get_logical_descendants()
        .filter_map(|v| v.cast::<MenuItem>())
        .find(|v| v.name().as_deref() == Some(name))
        .unwrap()
}
fn click(window: &Window, name: &str) -> bool {
    let args = RoutedEventArgs::with_event(MenuItem::click_event());
    menu(window, name).raise_event(&args);
    drain(window);
    args.handled()
}
fn status(window: &Window) -> String {
    window
        .find_descendant_of_type_where::<TextBlock>(false, |v| {
            v.name().as_deref() == Some("LayoutFileStatus")
        })
        .unwrap()
        .text()
        .unwrap_or_default()
}

#[test]
fn native_picker_file_menu_roundtrip_restores_views_locators_and_commands_under_both_themes() {
    run(|| {
        for theme in [0, 1] {
            App::create_theme_view_model(App::theme_manager().unwrap())
                .select_tab_theme()
                .execute(Some(&theme));
            let fixture = FileFixture::new();
            let provider = &fixture.factory.provider;
            let path = fixture.directory.join("layout.json");
            std::fs::write(&path, b"previous file").unwrap();
            let (model, window) = fixture.window();
            let original = model.layout().unwrap();
            let title = original.title();
            provider.save_file(&path);
            provider.ready.set(false);
            assert!(click(&window, "SaveLayoutMenuItem"));
            assert!(model.layout_files().unwrap().is_busy());
            assert_eq!(status(&window), "Saving layout…");
            assert!(!menu(&window, "OpenLayoutMenuItem")
                .command()
                .unwrap()
                .can_execute(None));
            assert!(!click(&window, "OpenLayoutMenuItem"));
            assert_eq!(std::fs::read(&path).unwrap(), b"previous file");
            provider.resume();
            drain(&window);
            assert_eq!(status(&window), "Saved layout.json");
            assert!(!model.layout_files().unwrap().is_busy());
            let bytes = std::fs::read(&path).unwrap();
            assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
            assert!(String::from_utf8_lossy(&bytes).contains("CustomDocumentDock"));
            original.set_title("Changed after save".into());
            provider.open_file(&path);
            assert!(click(&window, "OpenLayoutMenuItem"));
            assert_eq!(status(&window), "Loaded layout.json");
            let loaded = model.layout().unwrap();
            assert!(!Rc::ptr_eq(&original, &loaded));
            assert_eq!(loaded.title(), title);
            let control = window
                .find_descendant_of_type::<DockControl>(false)
                .unwrap();
            assert!(Rc::ptr_eq(
                &control.layout().unwrap(),
                &(loaded.clone() as Rc<dyn IDock>)
            ));
            assert!(window
                .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
                .is_some());
            assert!(window
                .find_descendant_of_type::<crate::views::tools::Tool1View>(false)
                .is_some());
            let factory = loaded.factory().unwrap();
            let lookup = |name: &str| {
                let locators = factory.dockable_locator().unwrap();
                let create = locators
                    .borrow()
                    .get(&name.to_owned())
                    .cloned()
                    .flatten()
                    .unwrap();
                create().unwrap()
            };
            assert!(Rc::ptr_eq(
                &lookup("Root"),
                &(loaded.clone() as Rc<dyn IDockable>)
            ));
            let documents = lookup("Documents");
            let custom = documents
                .boxed_reference()
                .downcast_ref::<Rc<crate::view_models::docks::CustomDocumentDock>>()
                .unwrap()
                .clone();
            assert!(custom
                .visible_dockables()
                .unwrap()
                .get(0)
                .unwrap()
                .context()
                .is_some());
            custom.create_document().unwrap().execute(None);
            drain(&window);
            assert_eq!(custom.active_dockable().unwrap().id(), "Document4");
            // The original sample registers contexts for Document1..3 only.
            assert!(custom.active_dockable().unwrap().context().is_none());
            assert!(Rc::ptr_eq(
                &custom.active_dockable().unwrap().factory().unwrap(),
                &factory
            ));
            assert_eq!(fixture.factory.parents.borrow().len(), 1);
            assert_eq!(
                fixture.factory.parents.borrow()[0].upgrade().unwrap(),
                window.clone().upcast::<TopLevel>()
            );
            let options = provider.save_options.borrow();
            assert_eq!(options.len(), 1);
            assert_eq!(options[0].default_extension(), Some("json"));
            assert_eq!(options[0].show_overwrite_prompt(), Some(true));
            let options = provider.open_options.borrow();
            assert_eq!(options.len(), 1);
            assert!(!options[0].allow_multiple());
            assert_eq!(
                options[0].file_type_filter().unwrap()[0]
                    .patterns()
                    .unwrap()
                    .as_ref(),
                ["*.json"]
            );
            window.close();
            App::create_theme_view_model(App::theme_manager().unwrap())
                .select_tab_theme()
                .execute(Some(&0_i32));
        }
    });
}

#[test]
fn native_picker_cancellation_and_picker_file_or_parse_failures_preserve_the_bound_layout() {
    run(|| {
        for theme in [0, 1] {
            App::create_theme_view_model(App::theme_manager().unwrap())
                .select_tab_theme()
                .execute(Some(&theme));
            let fixture = FileFixture::new();
            let provider = &fixture.factory.provider;
            let (model, window) = fixture.window();
            let original = model.layout().unwrap();
            assert!(click(&window, "OpenLayoutMenuItem"));
            assert_eq!(status(&window), "Open cancelled.");
            assert!(click(&window, "SaveLayoutMenuItem"));
            assert_eq!(status(&window), "Save cancelled.");
            provider.opens.borrow_mut().push_back(Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Picker denied",
            )));
            assert!(click(&window, "OpenLayoutMenuItem"));
            assert_eq!(status(&window), "Loading layout failed: Picker denied");
            provider
                .saves
                .borrow_mut()
                .push_back(Err(io::Error::other("Save picker failed")));
            assert!(click(&window, "SaveLayoutMenuItem"));
            assert_eq!(status(&window), "Saving layout failed: Save picker failed");
            provider.open_file(&fixture.directory.join("missing.json"));
            assert!(click(&window, "OpenLayoutMenuItem"));
            assert!(status(&window).starts_with("Loading layout failed:"));
            let path = fixture.directory.join("invalid.json");
            std::fs::write(&path, b"{").unwrap();
            provider.open_file(&path);
            assert!(click(&window, "OpenLayoutMenuItem"));
            assert!(status(&window).starts_with("Loading layout failed:"));
            provider.save_file(&path.join("child.json"));
            assert!(click(&window, "SaveLayoutMenuItem"));
            assert!(status(&window).starts_with("Saving layout failed:"));
            assert!(Rc::ptr_eq(&original, &model.layout().unwrap()));
            let control = window
                .find_descendant_of_type::<DockControl>(false)
                .unwrap();
            assert!(Rc::ptr_eq(
                &control.layout().unwrap(),
                &(original as Rc<dyn IDock>)
            ));
            assert!(!model.layout_files().unwrap().is_busy());
            assert!(menu(&window, "OpenLayoutMenuItem")
                .command()
                .unwrap()
                .can_execute(None));
            std::fs::write(&path, [0xff, 0xfe, b'n', 0, b'u', 0, b'l', 0, b'l', 0]).unwrap();
            provider.open_file(&path);
            assert!(click(&window, "OpenLayoutMenuItem"));
            assert_eq!(status(&window), "Loaded invalid.json");
            assert!(model.layout().is_none());
            assert!(control.layout().is_none());
            assert_eq!(model.global_status(), "Global: (none)");
            window.close();
            App::create_theme_view_model(App::theme_manager().unwrap())
                .select_tab_theme()
                .execute(Some(&0_i32));
        }
    });
}
