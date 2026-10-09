use crate::browser_app_builder::BrowserPlatformOptions;
use crate::browser_input_handler::{BrowserInputHandler, IInputTopLevel};
use crate::browser_insets_manager::BrowserInsetsManager;
use crate::browser_native_control_host::BrowserNativeControlHost;
use crate::clipboard_impl::ClipboardImpl;
use crate::cursor::CssCursor;
use crate::interop::{dom_helper, input_helper, JsObject};
use crate::js_object_control_handle::JsObjectControlHandle;
use crate::rendering::RenderTargetBrowserSurface;
use crate::storage::BrowserLauncher;
use crate::storage::BrowserStorageProvider;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::input::platform::{Clipboard, IClipboard};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::IInputRoot;
use ferroui_base::platform::storage::{ILauncher, IStorageProvider};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider, ISystemNavigationManagerImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect, Size};
use ferroui_controls::platform::{
    IInputPane, IInsetsManager, INativeControlHostImpl, IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl,
    PlatformThemeVariant,
};
use ferroui_controls::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};

thread_local! {
    static LAST_TOP_LEVEL_ID: Cell<i32> = const { Cell::new(0) };
    static TOP_LEVELS: RefCell<HashMap<i32, Weak<BrowserTopLevelImpl>>> = RefCell::new(HashMap::new());
    static GLOBAL_EVENTS_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

type Callback<T> = RefCell<Option<Rc<T>>>;

/// The top-level of a view: a canvas inside an element of the page.
pub struct BrowserTopLevelImpl {
    container: JsObject,
    native_control_host: Rc<dyn INativeControlHostImpl>,
    input_handler: Rc<BrowserInputHandler>,
    insets_manager: Rc<BrowserInsetsManager>,
    clipboard: Rc<Clipboard>,
    storage_provider: Rc<dyn IStorageProvider>,
    current_cursor: RefCell<String>,
    surface: RefCell<Option<Rc<RenderTargetBrowserSurface>>>,
    /// The surfaces as the thread that renders reads them; see `publish_render_surfaces`.
    render_surfaces: Arc<Mutex<Vec<Arc<dyn IPlatformRenderSurface>>>>,
    top_level_id: i32,
    compositor: Rc<Compositor>,
    handle: Rc<dyn IPlatformHandle>,
    acrylic_compensation_levels: AcrylicPlatformCompensationLevels,

    input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Callback<dyn Fn(Rect)>,
    resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    scaling_changed: Callback<dyn Fn(f64)>,
    transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
    closed: Callback<dyn Fn()>,
    lost_focus: Callback<dyn Fn()>,
}

impl BrowserTopLevelImpl {
    /// The top-level with the given id, while it is alive.
    pub fn try_get_top_level(id: i32) -> Option<Rc<BrowserTopLevelImpl>> {
        TOP_LEVELS.with(|top_levels| top_levels.borrow().get(&id).and_then(Weak::upgrade))
    }

    /// Creates the top-level of `container`.
    ///
    /// `native_control_host` is the element native controls are placed in
    /// and `input_element` the hidden input element of the view.
    pub fn new(container: JsObject, native_control_host: JsObject, input_element: JsObject) -> Rc<Self> {
        // Once, before the first top-level: the events and the handlers of the page that are not
        // bound to a view.
        if !GLOBAL_EVENTS_INITIALIZED.with(|initialized| initialized.replace(true)) {
            dom_helper::init_global_dom_events(&BrowserWindowingPlatform::global_this());
            input_helper::initialize_background_handlers(&BrowserWindowingPlatform::global_this());
        }

        let top_level_id = LAST_TOP_LEVEL_ID.with(|last| {
            last.set(last.get() + 1);
            last.get()
        });

        let this = Rc::new_cyclic(|this: &Weak<Self>| {
            TOP_LEVELS.with(|top_levels| top_levels.borrow_mut().insert(top_level_id, this.clone()));

            // The input handler reaches its top-level through a weak reference: the top-level
            // owns the handler.
            let input_top_level: Weak<dyn IInputTopLevel> = this.clone();
            let input_handler =
                BrowserInputHandler::new(input_top_level, container.clone(), input_element, top_level_id);

            let opts = FerroLocator::current().get_service::<BrowserPlatformOptions>().unwrap_or_default();
            let surface = RenderTargetBrowserSurface::create(
                &container,
                &opts.renderer.rendering_modes(&opts.rendering_mode),
                top_level_id,
            );
            let compositor = surface.compositor();

            Self {
                handle: Rc::new(JsObjectControlHandle::new(container.clone())),
                container,
                native_control_host: BrowserNativeControlHost::new(native_control_host),
                input_handler,
                insets_manager: Rc::new(BrowserInsetsManager::new()),
                clipboard: Clipboard::new(Rc::new(ClipboardImpl::new())),
                storage_provider: Rc::new(BrowserStorageProvider::new()),
                current_cursor: RefCell::new(CssCursor::DEFAULT.to_string()),
                surface: RefCell::new(Some(surface)),
                render_surfaces: Arc::new(Mutex::new(Vec::new())),
                top_level_id,
                compositor,
                acrylic_compensation_levels: AcrylicPlatformCompensationLevels::new(1.0, 1.0, 1.0),
                input: RefCell::new(None),
                paint: RefCell::new(None),
                resized: RefCell::new(None),
                scaling_changed: RefCell::new(None),
                transparency_level_changed: RefCell::new(None),
                closed: RefCell::new(None),
                lost_focus: RefCell::new(None),
            }
        });

        if let Some(surface) = this.surface() {
            surface.size_changed({
                let this = Rc::downgrade(&this);
                Rc::new(move || {
                    if let Some(this) = this.upgrade() {
                        this.on_size_changed();
                    }
                })
            });
            surface.scaling_changed({
                let this = Rc::downgrade(&this);
                Rc::new(move || {
                    if let Some(this) = this.upgrade() {
                        this.on_scaling_changed();
                    }
                })
            });
        }

        this
    }

    fn on_scaling_changed(&self) {
        if let Some(surface) = self.surface() {
            let scaling_changed = self.scaling_changed.borrow().clone();
            if let Some(scaling_changed) = scaling_changed {
                scaling_changed(surface.scaling());
            }
        }
    }

    /// Writes the surfaces the top-level has now to the cell the renderer reads.
    ///
    /// The reference reads `Surfaces` of the top-level whenever the renderer creates its render
    /// target, and the list changes: it is empty once the top-level is disposed. (Until the
    /// render target of the canvas exists the list has the surface, which is not ready; the
    /// reference has an empty list then.) The renderer cannot reach the top-level here
    /// (its function may be called by another thread), so the top-level publishes the list when
    /// it can have changed: when the renderer asks for the function, when the canvas changes
    /// its size, and when the top-level is disposed.
    fn publish_render_surfaces(&self) {
        let surfaces = ITopLevelImpl::surfaces(self);
        *self.render_surfaces.lock().unwrap_or_else(PoisonError::into_inner) = surfaces;
    }

    fn on_size_changed(&self) {
        self.publish_render_surfaces();
        if let Some(surface) = self.surface() {
            let resized = self.resized.borrow().clone();
            if let Some(resized) = resized {
                resized(surface.client_size(), WindowResizeReason::User);
            }
            self.insets_manager.notify_safe_area_padding_changed();
        }
    }

    /// The surface the top-level renders to; `None` once disposed.
    pub fn surface(&self) -> Option<Rc<RenderTargetBrowserSurface>> {
        self.surface.borrow().clone()
    }

    /// The id the script side routes its callbacks by.
    pub fn top_level_id(&self) -> i32 {
        self.top_level_id
    }

    /// The input handler of the top-level: what turns the events of the
    /// page into raw input events.
    pub fn input_handler(&self) -> &Rc<BrowserInputHandler> {
        &self.input_handler
    }

    /// The input root the top-level delivers input to.
    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_handler.input_root()
    }

    /// The keyboard focus of the page left the element of the top-level.
    // Differs from the original, where the lost-focus notification is never raised.
    pub(crate) fn on_lost_focus(&self) {
        let lost_focus = self.lost_focus.borrow().clone();
        if let Some(lost_focus) = lost_focus {
            self.input_handler().text_input_method().while_focus_leaves_view(|| lost_focus());
        }
    }
}

