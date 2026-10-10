//! Tests of the storage, launcher and leftover ports. Upstream has no tests
//! for these members (the helper tests of upstream are ported next to the
//! helpers in the base crate), so none of these is a port.

use crate::application_lifetimes::{ActivatedEventArgs, ActivationKind, FileActivatedEventArgs};
use crate::border_visual::ServerBorderVisual;
use crate::diagnostics::ToolTipDiagnostics;
use crate::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
    IPlatformHandle, IStorageProviderFactory, IX11OptionsToplevelImplFeature, X11NetWmWindowType, X11Properties,
};
use crate::primitives::HeaderedContentControl;
use crate::testing::{CompositorTestServices, MockWindowingPlatform, TestServices, UnitTestApplication};
use crate::{
    as_content_control, as_headered, Border, Button, ContentControl, Control, HyperlinkButton, NativeControlHost,
    TopLevel, UrlOpenedEventArgs, Window,
};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{
    FilePickerOpenOptions, ILauncher, IStorageItem, IStorageProvider, NoopStorageProvider,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, CornerRadius, FerroLocator, Rect, Ref, Size};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn ready<T>(mut future: LocalBoxFuture<T>) -> T {
    let mut cx = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future is expected to be complete"),
    }
}

// --- TopLevel.StorageProvider / TopLevel.Launcher ---

/// A storage provider that only differs from the one without pickers by
/// saying that it can open.
struct TestStorageProvider(NoopStorageProvider);

impl TestStorageProvider {
    fn new() -> Rc<dyn IStorageProvider> {
        Rc::new(Self(NoopStorageProvider))
    }
}

impl IStorageProvider for TestStorageProvider {
    fn can_open(&self) -> bool {
        true
    }

    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn ferroui_base::platform::storage::IStorageFile>>>> {
        self.0.open_file_picker_async(options)
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<ferroui_base::platform::storage::OpenFilePickerResult>> {
        self.0.open_file_picker_with_result_async(options)
    }

    fn can_save(&self) -> bool {
        false
    }

    fn save_file_picker_async(
        &self,
        options: ferroui_base::platform::storage::FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn ferroui_base::platform::storage::IStorageFile>>>> {
        self.0.save_file_picker_async(options)
    }

    fn save_file_picker_with_result_async(
        &self,
        options: ferroui_base::platform::storage::FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<ferroui_base::platform::storage::SaveFilePickerResult>> {
        self.0.save_file_picker_with_result_async(options)
    }

    fn can_pick_folder(&self) -> bool {
        false
    }

    fn open_folder_picker_async(
        &self,
        options: ferroui_base::platform::storage::FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn ferroui_base::platform::storage::IStorageFolder>>>> {
        self.0.open_folder_picker_async(options)
    }

    fn open_file_bookmark_async(
        &self,
        bookmark: &str,
    ) -> LocalBoxFuture<Option<Rc<dyn ferroui_base::platform::storage::IStorageBookmarkFile>>> {
        self.0.open_file_bookmark_async(bookmark)
    }

    fn open_folder_bookmark_async(
        &self,
        bookmark: &str,
    ) -> LocalBoxFuture<Option<Rc<dyn ferroui_base::platform::storage::IStorageBookmarkFolder>>> {
        self.0.open_folder_bookmark_async(bookmark)
    }

    fn try_get_file_from_path_async(
        &self,
        file_path: &Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn ferroui_base::platform::storage::IStorageFile>>> {
        self.0.try_get_file_from_path_async(file_path)
    }

    fn try_get_folder_from_path_async(
        &self,
        folder_path: &Uri,
    ) -> LocalBoxFuture<Option<Rc<dyn ferroui_base::platform::storage::IStorageFolder>>> {
        self.0.try_get_folder_from_path_async(folder_path)
    }

    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: ferroui_base::platform::storage::WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn ferroui_base::platform::storage::IStorageFolder>>> {
        self.0.try_get_well_known_folder_async(well_known_folder)
    }
}

struct TestStorageProviderFactory {
    provider: Rc<dyn IStorageProvider>,
    top_levels: RefCell<Vec<Ref<TopLevel>>>,
}

impl IStorageProviderFactory for TestStorageProviderFactory {
    fn create_provider(&self, top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
        self.top_levels.borrow_mut().push(top_level.clone());
        self.provider.clone()
    }
}

#[test]
fn storage_provider_without_platform_support_has_no_pickers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Window::with_impl(MockWindowingPlatform::create_window_mock());

    let storage_provider = target.storage_provider();

    assert!(!storage_provider.can_open());
    assert!(!storage_provider.can_save());
    assert!(!storage_provider.can_pick_folder());
    assert!(ready(storage_provider.open_file_picker_async(FilePickerOpenOptions::new())).unwrap().is_empty());
    // The provider is created once.
    assert!(Rc::ptr_eq(&storage_provider, &target.storage_provider()));
}

#[test]
fn storage_provider_comes_from_the_platform_implementation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let provider = TestStorageProvider::new();
    window_impl.setup_feature::<dyn IStorageProvider>(provider.clone());
    let target = Window::with_impl(window_impl.clone());

    assert!(Rc::ptr_eq(&provider, &target.storage_provider()));

    // The provider is kept, whatever the platform says later.
    window_impl.remove_feature::<dyn IStorageProvider>();
    assert!(Rc::ptr_eq(&provider, &target.storage_provider()));
}

#[test]
fn storage_provider_factory_takes_precedence_over_the_platform_implementation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let factory = Rc::new(TestStorageProviderFactory {
        provider: TestStorageProvider::new(),
        top_levels: RefCell::new(Vec::new()),
    });
    FerroLocator::current_mutable()
        .bind::<dyn IStorageProviderFactory>()
        .to_constant(factory.clone() as Rc<dyn IStorageProviderFactory>);
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.setup_feature::<dyn IStorageProvider>(TestStorageProvider::new());
    let target = Window::with_impl(window_impl);

    assert!(Rc::ptr_eq(&factory.provider, &target.storage_provider()));
    assert!(Rc::ptr_eq(&factory.provider, &target.storage_provider()));
    let top_levels = factory.top_levels.borrow();
    assert_eq!(1, top_levels.len());
    assert!(top_levels[0] == target.clone().upcast::<TopLevel>());
}

#[derive(Default)]
struct TestLauncher {
    succeeds: bool,
    uris: RefCell<Vec<Uri>>,
}

impl ILauncher for TestLauncher {
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool> {
        self.uris.borrow_mut().push(uri.clone());
        Box::pin(std::future::ready(self.succeeds))
    }

    fn launch_file_async(&self, _storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(self.succeeds))
    }
}

#[test]
fn launcher_without_platform_support_launches_nothing() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Window::with_impl(MockWindowingPlatform::create_window_mock());

