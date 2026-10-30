//! Real TopLevel storage-provider calls, with controllable picker/read completion.
use super::*;
use crate::{PreviewEnvironment, PreviewStageConfiguration, PreviewStageId, PreviewStageStatus};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{
    FolderPickerOpenOptions, IStorageBookmarkFile, IStorageBookmarkFolder, IStorageFile,
    IStorageFolder, IStorageItem, IStorageProvider, NoopStorageProvider, OpenFilePickerResult,
    SaveFilePickerResult, StorageItemProperties, WellKnownFolder,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::Uri;
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::Window;
use ferroui_designer_core::Ast;
use std::future::{poll_fn, Future};
use std::io::{self, Cursor};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

const SOURCE_URI: &str = "memory://preview-stage-files/Current.xaml";
const MANIFEST_URI: &str = "memory://preview-stage-files/Workspace.ferroui-previews.json";
const MISSING_URI: &str = "file:///unopened-preview-stage-test/Missing.xaml";
const SOURCE: &str =
    r#"<Grid xmlns="https://github.com/ferroui"><Button Name="Target" Content="Original"/></Grid>"#;

fn ready<T: 'static>(value: T) -> LocalBoxFuture<T> {
    Box::pin(std::future::ready(value))
}

fn gated<T: 'static>(gate: Rc<Cell<bool>>, value: T) -> LocalBoxFuture<T> {
    let mut value = Some(value);
    Box::pin(poll_fn(move |_| {
        if gate.get() {
            Poll::Ready(
                value
                    .take()
                    .expect("A completed fixture future is not repolled"),
            )
        } else {
            Poll::Pending
        }
    }))
}

fn poll<T>(future: Pin<&mut impl Future<Output = T>>) -> Poll<T> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

fn complete<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match poll(future.as_mut()) {
        Poll::Ready(value) => value,
        Poll::Pending => {
            panic!("Release the fixture picker/read gate before completing the operation")
        }
    }
}

struct MemoryFile {
    bytes: Rc<RefCell<Vec<u8>>>,
    read_gate: Rc<Cell<bool>>,
    reads: Cell<usize>,
    writes: Cell<usize>,
}

impl Default for MemoryFile {
    fn default() -> Self {
        Self {
            bytes: Default::default(),
            read_gate: Rc::new(Cell::new(true)),
            reads: Cell::new(0),
            writes: Cell::new(0),
        }
    }
}

struct MemoryWriter(Rc<RefCell<Vec<u8>>>);

impl Write for MemoryWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
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
        "Workspace.ferroui-previews.json".into()
    }
    fn path(&self) -> Uri {
        Uri::absolute(MANIFEST_URI).unwrap()
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
            "No fixture folders",
        )))
    }
    fn as_storage_file(self: Rc<Self>) -> Option<Rc<dyn IStorageFile>> {
        Some(self)
    }
}

impl IStorageFile for MemoryFile {
    fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
        self.reads.set(self.reads.get() + 1);
        gated(
            self.read_gate.clone(),
            Ok(Box::new(Cursor::new(self.bytes.borrow().clone())) as Box<dyn Read>),
        )
    }
    fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
        self.writes.set(self.writes.get() + 1);
        self.bytes.borrow_mut().clear();
        ready(Ok(
            Box::new(MemoryWriter(self.bytes.clone())) as Box<dyn Write>
        ))
    }
}

struct ManifestProvider {
    file: Rc<MemoryFile>,
    open_gate: Rc<Cell<bool>>,
    save_gate: Rc<Cell<bool>>,
    open_requests: Cell<usize>,
    save_requests: Cell<usize>,
    path_requests: Cell<usize>,
    fallback: NoopStorageProvider,
}