impl IInputTopLevel for BrowserTopLevelImpl {
    fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        let input = self.input.borrow().clone();
        if let Some(input) = input {
            input(args);
        }
    }

    fn page_size(&self) -> Size {
        ITopLevelImpl::client_size(self)
    }
}

impl IDisposable for BrowserTopLevelImpl {
    fn dispose(&self) {
        let surface = self.surface.borrow_mut().take();
        if let Some(surface) = surface {
            surface.dispose();
        }
        // The surface is gone: the renderer is handed no surfaces from here on.
        self.publish_render_surfaces();

        // Differs from the original, which leaves the listeners of the element in place: a
        // disposed top-level no longer receives the input of the page.
        self.input_handler.dispose();
    }
}

impl IOptionalFeatureProvider for BrowserTopLevelImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IStorageProvider>() {
            return Some(Rc::new(self.storage_provider.clone()));
        }

        if feature_type == TypeId::of::<dyn ITextInputMethodImpl>() {
            let text_input_method: Rc<dyn ITextInputMethodImpl> = self.input_handler.text_input_method().clone();
            return Some(Rc::new(text_input_method));
        }

        if feature_type == TypeId::of::<dyn ISystemNavigationManagerImpl>() {
            let service = FerroLocator::current().get_service::<dyn ISystemNavigationManagerImpl>()?;
            return Some(Rc::new(service));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let service = FerroLocator::current().get_service::<dyn IScreenImpl>()?;
            return Some(Rc::new(service));
        }

        if feature_type == TypeId::of::<dyn INativeControlHostImpl>() {
            return Some(Rc::new(self.native_control_host.clone()));
        }

        if feature_type == TypeId::of::<dyn IInsetsManager>() {
            let insets_manager: Rc<dyn IInsetsManager> = self.insets_manager.clone();
            return Some(Rc::new(insets_manager));
        }

        if feature_type == TypeId::of::<dyn IClipboard>() {
            let clipboard: Rc<dyn IClipboard> = self.clipboard.clone();
            return Some(Rc::new(clipboard));
        }

        if feature_type == TypeId::of::<dyn IInputPane>() {
            let input_pane: Rc<dyn IInputPane> = self.input_handler.input_pane().clone();
            return Some(Rc::new(input_pane));
        }

        if feature_type == TypeId::of::<dyn ILauncher>() {
            let launcher: Rc<dyn ILauncher> = Rc::new(BrowserLauncher::new());
            return Some(Rc::new(launcher));
        }

        None
    }
}