    let launched = ready(target.launcher().launch_uri_async(&Uri::absolute("https://example.org/").unwrap()));

    assert!(!launched);
}

#[test]
fn launcher_comes_from_the_platform_implementation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let launcher = Rc::new(TestLauncher { succeeds: true, ..TestLauncher::default() });
    window_impl.setup_feature::<dyn ILauncher>(launcher.clone());
    let target = Window::with_impl(window_impl);

    let uri = Uri::absolute("https://example.org/").unwrap();
    assert!(ready(target.launcher().launch_uri_async(&uri)));
    assert_eq!(vec![uri], *launcher.uris.borrow());
}

// --- HyperlinkButton.OnClick ---

fn hyperlink_in_window(launcher: &Rc<TestLauncher>) -> (Ref<Window>, Ref<HyperlinkButton>) {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.setup_feature::<dyn ILauncher>(launcher.clone());
    let window = Window::with_impl(window_impl);
    let target = HyperlinkButton::new();
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    (window, target)
}

#[test]
fn hyperlink_button_click_launches_the_navigate_uri_and_becomes_visited() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let launcher = Rc::new(TestLauncher { succeeds: true, ..TestLauncher::default() });
    let (window, target) = hyperlink_in_window(&launcher);
    let uri = Uri::absolute("https://example.org/page").unwrap();
    target.set_navigate_uri(Some(uri.clone()));
    let clicks = Rc::new(Cell::new(0));
    target.click({
        let clicks = clicks.clone();
        move |_, _| clicks.set(clicks.get() + 1)
    });

    target.perform_click();

    // The click is raised at once, the URI is launched from a dispatcher job.
    assert_eq!(1, clicks.get());
    assert!(launcher.uris.borrow().is_empty());
    assert!(!target.is_visited());

    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(vec![uri], *launcher.uris.borrow());
    assert!(target.is_visited());
    assert!(target.classes().contains(":visited"));

    window.close();
}

#[test]
fn hyperlink_button_stays_unvisited_when_the_uri_cannot_be_launched_or_is_not_set() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let launcher = Rc::new(TestLauncher::default());
    let (window, target) = hyperlink_in_window(&launcher);

    // No URI: nothing is launched.
    target.perform_click();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(launcher.uris.borrow().is_empty());
    assert!(!target.is_visited());

    // The launcher reports a failure.
    target.set_navigate_uri(Some(Uri::absolute("unknown:thing").unwrap()));
    target.perform_click();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(1, launcher.uris.borrow().len());
    assert!(!target.is_visited());

    window.close();
}

// --- X11Properties ---

#[derive(Default)]
struct TestX11Options {
    window_types: RefCell<Vec<X11NetWmWindowType>>,
    classes: RefCell<Vec<Option<String>>>,
}

impl IX11OptionsToplevelImplFeature for TestX11Options {
    fn set_net_wm_window_type(&self, type_: X11NetWmWindowType) {
        self.window_types.borrow_mut().push(type_);
    }

    fn set_wm_class(&self, class_name: Option<&str>) {
        self.classes.borrow_mut().push(class_name.map(str::to_owned));
    }
}

#[test]
fn x11_properties_are_forwarded_to_the_platform_feature() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let options = Rc::new(TestX11Options::default());
    window_impl.setup_feature::<dyn IX11OptionsToplevelImplFeature>(options.clone());
    let target = Window::with_impl(window_impl);

    assert_eq!(X11NetWmWindowType::Normal, X11Properties::get_net_wm_window_type(&target));
    assert_eq!(None, X11Properties::get_wm_class(&target));

    X11Properties::set_net_wm_window_type(&target, X11NetWmWindowType::Dialog);
    X11Properties::set_wm_class(&target, Some("editor".to_owned()));
    X11Properties::set_wm_class(&target, None);

    assert_eq!(X11NetWmWindowType::Dialog, X11Properties::get_net_wm_window_type(&target));
    assert_eq!(vec![X11NetWmWindowType::Dialog], *options.window_types.borrow());
    assert_eq!(vec![Some("editor".to_owned()), None], *options.classes.borrow());

    // Without the feature the values are only stored.
    let plain = Window::with_impl(MockWindowingPlatform::create_window_mock());
    X11Properties::set_wm_class(&plain, Some("plain".to_owned()));
    assert_eq!(Some("plain".to_owned()), X11Properties::get_wm_class(&plain));
    assert_eq!(2, options.classes.borrow().len());
}

