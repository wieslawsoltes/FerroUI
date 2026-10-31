//! Exercise viewport retention through the real native Save As picker/stream flow.
use crate::{CanvasViewport, DesignerWorkspace};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageItem, IStorageProvider,
    NoopStorageProvider, OpenFilePickerResult, SaveFilePickerResult, StorageItemProperties,
    WellKnownFolder,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::Uri;
use ferroui_base::{Ref, Vector};
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::{Control, Window};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::io::{self, Cursor, Read, Write};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

const ORIGINAL_URI: &str = "memory://views/Original.xaml";
const SAVED_URI: &str = "memory://views/Saved.xaml";
const SOURCE: &str = r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><UserControl.Resources><ControlTemplate x:Key="CardTemplate" TargetType="Button"><Border><ContentPresenter Content="{TemplateBinding Content}"/></Border></ControlTemplate></UserControl.Resources><Button x:Name="Target" Content="Original"/></UserControl>"#;

fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

fn complete<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("The in-memory native storage provider completes synchronously"),
    }
}

#[derive(Default)]
struct MemoryFile {
    bytes: Rc<RefCell<Vec<u8>>>,
    writes: Cell<usize>,
}

struct MemoryWriter(Rc<RefCell<Vec<u8>>>);

impl Write for MemoryWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl IDisposable for MemoryFile {
    fn dispose(&self) {}
}

impl IStorageItem for MemoryFile {
    fn name(&self) -> String {
        "Saved.xaml".into()
    }

    fn path(&self) -> Uri {
        Uri::absolute(SAVED_URI).unwrap()
    }

    fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
        ready(StorageItemProperties::new(
            Some(self.bytes.borrow().len() as u64),
            None,
            None,
        ))
    }

    fn can_bookmark(&self) -> bool {
        false
    }

    fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
        ready(None)
    }

    fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        ready(None)
    }

    fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> {
        self.bytes.borrow_mut().clear();
        ready(Ok(()))
    }

    fn move_async(
        &self,
        _destination: Rc<dyn IStorageFolder>,
    ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
        ready(Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "The save fixture has no folder operations",
        )))
    }

    fn as_storage_file(self: Rc<Self>) -> Option<Rc<dyn IStorageFile>> {
        Some(self)
    }
}

impl IStorageFile for MemoryFile {
    fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
        ready(Ok(
            Box::new(Cursor::new(self.bytes.borrow().clone())) as Box<dyn Read>
        ))
    }

    fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
        self.writes.set(self.writes.get() + 1);
        self.bytes.borrow_mut().clear();
        ready(Ok(
            Box::new(MemoryWriter(self.bytes.clone())) as Box<dyn Write>
        ))
    }
}

struct SaveProvider {
    result: RefCell<Option<Rc<dyn IStorageFile>>>,
    requests: Cell<usize>,
    fallback: NoopStorageProvider,
}

impl IStorageProvider for SaveProvider {
    fn can_open(&self) -> bool {
        false
    }

    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFile>>>> {
        self.fallback.open_file_picker_async(options)
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
        self.fallback.open_file_picker_with_result_async(options)
    }

    fn can_save(&self) -> bool {
        true
    }

    fn save_file_picker_async(
        &self,
        _options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageFile>>>> {
        self.requests.set(self.requests.get() + 1);
        ready(Ok(self.result.borrow_mut().take()))
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
        let future = self.save_file_picker_async(options);
        Box::pin(async move {
            Ok(SaveFilePickerResult {
                file: future.await?,
                selected_file_type: None,
            })
        })
    }

    fn can_pick_folder(&self) -> bool {
        false
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        self.fallback.open_folder_picker_async(options)
    }

    fn open_file_bookmark_async(
        &self,
        bookmark: &str,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        self.fallback.open_file_bookmark_async(bookmark)
    }

    fn open_folder_bookmark_async(
        &self,
        bookmark: &str,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        self.fallback.open_folder_bookmark_async(bookmark)
    }

    fn try_get_file_from_path_async(
        &self,
        path: &Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        self.fallback.try_get_file_from_path_async(path)
    }

    fn try_get_folder_from_path_async(
        &self,
        path: &Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        self.fallback.try_get_folder_from_path_async(path)
    }

    fn try_get_well_known_folder_async(
        &self,
        folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        self.fallback.try_get_well_known_folder_async(folder)
    }
}

struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    window: Ref<Window>,
    provider: Rc<SaveProvider>,
    file: Rc<MemoryFile>,
}