impl ITopLevelImpl for BrowserTopLevelImpl {
    fn desktop_scaling(&self) -> f64 {
        self.render_scaling()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        Some(self.handle.clone())
    }

    fn client_size(&self) -> Size {
        self.surface().map_or(Size::new(1.0, 1.0), |surface| surface.client_size())
    }

    fn render_scaling(&self) -> f64 {
        self.surface().map_or(1.0, |surface| surface.scaling())
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.surface().map_or_else(Vec::new, |surface| surface.get_render_surfaces())
    }

    fn render_surfaces(&self) -> Arc<dyn Fn() -> Vec<Arc<dyn IPlatformRenderSurface>> + Send + Sync> {
        self.publish_render_surfaces();
        let surfaces = self.render_surfaces.clone();
        Arc::new(move || surfaces.lock().unwrap_or_else(PoisonError::into_inner).clone())
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.compositor.clone())
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        self.input.borrow().clone()
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        *self.input.borrow_mut() = value;
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        self.paint.borrow().clone()
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        *self.paint.borrow_mut() = value;
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        self.resized.borrow().clone()
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        *self.resized.borrow_mut() = value;
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        self.scaling_changed.borrow().clone()
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.scaling_changed.borrow_mut() = value;
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.transparency_level_changed.borrow().clone()
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        *self.transparency_level_changed.borrow_mut() = value;
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        self.input_handler.set_input_root(input_root);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        Point::new(point.x as f64, point.y as f64)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::new(point.x as i32, point.y as i32)
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        let val = cursor
            .as_ref()
            .and_then(|cursor| cursor.as_any().downcast_ref::<CssCursor>())
            .and_then(|cursor| cursor.value().map(str::to_string))
            .unwrap_or_else(|| CssCursor::DEFAULT.to_string());
        if *self.current_cursor.borrow() != val {
            input_helper::set_cursor(&self.container, &val);
            *self.current_cursor.borrow_mut() = val;
        }
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.closed.borrow().clone()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        self.lost_focus.borrow().clone()
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        *self.lost_focus.borrow_mut() = value;
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        None
    }

    fn set_transparency_level_hint(&self, _transparency_levels: &[WindowTransparencyLevel]) {}

    fn transparency_level(&self) -> WindowTransparencyLevel {
        WindowTransparencyLevel::none()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        self.acrylic_compensation_levels
    }

    fn set_frame_theme_variant(&self, _theme_variant: Option<PlatformThemeVariant>) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