// --- NativeControlHost ---

struct TestHandle {
    destroyed: Cell<bool>,
}

impl IPlatformHandle for TestHandle {
    fn handle(&self) -> isize {
        42
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("TEST")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(
        &self,
    ) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for TestHandle {
    fn destroy(&self) {
        self.destroyed.set(true);
    }
}

#[derive(Default)]
pub(crate) struct TestAttachment {
    attached_to: RefCell<Option<Rc<dyn INativeControlHostImpl>>>,
    pub(crate) shown: RefCell<Vec<Rect>>,
    pub(crate) hidden: RefCell<Vec<Size>>,
    disposed: Cell<bool>,
    /// Whether the attachment refuses to move to another host.
    incompatible: Cell<bool>,
}

impl IDisposable for TestAttachment {
    fn dispose(&self) {
        self.disposed.set(true);
        *self.attached_to.borrow_mut() = None;
    }
}

impl INativeControlHostControlTopLevelAttachment for TestAttachment {
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.attached_to.borrow().clone()
    }

    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
        *self.attached_to.borrow_mut() = value;
    }

    fn is_compatible_with(&self, _host: &dyn INativeControlHostImpl) -> bool {
        !self.incompatible.get()
    }

    fn hide_with_size(&self, size: Size) {
        self.hidden.borrow_mut().push(size);
    }

    fn show_in_bounds(&self, rect: Rect) {
        self.shown.borrow_mut().push(rect);
    }
}

pub(crate) struct TestNativeControlHostImpl {
    this: std::rc::Weak<TestNativeControlHostImpl>,
    handles: RefCell<Vec<Rc<TestHandle>>>,
    pub(crate) attachments: RefCell<Vec<Rc<TestAttachment>>>,
    /// Whether the host refuses the handles of existing native controls.
    incompatible_with_handles: Cell<bool>,
}

impl TestNativeControlHostImpl {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            handles: RefCell::new(Vec::new()),
            attachments: RefCell::new(Vec::new()),
            incompatible_with_handles: Cell::new(false),
        })
    }

    fn attach(&self) -> Rc<TestAttachment> {
        let attachment = Rc::new(TestAttachment::default());
        let this: Rc<dyn INativeControlHostImpl> = self.this.upgrade().unwrap();
        *attachment.attached_to.borrow_mut() = Some(this);
        self.attachments.borrow_mut().push(attachment.clone());
        attachment
    }
}

impl INativeControlHostImpl for TestNativeControlHostImpl {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_default_child(&self, _parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
        let handle = Rc::new(TestHandle { destroyed: Cell::new(false) });
        self.handles.borrow_mut().push(handle.clone());
        handle
    }

    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let parent: Rc<dyn IPlatformHandle> = Rc::new(TestHandle { destroyed: Cell::new(false) });
        create(parent);
        self.attach()
    }

    fn create_new_attachment(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        self.attach()
    }

    fn is_compatible_with(&self, _handle: &dyn IPlatformHandle) -> bool {
        !self.incompatible_with_handles.get()
    }
}

#[test]
fn native_control_host_creates_shows_and_destroys_the_native_control() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let host_impl = TestNativeControlHostImpl::new();
    window_impl.setup_feature::<dyn INativeControlHostImpl>(host_impl.clone());
    let window = Window::with_impl(window_impl);
    window.set_width(200.0);
    window.set_height(100.0);

    let target = NativeControlHost::new();
    target.set_width(50.0);
    target.set_height(20.0);
    let changes = Rc::new(Cell::new(0));
    let _subscription = target.native_control_handle_changed({
        let changes = changes.clone();
        move || changes.set(changes.get() + 1)
    });
    assert!(target.native_control_handle().is_none());
    assert!(!target.try_update_native_control_position());

    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    // The control was created by the host of the platform, once.
    assert_eq!(1, host_impl.handles.borrow().len());
    assert_eq!(1, host_impl.attachments.borrow().len());
    assert_eq!(1, changes.get());
    let handle = target.native_control_handle().expect("the native control was created");
    assert_eq!(42, handle.handle());
    let attachment = host_impl.attachments.borrow()[0].clone();
    let shown = attachment.shown.borrow().last().copied().expect("the native control was shown");
    assert_eq!(Size::new(50.0, 20.0), shown.size());

    // An invisible host hides the native control.
    target.set_is_visible(false);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!attachment.hidden.borrow().is_empty());

    // Detached: the control is detached at once and destroyed later, if
    // the host has not been attached again by then.
    window.set_content(None::<BoxedValue>);
    assert!(attachment.attached_to().is_none());
    assert!(!host_impl.handles.borrow()[0].destroyed.get());
    assert!(target.native_control_handle().is_some());

    Dispatcher::ui_thread().run_jobs(None);

    assert!(host_impl.handles.borrow()[0].destroyed.get());
    assert!(attachment.disposed.get());
    assert!(target.native_control_handle().is_none());
    assert_eq!(2, changes.get());

    window.close();
}