impl Fixture {
    fn new(cancel: bool) -> Self {
        let file = Rc::new(MemoryFile::default());
        let provider = Rc::new(SaveProvider {
            result: RefCell::new((!cancel).then(|| file.clone() as Rc<dyn IStorageFile>)),
            requests: Cell::new(0),
            fallback: NoopStorageProvider,
        });
        let platform = MockWindowingPlatform::create_window_mock();
        platform.setup_feature::<dyn IStorageProvider>(provider.clone());
        let workspace = DesignerWorkspace::with_uri(SOURCE, ORIGINAL_URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = Window::with_impl(platform);
        window.set_width(1400.0);
        window.set_height(950.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        Self {
            workspace,
            window,
            provider,
            file,
        }
    }

    fn edit_content(&self) {
        let session = self.workspace.session();
        let target = session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        session
            .borrow_mut()
            .set_property(&[target], "Content", "Edited before Save As")
            .unwrap();
        self.workspace.refresh(true);
        self.window.update_layout();
        assert!(self.workspace.is_dirty());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn canvas_save_as_preserves_active_camera_source_and_undo_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(false);
    fixture.edit_content();
    let workspace = &fixture.workspace;
    let view = CanvasViewport::new(2.375, Vector::new(-173.25, 93.5)).unwrap();
    workspace.set_canvas_viewport(view);
    let session = workspace.session();
    let source = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();

    assert!(complete(workspace.save_file(true)).unwrap());
    fixture.window.update_layout();

    assert_eq!(fixture.provider.requests.get(), 1);
    assert_eq!(fixture.file.writes.get(), 1);
    assert_eq!(fixture.file.bytes.borrow().as_slice(), source.as_bytes());
    assert_eq!(session.borrow().document_uri(), SAVED_URI);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(workspace.canvas_viewport(), view);
    assert!(!workspace.is_dirty());
    assert!(session.borrow().can_undo());
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert_eq!(workspace.canvas_viewport(), view);
}

#[test]
fn canvas_save_as_from_definition_restores_the_document_camera_after_layout() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(false);
    let workspace = &fixture.workspace;
    let document_view = CanvasViewport::new(1.75, Vector::new(-137.0, 68.5)).unwrap();
    workspace.set_canvas_viewport(document_view);
    let session = workspace.session();
    let template = session
        .borrow()
        .component_catalog()
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some("CardTemplate"))
        .unwrap()
        .node;
    session
        .borrow_mut()
        .enter_definition_scope(template)
        .unwrap();
    workspace.refresh(true);
    fixture.window.update_layout();
    let definition_view = CanvasViewport::new(3.0, Vector::new(51.0, -88.0)).unwrap();
    workspace.set_canvas_viewport(definition_view);
    assert!(session.borrow().definition_scope().is_some());

    assert!(complete(workspace.save_file(true)).unwrap());
    fixture.window.update_layout();

    // Save As replaces the URI-bound preview and returns to the document, preserving
    // its established behavior. The saved document must regain its own camera, not
    // the definition view or a center offset counted twice as the scope bar closes.
    assert_eq!(fixture.file.bytes.borrow().as_slice(), SOURCE.as_bytes());
    assert_eq!(session.borrow().document_uri(), SAVED_URI);
    assert!(session.borrow().definition_scope().is_none());
    assert_eq!(workspace.canvas_viewport(), document_view);
    assert!(!session.borrow().can_undo());
}

#[test]
fn canvas_save_as_cancel_keeps_camera_uri_dirty_source_and_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(true);
    fixture.edit_content();
    let workspace = &fixture.workspace;
    let view = CanvasViewport::new(0.625, Vector::new(93.0, -47.25)).unwrap();
    workspace.set_canvas_viewport(view);
    let session = workspace.session();
    let source = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();

    assert!(!complete(workspace.save_file(true)).unwrap());
    fixture.window.update_layout();

    assert_eq!(fixture.provider.requests.get(), 1);
    assert_eq!(fixture.file.writes.get(), 0);
    assert!(fixture.file.bytes.borrow().is_empty());
    assert_eq!(workspace.canvas_viewport(), view);
    assert_eq!(session.borrow().document_uri(), ORIGINAL_URI);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
    assert!(workspace.is_dirty());
    assert!(session.borrow().can_undo());
}