impl IStorageProvider for ManifestProvider {
    fn can_open(&self) -> bool {
        true
    }
    fn open_file_picker_async(
        &self,
        _options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFile>>>> {
        self.open_requests.set(self.open_requests.get() + 1);
        gated(
            self.open_gate.clone(),
            Ok(vec![self.file.clone() as Rc<dyn IStorageFile>]),
        )
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
        self.save_requests.set(self.save_requests.get() + 1);
        gated(
            self.save_gate.clone(),
            Ok(Some(self.file.clone() as Rc<dyn IStorageFile>)),
        )
    }
    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
        self.fallback.save_file_picker_with_result_async(options)
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
        _path: &Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        self.path_requests.set(self.path_requests.get() + 1);
        ready(None)
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
    provider: Rc<ManifestProvider>,
    file: Rc<MemoryFile>,
}

impl Fixture {
    fn new() -> Self {
        let file = Rc::new(MemoryFile::default());
        let provider = Rc::new(ManifestProvider {
            file: file.clone(),
            open_gate: Rc::new(Cell::new(true)),
            save_gate: Rc::new(Cell::new(true)),
            open_requests: Cell::new(0),
            save_requests: Cell::new(0),
            path_requests: Cell::new(0),
            fallback: NoopStorageProvider,
        });
        let platform = MockWindowingPlatform::create_window_mock();
        platform.setup_feature::<dyn IStorageProvider>(provider.clone());
        let workspace = DesignerWorkspace::with_uri(SOURCE, SOURCE_URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = Window::with_impl(platform);
        window.set_width(1400.0);
        window.set_height(950.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        window.update_layout();
        workspace
            .add_preview_stage(configuration("Original stage", SOURCE_URI))
            .unwrap();
        Self {
            workspace,
            window,
            provider,
            file,
        }
    }

    fn set_manifest(&self, manifest: &PreviewStageManifest) {
        *self.file.bytes.borrow_mut() = manifest.to_json().unwrap().into_bytes();
    }

    fn ids(&self) -> Vec<PreviewStageId> {
        self.workspace
            .preview_stages()
            .iter()
            .map(|stage| stage.id)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn configuration(name: &str, uri: &str) -> PreviewStageConfiguration {
    let mut config = PreviewStageConfiguration::new(name, uri, PreviewEnvironment::default());
    config.pinned = true;
    config
}

struct Authored {
    uri: String,
    source: String,
    revision: u64,
    ast: Arc<Ast>,
    dirty: bool,
    document_dirty: bool,
    undo: Option<String>,
    redo: Option<String>,
}

impl Authored {
    fn capture(workspace: &DesignerWorkspace) -> Self {
        let session = workspace.session.borrow();
        let document = session.document();
        Self {
            uri: session.document_uri().into(),
            source: document.source().into(),
            revision: document.revision(),
            ast: document.ast_snapshot(),
            dirty: workspace.is_dirty(),
            document_dirty: document.is_dirty(),
            undo: document.undo_label().map(str::to_owned),
            redo: document.redo_label().map(str::to_owned),
        }
    }
    fn assert_unchanged(&self, workspace: &DesignerWorkspace) {
        let session = workspace.session.borrow();
        let document = session.document();
        assert_eq!(session.document_uri(), self.uri);
        assert_eq!(document.source(), self.source);
        assert_eq!(document.revision(), self.revision);
        assert!(Arc::ptr_eq(&document.ast_snapshot(), &self.ast));
        assert_eq!(workspace.is_dirty(), self.dirty);
        assert_eq!(document.is_dirty(), self.document_dirty);
        assert_eq!(document.undo_label(), self.undo.as_deref());
        assert_eq!(document.redo_label(), self.redo.as_deref());
    }
}

#[test]
fn save_picker_writes_its_captured_manifest_without_saving_or_editing_xaml() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    workspace
        .session
        .borrow_mut()
        .set_property(&[target], "Content", "Unsaved content")
        .unwrap();
    workspace.refresh(true);
    assert!(workspace.is_dirty());
    let authored = Authored::capture(workspace);
    let captured = workspace.preview_stage_manifest();
    fixture.provider.save_gate.set(false);
    let mut operation = Box::pin(workspace.save_preview_stages_file());
    assert!(poll(operation.as_mut()).is_pending());
    assert_eq!(fixture.provider.save_requests.get(), 1);
    assert_eq!(fixture.file.writes.get(), 0);

    workspace
        .add_preview_stage(configuration("Added during picker", SOURCE_URI))
        .unwrap();
    assert_ne!(workspace.preview_stage_manifest(), captured);
    fixture.provider.save_gate.set(true);
    assert!(complete(operation).unwrap());
    let bytes = fixture.file.bytes.borrow().clone();
    assert_eq!(bytes, captured.to_json().unwrap().as_bytes());
    assert_eq!(
        PreviewStageManifest::from_json(std::str::from_utf8(&bytes).unwrap()).unwrap(),
        captured
    );
    assert_eq!(fixture.file.writes.get(), 1);
    assert_eq!(fixture.ids().len(), 2);
    authored.assert_unchanged(workspace);
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    assert!(!workspace.session.borrow().can_undo());
}

#[cfg(feature = "render-smoke")]
#[test]
fn image_save_picker_keeps_captured_native_png_when_the_stage_changes_or_is_removed() {
    use crate::PreviewStageExportOptions;
    use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_harfbuzz::HarfBuzzTextShaper;
    use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};

    let _locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    let mut services = TestServices::styled_window();
    services.render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    services.font_manager_impl = FerroLocator::current().get_service::<dyn IFontManagerImpl>();
    services.text_shaper_impl = Some(Rc::new(HarfBuzzTextShaper::new()) as Rc<dyn ITextShaperImpl>);
    let _app = UnitTestApplication::start(services);
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    workspace.set_preview_stages_visible(true).unwrap();
    for _ in 0..3 {
        Dispatcher::ui_thread().run_jobs(None);
        fixture.window.update_layout();
    }
    let stage = fixture.ids()[0];
    let authored = Authored::capture(workspace);
    let captured = workspace
        .preview_stage_image(stage, PreviewStageExportOptions::default())
        .unwrap();
    assert_eq!(&captured.png[..8], b"\x89PNG\r\n\x1a\n");
    fixture.provider.save_gate.set(false);
    let mut operation = Box::pin(
        workspace.save_preview_stage_image_file(stage, PreviewStageExportOptions::default()),
    );
    assert!(poll(operation.as_mut()).is_pending());
    assert_eq!(fixture.provider.save_requests.get(), 1);
    assert_eq!(fixture.file.writes.get(), 0);
    let mut changed = workspace.preview_stage(stage).unwrap().configuration;
    changed.environment.width = 280.0;
    changed.environment.height = 180.0;
    workspace.update_preview_stage(stage, changed).unwrap();
    fixture.window.update_layout();
    let newer = workspace
        .preview_stage_image(stage, PreviewStageExportOptions::default())
        .unwrap();
    assert_ne!(captured.png, newer.png);
    workspace.remove_preview_stage(stage).unwrap();
    fixture.provider.save_gate.set(true);
    assert!(complete(operation).unwrap());
    assert_eq!(*fixture.file.bytes.borrow(), captured.png);
    assert_eq!(fixture.file.writes.get(), 1);
    assert!(workspace.preview_stage(stage).is_none());
    assert_eq!(fixture.provider.path_requests.get(), 0);
    authored.assert_unchanged(workspace);
}

#[test]
fn load_picker_restores_fresh_stage_ids_and_keeps_missing_source_as_a_reference() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let retired = fixture.ids();
    let authored = Authored::capture(workspace);
    let manifest = PreviewStageManifest {
        prototype: Default::default(),
        version: 1,
        stages: vec![
            configuration("Restored open document", SOURCE_URI),
            configuration("Unavailable view", MISSING_URI),
        ],
        active: Some(0),
    };
    fixture.set_manifest(&manifest);
    assert!(complete(workspace.load_preview_stages_file()).unwrap());
    assert_eq!(workspace.preview_stage_manifest(), manifest);
    assert!(fixture.ids().iter().all(|id| !retired.contains(id)));
    assert!(retired
        .iter()
        .all(|id| workspace.preview_stage(*id).is_none()));
    let missing = workspace
        .preview_stages()
        .into_iter()
        .find(|stage| stage.configuration.source_uri == MISSING_URI)
        .unwrap();
    assert_eq!(missing.status, PreviewStageStatus::MissingDocument);
    assert_eq!(missing.rendered_revision, None);
    assert_eq!(fixture.provider.path_requests.get(), 0);
    assert_eq!(fixture.provider.open_requests.get(), 1);
    assert_eq!(fixture.file.reads.get(), 1);
    assert_eq!(
        workspace.preview_stage_sources().len(),
        1,
        "Loading a set must not open its referenced URIs"
    );
    assert!(workspace.preview_stages_visible());
    authored.assert_unchanged(workspace);
}

#[test]
fn invalid_or_oversized_manifests_leave_accepted_stages_and_source_intact() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    let accepted = workspace.preview_stage_manifest();
    let ids = fixture.ids();
    let model = workspace.preview_stage_model();
    let authored = Authored::capture(workspace);
    for invalid in [
        "{broken".to_owned(),
        r#"{"version":4,"stages":[],"active":null}"#.to_owned(),
        " ".repeat(1_048_577),
    ] {
        *fixture.file.bytes.borrow_mut() = invalid.into_bytes();
        assert!(complete(workspace.load_preview_stages_file()).is_err());
        assert_eq!(workspace.preview_stage_manifest(), accepted);
        assert_eq!(fixture.ids(), ids);
        assert_eq!(workspace.preview_stage_model(), model);
        authored.assert_unchanged(workspace);
    }
    assert_eq!(fixture.provider.path_requests.get(), 0);
}

#[test]
fn delayed_picker_or_read_cannot_replace_newer_board_or_pending_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for delay_read in [false, true] {
        let fixture = Fixture::new();
        let workspace = &fixture.workspace;
        fixture.set_manifest(&PreviewStageManifest {
            prototype: Default::default(),
            version: 1,
            stages: vec![configuration("From delayed file", SOURCE_URI)],
            active: Some(0),
        });
        fixture.provider.open_gate.set(delay_read);
        fixture.file.read_gate.set(!delay_read);
        let mut operation = Box::pin(workspace.load_preview_stages_file());
        assert!(poll(operation.as_mut()).is_pending());
        assert_eq!(fixture.file.reads.get(), usize::from(delay_read));
        if delay_read {
            workspace.source.set_text(Some("<Grid"));
            assert!(workspace.pending_source.get());
        } else {
            workspace
                .add_preview_stage(configuration("Newer local stage", SOURCE_URI))
                .unwrap();
        }
        let newer_manifest = workspace.preview_stage_manifest();
        let newer_ids = fixture.ids();
        let newer_model = workspace.preview_stage_model();
        let authored = Authored::capture(workspace);
        fixture.provider.open_gate.set(true);
        fixture.file.read_gate.set(true);
        let error = complete(operation).unwrap_err();
        assert!(error.contains("changed while"), "{error}");
        assert_eq!(workspace.preview_stage_manifest(), newer_manifest);
        assert_eq!(fixture.ids(), newer_ids);
        assert_eq!(workspace.preview_stage_model(), newer_model);
        authored.assert_unchanged(workspace);
        if delay_read {
            assert!(workspace.pending_source.get());
            assert_eq!(workspace.source.text().as_deref(), Some("<Grid"));
        }
    }
}

#[test]
fn loading_an_empty_collection_keeps_the_board_empty() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    workspace.set_preview_stages_visible(true).unwrap();
    let authored = Authored::capture(workspace);
    let empty = PreviewStageManifest {
        prototype: Default::default(),
        version: 1,
        stages: Vec::new(),
        active: None,
    };
    fixture.set_manifest(&empty);
    assert!(complete(workspace.load_preview_stages_file()).unwrap());
    assert_eq!(workspace.preview_stage_manifest(), empty);
    assert!(workspace.preview_stages().is_empty());
    assert!(complete(workspace.load_preview_stages_file()).unwrap());
    assert!(
        workspace.preview_stages().is_empty(),
        "Showing a loaded set must not insert an implicit Current view"
    );
    authored.assert_unchanged(workspace);
}