/// Shows a window with a native control host whose native control exists.
fn shown_native_control_host() -> (Ref<Window>, Rc<TestNativeControlHostImpl>, Ref<NativeControlHost>) {
    let window_impl = MockWindowingPlatform::create_window_mock();
    let host_impl = TestNativeControlHostImpl::new();
    window_impl.setup_feature::<dyn INativeControlHostImpl>(host_impl.clone());
    let window = Window::with_impl(window_impl);
    window.set_width(200.0);
    window.set_height(100.0);

    let target = NativeControlHost::new();
    target.set_width(50.0);
    target.set_height(20.0);
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(1, host_impl.handles.borrow().len());
    assert_eq!(1, host_impl.attachments.borrow().len());
    (window, host_impl, target)
}

#[test]
fn native_control_host_reattaches_the_existing_control_when_its_attachment_is_incompatible() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (window, host_impl, target) = shown_native_control_host();
    let handle = target.native_control_handle().expect("the native control was created");
    let first_attachment = host_impl.attachments.borrow()[0].clone();
    first_attachment.incompatible.set(true);

    // Reparented before the queued destruction runs: the attachment cannot
    // follow, so it is disposed and the control is attached anew.
    window.set_content(None::<BoxedValue>);
    assert!(first_attachment.attached_to().is_none());
    window.set_content(Some(Control::boxed(target.clone())));
    Dispatcher::ui_thread().run_jobs(None);

    assert!(first_attachment.disposed.get());
    assert_eq!(2, host_impl.attachments.borrow().len());
    let second_attachment = host_impl.attachments.borrow()[1].clone();
    assert!(!second_attachment.disposed.get());
    assert!(second_attachment.attached_to().is_some());
    // The control itself is kept.
    assert_eq!(1, host_impl.handles.borrow().len());
    assert!(!host_impl.handles.borrow()[0].destroyed.get());
    let current = target.native_control_handle().expect("the native control is kept");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&handle), Rc::as_ptr(&current)));
    assert!(!second_attachment.shown.borrow().is_empty());

    window.close();
}

#[test]
fn native_control_host_recreates_the_control_when_the_host_is_incompatible_with_it() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (window, host_impl, target) = shown_native_control_host();
    let changes = Rc::new(Cell::new(0));
    let _subscription = target.native_control_handle_changed({
        let changes = changes.clone();
        move || changes.set(changes.get() + 1)
    });
    let first_attachment = host_impl.attachments.borrow()[0].clone();
    first_attachment.incompatible.set(true);
    host_impl.incompatible_with_handles.set(true);

    window.set_content(None::<BoxedValue>);
    window.set_content(Some(Control::boxed(target.clone())));
    Dispatcher::ui_thread().run_jobs(None);

    // Neither the attachment nor the control can be used with the host: the
    // control is destroyed and both are created again.
    assert!(first_attachment.disposed.get());
    assert!(host_impl.handles.borrow()[0].destroyed.get());
    assert_eq!(2, host_impl.handles.borrow().len());
    assert!(!host_impl.handles.borrow()[1].destroyed.get());
    assert_eq!(2, host_impl.attachments.borrow().len());
    assert!(target.native_control_handle().is_some());
    // Destroyed, then created.
    assert_eq!(2, changes.get());

    window.close();
}

// --- BorderVisual ---

#[test]
fn border_composition_visual_sends_the_corner_radius_to_the_server() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    services.setup(&window_impl);
    let window = Window::with_impl(window_impl);
    let target = Border::new();
    target.set_corner_radius(CornerRadius::uniform(4.0));
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    services.run_jobs();

    let server_corner_radius = || {
        let visual = target.composition_visual().expect("the border is attached to a composited tree");
        let server = visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).unwrap();
        server.content_as::<ServerBorderVisual>().expect("the server visual is a border visual").corner_radius()
    };
    assert_eq!(CornerRadius::uniform(4.0), server_corner_radius());

    target.set_corner_radius(CornerRadius::new(1.0, 2.0, 3.0, 4.0));
    services.run_jobs();
    assert_eq!(CornerRadius::new(1.0, 2.0, 3.0, 4.0), server_corner_radius());

    target.set_corner_radius(CornerRadius::default());
    services.run_jobs();
    assert_eq!(CornerRadius::default(), server_corner_radius());

    // Other controls keep the plain draw list visual.
    let button = Button::new();
    window.set_content(Some(Control::boxed(button.clone())));
    services.run_jobs();
    let visual = button.composition_visual().expect("the button is attached to a composited tree");
    let server = visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).unwrap();
    assert!(server.content_as::<ServerBorderVisual>().is_none());

    window.close();
}

// --- IContentControl / IHeadered ---

#[test]
fn content_controls_and_headered_controls_are_viewed_through_their_interfaces() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let content_control = ContentControl::new();
    let view = as_content_control(&content_control).expect("a content control is an IContentControl");
    view.set_content(Some(Rc::new("content".to_owned()) as BoxedValue));
    assert!(content_control.content().is_some());
    assert!(view.content().is_some());
    assert!(view.content_template().is_none());
    assert_eq!(content_control.horizontal_content_alignment(), view.horizontal_content_alignment());
    assert_eq!(content_control.vertical_content_alignment(), view.vertical_content_alignment());
    assert!(as_headered(&content_control).is_none());

    // Derived classes are covered by the registration of their base.
    let headered = HeaderedContentControl::new();
    assert!(as_content_control(&headered).is_some());
    let view = as_headered(&headered).expect("a headered content control is an IHeadered");
    assert!(view.header().is_none());
    view.set_header(Some(Rc::new("header".to_owned()) as BoxedValue));
    assert!(headered.header().is_some());

    assert!(as_content_control(&Border::new()).is_none());
    assert!(as_headered(&Border::new()).is_none());
}

// --- event args, diagnostics ---

#[test]
fn file_activated_event_args_are_activated_event_args_of_the_file_kind() {
    let args = FileActivatedEventArgs::new(Vec::new());
    assert_eq!(ActivationKind::File, args.kind());
    assert!(args.files().is_empty());

    let activated = ActivatedEventArgs::from(args);
    assert_eq!(ActivationKind::File, activated.kind());
    assert!(activated.as_file_activated().is_some());
    assert!(activated.as_protocol_activated().is_none());
    assert!(ActivatedEventArgs::new(ActivationKind::Reopen).as_file_activated().is_none());

    let urls = UrlOpenedEventArgs::new(vec!["app://open".to_owned()]);
    assert_eq!(["app://open".to_owned()], urls.urls());
}

#[test]
fn tool_tip_diagnostics_exposes_the_tool_tip_property() {
    assert!(std::ptr::eq(ToolTipDiagnostics::tool_tip_property(), crate::ToolTip::tool_tip_property()));
}

// --- OffscreenTopLevel ---

struct TestOffscreenImpl {
    mouse_device: Rc<dyn ferroui_base::input::IMouseDevice>,
}

impl crate::embedding::offscreen::OffscreenTopLevelImplOverrides for TestOffscreenImpl {
    fn surfaces(&self) -> Vec<std::sync::Arc<dyn ferroui_base::platform::surfaces::IPlatformRenderSurface>> {
        Vec::new()
    }

    fn mouse_device(&self) -> Rc<dyn ferroui_base::input::IMouseDevice> {
        self.mouse_device.clone()
    }
}

#[test]
fn offscreen_top_level_follows_its_implementation() {
    use crate::embedding::offscreen::{OffscreenTopLevel, OffscreenTopLevelImplBase};
    use crate::embedding::EmbeddableControlRoot;
    use crate::platform::ITopLevelImpl;
    use ferroui_base::{PixelPoint, Point};

    let services = CompositorTestServices::start(TestServices::styled_window());
    // The implementation creates a compositor of its own, on the render
    // loop of the current services.
    let render_loop: std::sync::Arc<dyn ferroui_base::rendering::IRenderLoop> = services.render_loop().clone();
    FerroLocator::current_mutable()
        .bind::<std::sync::Arc<dyn ferroui_base::rendering::IRenderLoop>>()
        .to_constant(Rc::new(render_loop));
    let mouse_device: Rc<dyn ferroui_base::input::IMouseDevice> = ferroui_base::input::MouseDevice::new();
    let platform_impl = OffscreenTopLevelImplBase::new(Rc::new(TestOffscreenImpl { mouse_device }));
    assert_eq!(1.0, platform_impl.render_scaling());
    assert_eq!(1.0, platform_impl.desktop_scaling());
    assert_eq!(Point::new(3.0, 4.0), platform_impl.point_to_client(PixelPoint::new(3, 4)));
    assert_eq!(PixelPoint::new(3, 4), platform_impl.point_to_screen(Point::new(3.0, 4.0)));
    assert!(platform_impl.create_popup().is_none());
    assert!(platform_impl.handle().is_none());

    let target = OffscreenTopLevel::new(platform_impl.clone());

    // The constructor prepares the top-level.
    assert!(target.is_initialized());
    assert!(platform_impl.input_root().is_some());
    assert!(std::ptr::eq(target.style_key(), EmbeddableControlRoot::TYPE));
    assert!(Rc::ptr_eq(target.offscreen_impl(), &platform_impl));

    platform_impl.set_client_size(Size::new(120.0, 80.0));
    assert_eq!(Size::new(120.0, 80.0), target.client_size());
    platform_impl.set_render_scaling(2.0);
    assert_eq!(2.0, target.render_scaling());
    assert_eq!(2.0, platform_impl.desktop_scaling());

    assert!(!platform_impl.is_disposed());
    target.dispose();
    assert!(platform_impl.is_disposed());
    assert!(target.platform_impl().is_none());

    services.dispose();
}

// --- Border clip through the border visual ---

struct ClipScene {
    services: CompositorTestServices,
    log: ferroui_base::rendering::testing::DrawingLog,
    window: Ref<Window>,
    target: Ref<Border>,
    child: Ref<Border>,
}

/// A 100 x 100 border at the top left of a canvas, with a 150 x 150 child
/// that overflows it.
fn clip_scene(corner_radius: CornerRadius, clip_to_bounds: bool) -> ClipScene {
    use ferroui_base::media::Brushes;
    use ferroui_base::rendering::testing::{DrawingLog, MockPlatformRenderInterface};

    let log = DrawingLog::new();
    let services = CompositorTestServices::start(
        TestServices::styled_window().with_render_interface(MockPlatformRenderInterface::new(log.clone())),
    );
    let window_impl = MockWindowingPlatform::create_window_mock();
    services.setup(&window_impl);
    let window = Window::with_impl(window_impl);
    window.set_width(300.0);
    window.set_height(300.0);

    let child = Border::new();
    child.set_width(150.0);
    child.set_height(150.0);
    child.set_background(Some(Brushes::red()));

    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    // A border that draws something: its content is invalidated when the
    // corner radius changes.
    target.set_background(Some(Brushes::blue()));
    target.set_corner_radius(corner_radius);
    target.set_clip_to_bounds(clip_to_bounds);
    target.set_child(child.clone());

    let canvas = crate::Canvas::new();
    canvas.children().add(target.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(canvas)));
    window.show();
    services.run_jobs();

    ClipScene { services, log, window, target, child }
}

impl ClipScene {
    fn hits_child(&self, x: f64, y: f64) -> bool {
        let origin = self.target.translate_point(ferroui_base::Point::new(0.0, 0.0), &self.window).unwrap();
        let hit = self.window.get_visual_at(ferroui_base::Point::new(origin.x + x, origin.y + y));
        hit.is_some_and(|hit| hit == self.child.clone().upcast::<ferroui_base::Visual>())
    }
}

#[test]
fn border_with_corner_radius_clips_its_content_to_the_rounded_bounds() {
    let scene = clip_scene(CornerRadius::uniform(30.0), true);

    // The child is painted inside the rounded clip of the border.
    let entries = scene.log.entries();
    let clip = entries.iter().position(|e| e == "PushRoundedClip 0, 0, 100, 100").expect("the rounded clip is pushed");
    let child = entries.iter().position(|e| e.starts_with("DrawRectangle Red")).expect("the child is drawn");
    assert!(clip < child, "{entries:?}");
    assert_eq!(0, scene.log.count("PushClip 0, 0, 100, 100"));

    // Inside the bounds the child is hit.
    assert!(scene.hits_child(50.0, 50.0));
    // The part of the child outside the bounds of the border is clipped.
    assert!(!scene.hits_child(120.0, 50.0));
    assert!(!scene.hits_child(50.0, 120.0));
    // As upstream, hit testing clips to the bounds, not to the rounded
    // corners: only painting is rounded.
    assert!(scene.hits_child(2.0, 2.0));
    assert!(scene.hits_child(98.0, 98.0));

    // A changed radius is used by the next frame.
    scene.log.clear();
    scene.target.set_corner_radius(CornerRadius::default());
    scene.services.run_jobs();
    assert_eq!(1, scene.log.count("PushClip 0, 0, 100, 100"), "{:?}", scene.log.entries());
    assert_eq!(0, scene.log.count("PushRoundedClip"));

    scene.window.close();
    scene.services.dispose();
}

#[test]
fn border_without_corner_radius_clips_its_content_to_the_bounds() {
    let scene = clip_scene(CornerRadius::default(), true);

    assert_eq!(1, scene.log.count("PushClip 0, 0, 100, 100"), "{:?}", scene.log.entries());
    assert_eq!(0, scene.log.count("PushRoundedClip"));
    assert!(scene.hits_child(50.0, 50.0));
    assert!(scene.hits_child(2.0, 2.0));
    assert!(!scene.hits_child(120.0, 50.0));

    scene.window.close();
    scene.services.dispose();
}

#[test]
fn border_that_does_not_clip_to_bounds_does_not_clip_its_content() {
    let scene = clip_scene(CornerRadius::uniform(30.0), false);

    assert_eq!(0, scene.log.count("PushRoundedClip"), "{:?}", scene.log.entries());
    assert_eq!(0, scene.log.count("PushClip 0, 0, 100, 100"));
    assert_eq!(1, scene.log.count("DrawRectangle Red"));
    assert!(scene.hits_child(50.0, 50.0));
    assert!(scene.hits_child(2.0, 2.0));
    // The overflowing part of the child is hit too.
    assert!(scene.hits_child(120.0, 50.0));

    // Turning clipping on applies the rounded clip.
    scene.log.clear();
    scene.target.set_clip_to_bounds(true);
    scene.services.run_jobs();
    assert_eq!(1, scene.log.count("PushRoundedClip 0, 0, 100, 100"), "{:?}", scene.log.entries());
    assert!(!scene.hits_child(120.0, 50.0));

    scene.window.close();
    scene.services.dispose();
}

// --- InProcessDragSource ---

mod in_process_drag_source {
    use crate::mouse_test_helper::MouseTestHelper;
    use crate::platform::InProcessDragSource;
    use crate::testing::{MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
    use crate::{Border, Canvas, Control, Window};
    use ferroui_base::input::platform::IPlatformDragSource;
    use ferroui_base::input::raw::{
        IDragDropDevice, IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs,
        RawPointerEventType,
    };
    use ferroui_base::input::{
        DataFormat, DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropDevice, DragDropEffects, DragEventArgs,
        IDataTransfer, IDataTransferItem, IInputManager, InputElement, InputManager, Key, KeyDeviceType,
        KeyboardDevice, LocalBoxFuture, MouseDevice, PhysicalKey, PointerPressedEventArgs, RawInputModifiers,
    };
    use ferroui_base::media::Brushes;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{FerroLocator, Point, Ref};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    struct CountingDataTransfer {
        inner: Rc<DataTransfer>,
        disposed: Cell<u32>,
    }

    impl IDisposable for CountingDataTransfer {
        fn dispose(&self) {
            self.disposed.set(self.disposed.get() + 1);
        }
    }

    impl IDataTransfer for CountingDataTransfer {
        fn formats(&self) -> Rc<[DataFormat]> {
            self.inner.formats()
        }

        fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
            IDataTransfer::items(&*self.inner)
        }
    }

    struct Scene {
        _app: UnitTestApplicationScope,
        input_manager: Rc<dyn IInputManager>,
        mouse: Rc<MouseDevice>,
        window_impl: Rc<MockWindowImpl>,
        window: Ref<Window>,
        events: Rc<RefCell<Vec<String>>>,
        trigger: PointerPressedEventArgs,
        data: Rc<CountingDataTransfer>,
    }

    /// A window with two 100 x 100 drop targets side by side: "left" accepts
    /// a copy, "right" a copy or a move.
    fn scene() -> Scene {
        let input_manager: Rc<dyn IInputManager> = Rc::new(InputManager::new());
        let app = UnitTestApplication::start(TestServices::styled_window().with_input_manager(input_manager.clone()));
        FerroLocator::current_mutable()
            .bind::<dyn IDragDropDevice>()
            .to_constant(DragDropDevice::instance() as Rc<dyn IDragDropDevice>);

        let window_impl = MockWindowingPlatform::create_window_mock();
        let window = Window::with_impl(window_impl.clone());
        window.set_width(200.0);
        window.set_height(100.0);

        let events = Rc::new(RefCell::new(Vec::new()));
        let canvas = Canvas::new();
        for (name, left, effects) in [
            ("left", 0.0, DragDropEffects::COPY),
            ("right", 100.0, DragDropEffects::COPY | DragDropEffects::MOVE),
        ] {
            let target = Border::new();
            target.set_width(100.0);
            target.set_height(100.0);
            target.set_background(Some(Brushes::red()));
            Canvas::set_left(&target, left);
            DragDrop::set_allow_drop(&target, true);
            for (event, routed_event) in [
                ("enter", DragDrop::drag_enter_event()),
                ("over", DragDrop::drag_over_event()),
                ("leave", DragDrop::drag_leave_event()),
                ("drop", DragDrop::drop_event()),
            ] {
                let events = events.clone();
                target.add_handler(routed_event, move |_, e: &DragEventArgs| {
                    let text = e.data_transfer().try_get_text().unwrap_or_default();
                    events.borrow_mut().push(format!("{event} {name} {text}"));
                    e.set_drag_effects(e.drag_effects() & effects);
                });
            }
            canvas.children().add(target.upcast::<Control>());
        }
        window.set_content(Some(Control::boxed(canvas.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);

        // The pointer press that starts the drag.
        let trigger: Rc<RefCell<Option<PointerPressedEventArgs>>> = Rc::new(RefCell::new(None));
        canvas.add_handler(InputElement::pointer_pressed_event(), {
            let trigger = trigger.clone();
            move |_, e: &PointerPressedEventArgs| *trigger.borrow_mut() = Some(e.clone())
        });
        let helper = MouseTestHelper::new();
        helper.down(&canvas);
        let trigger = trigger.borrow_mut().take().expect("a pointer pressed event");

        let inner = DataTransfer::new();
        inner.add(DataTransferItem::create_text(Some("payload")));
        let data = Rc::new(CountingDataTransfer { inner, disposed: Cell::new(0) });

        Scene { _app: app, input_manager, mouse: MouseDevice::new(), window_impl, window, events, trigger, data }
    }

    impl Scene {
        fn start(&self, allowed_effects: DragDropEffects) -> LocalBoxFuture<DragDropEffects> {
            let source = InProcessDragSource::new();
            source.do_drag_drop_async(&self.trigger, self.data.clone(), allowed_effects)
        }

        /// Sends a raw pointer event; returns whether the drag source
        /// handled it.
        fn pointer(&self, type_: RawPointerEventType, x: f64, y: f64, modifiers: RawInputModifiers) -> bool {
            let args = Rc::new(RawPointerEventArgs::new(
                self.mouse.clone(),
                0,
                self.window.input_root(),
                type_,
                Point::new(x, y),
                modifiers,
            ));
            self.input_manager.process_input(args.clone() as Rc<dyn IRawInputEventArgs>);
            args.handled()
        }

        fn key(&self, key: Key, modifiers: RawInputModifiers) -> bool {
            let args = Rc::new(RawKeyEventArgs::new(
                KeyboardDevice::new(),
                0,
                self.window.input_root(),
                RawKeyEventType::KeyDown,
                key,
                modifiers,
                PhysicalKey::None,
                None,
                KeyDeviceType::Keyboard,
            ));
            self.input_manager.process_input(args.clone() as Rc<dyn IRawInputEventArgs>);
            args.handled()
        }

        fn take_events(&self) -> Vec<String> {
            std::mem::take(&mut *self.events.borrow_mut())
        }
    }

    fn poll(future: &mut LocalBoxFuture<DragDropEffects>) -> Option<DragDropEffects> {
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => Some(value),
            Poll::Pending => None,
        }
    }

    const LEFT: RawInputModifiers = RawInputModifiers::LEFT_MOUSE_BUTTON;

    #[test]
    fn drag_over_two_controls_raises_the_drag_events_and_drops() {
        let scene = scene();
        let mut drag = scene.start(DragDropEffects::COPY | DragDropEffects::MOVE);
        assert_eq!(None, poll(&mut drag));
        assert!(scene.take_events().is_empty());

        // Entering the window over the left target.
        assert!(scene.pointer(RawPointerEventType::Move, 50.0, 50.0, LEFT));
        assert_eq!(scene.take_events(), ["enter left payload"]);
        // The cursor of the accepted effect overrides the cursor of the window.
        assert!(scene.window_impl.cursor().is_some());

        assert!(scene.pointer(RawPointerEventType::Move, 60.0, 50.0, LEFT));
        assert_eq!(scene.take_events(), ["over left payload"]);

        // Moving on to the right target.
        assert!(scene.pointer(RawPointerEventType::Move, 150.0, 50.0, LEFT));
        assert_eq!(scene.take_events(), ["leave left payload", "enter right payload"]);
        assert_eq!(None, poll(&mut drag));
        assert_eq!(0, scene.data.disposed.get());

        // Releasing the button that started the drag drops the data.
        assert!(scene.pointer(RawPointerEventType::LeftButtonUp, 150.0, 50.0, RawInputModifiers::NONE));
        assert_eq!(scene.take_events(), ["drop right payload"]);
        // Both effects are accepted: without modifiers the data is moved.
        assert_eq!(Some(DragDropEffects::MOVE), poll(&mut drag));
        assert_eq!(1, scene.data.disposed.get());
        // The cursor override is gone.
        assert!(scene.window_impl.cursor().is_none());

        // The drag has ended: input is no longer intercepted.
        assert!(!scene.pointer(RawPointerEventType::Move, 60.0, 50.0, RawInputModifiers::NONE));
        assert!(scene.take_events().is_empty());

        scene.window.close();
    }

    #[test]
    fn the_modifier_keys_choose_between_the_accepted_effects() {
        let scene = scene();
        let mut drag = scene.start(DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK);

        assert!(scene.pointer(RawPointerEventType::Move, 150.0, 50.0, LEFT));
        assert_eq!(scene.take_events(), ["enter right payload"]);

        // A modifier key repeats the drag over with the new modifiers.
        assert!(!scene.key(Key::LeftCtrl, RawInputModifiers::CONTROL));
        assert_eq!(scene.take_events(), ["over right payload"]);

        // Control chooses the copy.
        assert!(scene.pointer(RawPointerEventType::LeftButtonUp, 150.0, 50.0, RawInputModifiers::CONTROL));
        assert_eq!(scene.take_events(), ["drop right payload"]);
        assert_eq!(Some(DragDropEffects::COPY), poll(&mut drag));

        scene.window.close();
    }

    #[test]
    fn escape_and_other_buttons_cancel_the_drag() {
        let scene = scene();
        let mut drag = scene.start(DragDropEffects::COPY);
        assert!(scene.pointer(RawPointerEventType::Move, 50.0, 50.0, LEFT));
        assert_eq!(scene.take_events(), ["enter left payload"]);

        assert!(scene.key(Key::Escape, RawInputModifiers::NONE));
        assert_eq!(scene.take_events(), ["leave left payload"]);
        assert_eq!(Some(DragDropEffects::NONE), poll(&mut drag));
        assert_eq!(1, scene.data.disposed.get());
        assert!(scene.window_impl.cursor().is_none());
        scene.window.close();

        // A button press during the drag cancels it too.
        let scene = self::scene();
        let mut drag = scene.start(DragDropEffects::COPY);
        assert!(scene.pointer(RawPointerEventType::Move, 50.0, 50.0, LEFT));
        assert!(scene.pointer(RawPointerEventType::RightButtonDown, 50.0, 50.0, LEFT | RawInputModifiers::RIGHT_MOUSE_BUTTON));
        assert_eq!(scene.take_events(), ["enter left payload", "leave left payload"]);
        assert_eq!(Some(DragDropEffects::NONE), poll(&mut drag));

        // So does the release of a button that did not start the drag.
        let mut drag = scene.start(DragDropEffects::COPY);
        assert!(scene.pointer(RawPointerEventType::Move, 50.0, 50.0, LEFT));
        assert!(scene.pointer(RawPointerEventType::RightButtonUp, 50.0, 50.0, LEFT));
        assert_eq!(scene.take_events(), ["enter left payload", "leave left payload"]);
        assert_eq!(Some(DragDropEffects::NONE), poll(&mut drag));

        scene.window.close();
    }

    #[test]
    fn a_target_that_does_not_accept_an_allowed_effect_gets_no_drop_effect() {
        let scene = scene();
        // Only a link is allowed, which no target accepts.
        let mut drag = scene.start(DragDropEffects::LINK);
        assert!(scene.pointer(RawPointerEventType::Move, 50.0, 50.0, LEFT));
        assert!(scene.pointer(RawPointerEventType::LeftButtonUp, 50.0, 50.0, RawInputModifiers::NONE));
        assert_eq!(Some(DragDropEffects::NONE), poll(&mut drag));

        // A drag source is used for one drag: a second one is refused.
        let source = InProcessDragSource::new();
        let mut first = source.do_drag_drop_async(&scene.trigger, scene.data.clone(), DragDropEffects::COPY);
        let mut second = source.do_drag_drop_async(&scene.trigger, scene.data.clone(), DragDropEffects::COPY);
        assert_eq!(None, poll(&mut first));
        assert_eq!(Some(DragDropEffects::NONE), poll(&mut second));
        assert!(scene.key(Key::Escape, RawInputModifiers::NONE));
        assert_eq!(Some(DragDropEffects::NONE), poll(&mut first));

        scene.window.close();
    }
}
